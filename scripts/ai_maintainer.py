#!/usr/bin/env python3
import argparse
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
TEXT_FILE_SUFFIXES = {".rs", ".md", ".toml", ".txt", ".json"}
SKIP_DIRS = {".git", ".github", "target", "data", "__pycache__"}
MODEL_BLOCKLIST = (
    "embedding",
    "image",
    "audio",
    "tts",
    "live",
    "aqa",
    "vision",
)


def log(message):
    print(f"[ai-maintainer] {message}")


def set_output(name, value):
    output_path = os.environ.get("GITHUB_OUTPUT")
    if not output_path:
        return
    with open(output_path, "a", encoding="utf-8") as handle:
        handle.write(f"{name}={value}\n")


def load_config(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def fetch_json(url, timeout_seconds, retries, payload=None):
    data = None
    headers = {}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"

    request = urllib.request.Request(url, data=data, headers=headers)
    last_error = None
    for attempt in range(1, retries + 1):
        try:
            with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
                return json.load(response)
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as error:
            last_error = error
            log(f"HTTP attempt {attempt}/{retries} failed: {error}")
            if attempt < retries:
                time.sleep(min(5 * attempt, 15))

    raise RuntimeError(f"request failed after {retries} attempts: {last_error}")


def list_available_models(api_key, config):
    base_url = "https://generativelanguage.googleapis.com/v1beta/models"
    timeout_seconds = config["model_list_timeout_seconds"]
    retries = config["http_retries"]
    page_token = None
    models = []
    for _ in range(10):
        query = {"key": api_key}
        if page_token:
            query["pageToken"] = page_token
        url = f"{base_url}?{urllib.parse.urlencode(query)}"
        payload = fetch_json(url, timeout_seconds=timeout_seconds, retries=retries)
        models.extend(payload.get("models", []))
        page_token = payload.get("nextPageToken")
        if not page_token:
            break
    return models


def supports_generate_content(model):
    return "generateContent" in model.get("supportedGenerationMethods", [])


def normalize_model_name(name):
    return name.removeprefix("models/")


def version_score(model_name):
    match = re.search(r"gemini-(\d+)(?:\.(\d+))?", model_name)
    if not match:
        return (0, 0)
    return (int(match.group(1)), int(match.group(2) or 0))


def heuristic_model_rank(model_name):
    major, minor = version_score(model_name)
    is_preview = 1 if ("preview" in model_name or "-exp" in model_name) else 0
    is_lite = 1 if "lite" in model_name else 0
    return (-major, -minor, is_preview, is_lite, model_name)


def discover_model_candidates(api_key, config):
    try:
        models = list_available_models(api_key, config)
    except Exception as error:  # noqa: BLE001
        log(f"model discovery failed, falling back to configured candidates: {error}")
        return list(dict.fromkeys(config["candidate_models"]))

    usable = []
    for model in models:
        if not supports_generate_content(model):
            continue
        model_name = normalize_model_name(model["name"])
        lower_name = model_name.lower()
        if not lower_name.startswith("gemini-"):
            continue
        if "flash" not in lower_name:
            continue
        if any(blocked in lower_name for blocked in MODEL_BLOCKLIST):
            continue
        usable.append(model_name)

    ranked = []
    for candidate in config["candidate_models"]:
        if candidate in usable:
            ranked.append(candidate)

    for model_name in sorted(usable, key=heuristic_model_rank):
        if model_name not in ranked:
            ranked.append(model_name)

    if ranked:
        return ranked

    return list(dict.fromkeys(config["candidate_models"]))


def build_repo_context():
    interesting_paths = []
    for path in sorted(REPO_ROOT.rglob("*")):
        if not path.is_file():
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.suffix.lower() not in TEXT_FILE_SUFFIXES:
            continue
        relative = path.relative_to(REPO_ROOT)
        interesting_paths.append(relative)

    lines = ["Repository tree:"]
    for relative in interesting_paths:
        lines.append(f"- {relative.as_posix()}")

    sample_files = [
        Path("README.md"),
        Path("agents.md"),
        Path("Cargo.toml"),
    ]
    sample_files.extend(sorted(Path("src").glob("*.rs")))

    for relative in sample_files:
        path = REPO_ROOT / relative
        if not path.exists() or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        lines.append("")
        lines.append(f"FILE: {relative.as_posix()}")
        lines.append("```")
        lines.append(text[:14000])
        lines.append("```")

    return "\n".join(lines)


def build_prompt(config):
    repo_context = build_repo_context()
    return f"""You are maintaining a local Git repository for a privacy-first Windows activity replay tool written in Rust.

Primary instruction from repository owner:
{config["prompt"]}

Hard constraints:
- Do not add spyware-like behavior, stealth recording, telemetry, networking, or raw keylogging.
- Prefer one small, safe, additive improvement that preserves the privacy-first positioning.
- Prefer changes in src/ and tests/. README.md and Cargo.toml are allowed if necessary.
- Do not modify GitHub workflows, automation scripts, licenses, or generated data.
- Keep the patch small: at most {config["max_changed_files"]} files.
- Add or adjust tests when practical.
- If there is no clearly safe and worthwhile improvement, respond with exactly NO_CHANGE.

Return format:
- Return ONLY a unified git diff rooted at the repository root.
- The diff must start with diff --git headers.
- Do not include explanations, markdown fences, or commentary.

Current repository context:
{repo_context}
"""


def generate_patch(api_key, model_name, prompt, config):
    url = (
        f"https://generativelanguage.googleapis.com/v1beta/models/"
        f"{model_name}:generateContent?key={urllib.parse.quote(api_key)}"
    )
    payload = {
        "contents": [
            {
                "role": "user",
                "parts": [{"text": prompt}],
            }
        ],
        "generationConfig": {
            "temperature": 0.2,
            "topP": 0.8,
            "maxOutputTokens": 8192,
        },
    }

    response = fetch_json(
        url,
        timeout_seconds=config["generation_timeout_seconds"],
        retries=config["http_retries"],
        payload=payload,
    )
    candidates = response.get("candidates", [])
    if not candidates:
        raise RuntimeError("model returned no candidates")

    parts = candidates[0].get("content", {}).get("parts", [])
    text_parts = [part.get("text", "") for part in parts if "text" in part]
    if not text_parts:
        raise RuntimeError("model returned no text parts")
    return "".join(text_parts).strip()


def extract_diff(text):
    if text == "NO_CHANGE":
        return None

    fenced = re.search(r"```(?:diff)?\n(.*)\n```", text, re.DOTALL)
    if fenced:
        text = fenced.group(1).strip()

    if "diff --git " in text:
        return text[text.index("diff --git "):].strip() + "\n"

    return text.strip() + "\n"


def parse_patch_paths(diff_text):
    paths = []
    for line in diff_text.splitlines():
        if line.startswith("diff --git "):
            match = re.match(r"diff --git a/(.+?) b/(.+)$", line)
            if not match:
                raise RuntimeError(f"unparseable diff header: {line}")
            old_path, new_path = match.group(1), match.group(2)
            paths.append(new_path if new_path != "/dev/null" else old_path)
    return paths


def validate_patch(diff_text, config):
    if len(diff_text.encode("utf-8")) > config["max_patch_bytes"]:
        raise RuntimeError("patch too large")

    changed_paths = parse_patch_paths(diff_text)
    unique_paths = sorted(set(changed_paths))
    if not unique_paths:
        raise RuntimeError("patch has no changed files")
    if len(unique_paths) > config["max_changed_files"]:
        raise RuntimeError("patch touches too many files")

    blocked_prefixes = tuple(config["blocked_path_prefixes"])
    allowed_prefixes = tuple(config["allowed_path_prefixes"])
    allowed_exact = set(config["allowed_exact_paths"])

    for path in unique_paths:
        normalized = path.replace("\\", "/")
        if normalized.startswith(blocked_prefixes):
            raise RuntimeError(f"patch touches blocked path: {normalized}")
        if normalized in allowed_exact:
            continue
        if normalized.startswith(allowed_prefixes):
            continue
        raise RuntimeError(f"patch touches disallowed path: {normalized}")

    if "Binary files " in diff_text or "GIT binary patch" in diff_text:
        raise RuntimeError("binary patches are blocked")
    if re.search(r"^deleted file mode ", diff_text, re.MULTILINE):
        raise RuntimeError("file deletions are blocked")
    if re.search(r"^rename (from|to) ", diff_text, re.MULTILINE):
        raise RuntimeError("file renames are blocked")

    added = 0
    removed = 0
    for line in diff_text.splitlines():
        if line.startswith("+++ ") or line.startswith("--- "):
            continue
        if line.startswith("+"):
            added += 1
        elif line.startswith("-"):
            removed += 1

    if added > config["max_added_lines"]:
        raise RuntimeError("patch adds too many lines")
    if removed > config["max_removed_lines"]:
        raise RuntimeError("patch removes too many lines")

    return unique_paths


def write_patch_file(diff_text):
    patch_dir = REPO_ROOT / ".autonomy"
    patch_dir.mkdir(exist_ok=True)
    patch_path = patch_dir / "latest.patch"
    patch_path.write_text(diff_text, encoding="utf-8")
    return patch_path


def run(command, *, check=True):
    log(f"running: {' '.join(command)}")
    completed = subprocess.run(
        command,
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
    )
    if completed.stdout:
        print(completed.stdout, end="")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    if check and completed.returncode != 0:
        raise RuntimeError(f"command failed ({completed.returncode}): {' '.join(command)}")
    return completed


def apply_patch(patch_path):
    run(["git", "apply", "--check", str(patch_path)])
    run(["git", "apply", str(patch_path)])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--config",
        default=str(REPO_ROOT / "automation" / "autonomy.json"),
    )
    args = parser.parse_args()

    config = load_config(args.config)
    set_output("patch_applied", "false")
    set_output("selected_model", "")
    set_output("skip_reason", "")

    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        log("GEMINI_API_KEY is missing; skipping")
        set_output("skip_reason", "missing_api_key")
        return 0

    candidate_models = discover_model_candidates(api_key, config)
    if not candidate_models:
        log("no suitable Gemini flash model candidates are available; skipping")
        set_output("skip_reason", "no_model_available")
        return 0

    set_output("selected_model", candidate_models[0])
    log(f"candidate models: {', '.join(candidate_models)}")
    prompt = build_prompt(config)

    attempts = []
    for model_name in candidate_models:
        for attempt in range(1, config["per_model_retries"] + 1):
            try:
                set_output("selected_model", model_name)
                response_text = generate_patch(api_key, model_name, prompt, config)
                diff_text = extract_diff(response_text)
                if diff_text is None:
                    log("model returned NO_CHANGE; skipping")
                    set_output("skip_reason", "no_change")
                    return 0

                changed_paths = validate_patch(diff_text, config)
                patch_path = write_patch_file(diff_text)
                apply_patch(patch_path)
                log(f"patch applied safely to: {', '.join(changed_paths)}")
                set_output("patch_applied", "true")
                return 0
            except Exception as error:  # noqa: BLE001
                attempts.append(f"{model_name} attempt {attempt}: {error}")
                log(f"generation attempt failed: {error}")

    log("all model attempts failed; skipping without repository changes")
    for item in attempts:
        log(item)
    set_output("skip_reason", "generation_failed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
