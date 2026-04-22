# FastForward-Timemachine

FastForward-Timemachine is a privacy-first, local-only activity replay engine for Windows.

It is designed to help users review their work timeline with explicit consent, visible recording state, and strict local storage boundaries. The project is intentionally framed as a productivity tool, not surveillance software.

## Principles

- Recording must always be visible and user-controlled.
- Data stays local. No telemetry, sync, or remote transfer.
- Raw keyboard text is excluded by default.
- Privacy filters are a first-class feature, not an afterthought.
- Modules should remain independently testable.

## Planned Modules

- `capture/`: screen capture and visual delta detection
- `input/`: safe keyboard and mouse metadata capture
- `process/`: process lifecycle and focus tracking
- `storage/`: timeline segments, compression, and indexing
- `encryption/`: optional local-only encryption

## Profiles

- `minimal`: screen capture only
- `focus`: selected apps only
- `privacy`: aggressive exclusion rules
- `full-replay`: advanced mode with explicit warnings

## Current State

The repository currently contains an initial Rust-oriented project skeleton, a recorder state machine, timeline event scaffolding, and a CLI status surface. Low-level capture and replay backends are not implemented yet.

## Autonomous Evolution

This repository includes a conservative, fail-closed autonomous setup:

- `.github/workflows/autonomous-propose.yml`
- `.github/workflows/autonomous-verify.yml`
- `.github/workflows/autonomous-merge.yml`

The flow is intentionally indirect:

- Daily schedule runs, then a `1 in 10` random gate decides whether to attempt a proposal.
- The bot refuses to run if an older autonomous PR is already open.
- Gemini model selection is dynamic and falls back across multiple Flash-family candidates.
- The AI may only produce a very small patch on approved paths.
- The proposal must pass `cargo fmt`, `cargo check`, and `cargo test` before a PR is even opened.
- The merge workflow waits for a minimum PR age before trying to merge.
- The merge workflow only merges an open labeled autonomous PR when GitHub checks are green and the PR is mergeable.
- There is no direct push to `main` from the generation workflow.

### Required Secret

Add the following repository secret before enabling the workflow:

- `GEMINI_API_KEY`: a Google AI Studio / Gemini API key

### Recommended Repository Settings

For a stricter setup, configure the repository itself to match the workflows:

- Protect `main`
- Require pull requests before merging
- Require status checks to pass before merging
- Require the `Autonomous Verify / verify` check
- Restrict direct pushes to `main`
- Allow GitHub Actions to create and approve pull requests only if you explicitly want full autonomy

### Safety Boundary

The automation is intentionally fail-closed rather than "always modify something."

- Model discovery failure: skip safely
- Generation failure: skip safely
- Invalid diff: skip safely
- Formatting or test failure: stop without PR
- Blocked path touched: stop without push
- Existing autonomous PR already open: stop without new proposal
- PR not old enough: stop without merge
- PR checks not green: stop without merge
- PR not cleanly mergeable: stop without merge

This reduces the chance of repository corruption, but it does not make autonomous coding "100% safe."

## Current CLI Surface

The binary is currently structured to accept a small set of configuration flags before startup:

- `--profile minimal|focus|privacy|full-replay`
- `--allow-app <exe-name>`
- `--exclude-app <exe-name>`
- `--enable-encryption`
- `--ack-full-replay`
- `--ack-keyboard-warning`

The current startup path validates unsafe combinations before recording begins. For example, `full-replay` requires explicit acknowledgement, and `focus` mode requires some scoping rule.

Session logs are currently written under `data/sessions/`. The storage layer also maintains:

- `session-index.log`: append-only summary of persisted sessions
- `last-session.txt`: pointer to the most recently persisted session file

## Safety Boundary

This project must not behave like spyware, a stealth recorder, or a keylogger. If a proposed feature creates that ambiguity, it should be redesigned.
