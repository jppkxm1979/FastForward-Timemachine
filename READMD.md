# FastForward-Timemachine Working Notes

## Purpose

This file mirrors the project direction from `agents.md` and serves as a human-readable working brief for ongoing development.

FastForward-Timemachine is a local-only activity replay engine focused on transparent recording, privacy controls, and efficient timeline reconstruction.

## Non-Negotiable Constraints

- No hidden or stealth recording behavior
- No network transmission, analytics, or sync
- No raw keyboard text storage by default
- User-visible toggles for each recording source
- Privacy filters for excluded apps, private browsing contexts, and password-entry contexts where feasible

## Delivery Priorities

1. Efficient screen capture pipeline
2. Timeline storage format
3. Replay path
4. Safe event enrichment
5. Visualization later

## Immediate Build Direction

- Use Rust as the primary implementation language
- Keep modules separated by responsibility
- Expose a CLI that always reports recording status
- Start with stubs that make privacy boundaries explicit before adding OS integrations

## Current Next Steps

- Add Windows-specific capture/process abstraction traits without implementing stealth behavior
- Define a serializable session manifest and storage layout
- Add explicit CLI commands for `start`, `stop`, and `status`
- Replace placeholder timestamps with a monotonic clock source

## Progress Snapshot

- Recorder now validates profile safety rules before `start`
- CLI command surface now distinguishes `start`, `status`, and `stop`
- Stub capture and process backends define the integration seams for future WinAPI work
- Timeline sessions can now render to simple log lines for a future storage layer
- Session events now use a monotonic elapsed clock and can be written under `data/sessions/`
- Session files now carry explicit session IDs instead of profile-only filenames
