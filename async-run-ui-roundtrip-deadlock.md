# Forward-reference nil call + silent async.run errors (2026-10-17, CORRECTED)

Supersedes my earlier "async-run UI roundtrip deadlock" note, which had the WRONG root cause.

REAL BUG (maki-peer): refresh_session called local function current_session defined ~330 lines later. In Lua a local used before its `local function` line resolves to a GLOBAL (nil) -> "attempt to call a nil value" on every poll. Invisible for a day because:
1. Inside maki.async.run the error was swallowed SILENTLY: run() without on_finish enqueues the raw fn; the "errors are logged" docstring belongs to maki.async.spawn, NOT run. Silence != hang.
2. status (autocmds) and cwd (uv.cwd pre-seed) kept working, masking the dead read() path.
Became visible only when the call moved onto the pcall'd poll body ("peer poll: ...: attempt to call a nil value" every heartbeat in ~/.local/logs/maki/maki.log).

RULES:
- Order matters: any local function called from a file-top poll/timer must be defined ABOVE the caller; check with grep for definition vs call lines when moving code around.
- Never diagnose from missing log lines alone; a swallowed error and a hang look identical. Wrap or log errors explicitly at the failure site.
- maki.async.run: pass on_finish or expect NO error visibility. maki.async.spawn logs errors.
- UI roundtrips (maki.session.current/read, maki-lua util/command.rs ui_roundtrip) DO answer from the main-Lua-thread callback context (proven live). Whether they work inside async.run is UNPROVEN - do not re-assert the old deadlock claim.
- Multi-agent file racing: do NOT edit a file while its owner agent may be active; my broken intermediate got git-checkout-reverted under me. Hand the owner an exact patch (line ranges + no body changes) instead.

Process lesson: one wrong early hypothesis ("deadlock") cost three agents an afternoon; the cheap pcall/warn canary in the poll body was what actually surfaced it.

(Host-side locations for reference: maki-lua/src/api/async.rs run() enqueues raw fn with no error logging; maki-lua/src/api/util/command.rs ui_roundtrip docs "only works from a callback".)
