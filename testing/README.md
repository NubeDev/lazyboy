# Lazyboy UI tests

Browser-driven tests that exercise the React UI against a **live** backend —
the real `lazyboy-server` (HTTP+SSE core) and the Vite dev server, not a mock.
A failure here points at the actual app, not at fixture drift.

The tests use [Playwright](https://playwright.dev). They wait on DOM state,
never on `networkidle`: the app polls on 15s/60s timers (goose health, the
relative-time clock), so the network never goes idle.

## What is covered

### Reminders (`reminders.spec.ts`)

The first feature wired end to end and proven through the browser. Reminders
are pure Lazyboy SQLite — they need no configured goose provider — and have a
complete lifecycle, which makes them the cleanest full-stack slice:

```
RemindersPanel  ->  RpcClient.createReminder / dismissReminder
                ->  POST /spaces/:id/reminders  ·  POST /reminders/:id/dismiss
                ->  repo::reminder  ->  SQLite  ->  back up as a DTO
```

The spec drives that whole path from the rendered UI:

- **create** — type a body + future due time, click Add; the row appears as
  pending (not overdue, not dismissed) and the form clears.
- **dismiss** — create, then click Dismiss; the row stays in the list but
  flips to the `dismissed` badge and loses its Dismiss action.
- **form gating** — Add stays disabled until both body and due time are set.

Each run tags its reminder with a unique body (`pw-reminder-<hrtime>`), so
re-runs never collide and assertions scope to the row the test created.

## Running

Bring the stack up first; the tests do not spawn it.

```bash
# from the repo root — backend (detached, survives make exit via setsid)
# plus the Vite UI in the foreground:
make dev
```

`make dev` serves the UI on <http://localhost:5181> and the backend on
<http://127.0.0.1:7878> (the UI's `VITE_CORE_URL` default). Then, in another
shell:

```bash
cd testing
npm install                       # first time only
npx playwright install chromium   # first time only
npx playwright test
```

Override the UI origin with `UI_URL` if you serve it elsewhere.

## Notes

- Tests run serialized (`workers: 1`): they share one space's reminder list,
  so a parallel run could dismiss a row another test is asserting on.
- Created-but-not-dismissed reminders accumulate in the dev DB across runs.
  That is harmless — each test scopes to its own unique row — but reset with a
  fresh `lazyboy init` DB if the panel gets noisy.
