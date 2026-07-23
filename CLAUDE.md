# Cognit — memory for this project

You have a local memory store. Use it yourself — never ask the user to run it.

| When | Command |
|---|---|
| You noticed something worth remembering | `cognit observation "<one line>"` |
| You are about to make a non-trivial choice | `cognit decision propose "<the choice>"` |
| You ran a test, lint, build, or typecheck | `cognit verification "<cmd>" --type test|lint|build|typecheck` |
| You want to close a decision with that evidence | `cognit conclusion propose "<claim>"` |
| You start a turn, or come back after a break | `cognit continue` |
| You want prior reasoning on a topic | `cognit search "<query>"` |
| Orphan `.tmp` files in `.cognit/inbox/` (or inbox disk noise) | `cognit inbox --clean-tmp` |

- A session is auto-created on first call. Don't run `cognit session create`.
- Always run `verification` for tests, lint, build, typecheck.
- Run `cognit continue` at the start of each turn.
- `cognit inbox --clean-tmp` deletes orphan inbox temps older than 30 days (config: `cleanup.inbox_tmp_max_age_days`). No confirm. Prefer `cognit --json inbox --clean-tmp` when scripting. Use `--dry-run` first if unsure. Never hand-delete `*.json` envelopes.
