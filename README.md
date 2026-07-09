# Diavola

**Diavola** is a desktop app that launches and supervises your project's
development commands. Describe your processes in a `diavola.yml` file, and Diavola
starts everything in the right order, checks that each service is ready, and
shuts it all down cleanly.

No more scattered terminal tabs and sticky notes to remember startup order. One
file, one click, everything runs.

---

## How It Works

1. Create a `diavola.yml` in your project root
2. List your commands (API, database, worker, etc.)
3. Open Diavola, import your project
4. Click **Start** — Diavola launches everything in dependency order, waits for
   each service to be ready, then moves to the next
5. Watch logs per process, use the integrated terminal, and click **Stop** to
   shut everything down

---

## Configuration Reference

The config file is plain YAML. Edit it with any text editor or directly inside
Diavola using the built-in form editor.

### Structure

```yaml
env:                 # optional — shared env vars
  NODE_ENV: development
  DATABASE_URL: postgres://localhost:5432/myproject

logEntryPattern: "\\[\\d{2}:\\d{2}:\\d{2}\\]"  # optional — regex matching start of a new log entry

processes:           # required — at least one process
  <name>:
    kind: task | service              # required
    cmd: <shell command>              # required
    env:                              # optional — per-process overrides
      PORT: "3000"
    dependsOn:                        # optional
      <other_process>: success | ready
    ready:                            # optional (services only)
      type: log | http | delay | command
      # ... type-specific fields
    logEntryPattern: "\\[.*\\]"   # optional — overrides global
```

### Process Kinds

| Kind | Behavior |
|------|----------|
| `task` | Runs a command that finishes on its own (install, migrate, compile). Diavola waits for it to exit. If it fails (non-zero exit), everything stops. |
| `service` | Runs a long-lived process (server, worker). Diavola starts it and monitors its readiness. If it stops unexpectedly, everything stops. |

### Log Entry Grouping (`logEntryPattern`)

By default, every output line is treated as a standalone log entry. You can
provide a regex pattern that matches the start of a new log entry (e.g. a
timestamp, a log level prefix, a process tag). Lines that don't match
the pattern are visually grouped as continuations of the previous entry.

Set it globally or per-process (per-process overrides global). If the regex is
invalid, a warning is logged and all lines are treated as standalone entries.

```yaml
logEntryPattern: "^(ERROR|WARN|INFO|DEBUG|TRACE)\\b"  # global — regex for log grouping

processes:
  worker:
    kind: service
    cmd: npm run worker
    logEntryPattern: "^\\[\\d{2}:\\d{2}:\\d{2}\\]"    # per-process override (timestamp prefix)
```

### Dependencies (`dependsOn`)

Processes start in dependency order. Diavola resolves the graph, detects cycles,
and reports them as errors.

| Condition | Use with | Meaning |
|-----------|----------|---------|
| `success` | `task` | Wait for the task to exit with code 0 |
| `ready` | `service` | Wait for the service to pass its readiness check |

```yaml
api:
  kind: service
  cmd: npm run dev
  dependsOn:
    setup: success     # wait for setup task to finish
    database: ready    # wait for database service to be ready
```

### Readiness Checks (`ready`)

For `service` processes, Diavola needs to know when the service is actually ready.
There are four check types:

#### Log

Wait for a log line to appear in the process output.

```yaml
ready:
  type: log
  pattern: "listening on port 3000"
  timeoutMs: 30000    # optional, default: 60000

# or with regex:
ready:
  type: log
  pattern: "listening on (port )?\\d+"
  regex: true
```

#### HTTP

Poll a URL until it returns a 2xx response.

```yaml
ready:
  type: http
  url: http://localhost:3000/health
  intervalMs: 1000    # optional, default: 1000
  timeoutMs: 30000    # optional, default: 60000
```

#### Delay

Wait a fixed duration after the process starts.

```yaml
ready:
  type: delay
  durationMs: 2000
```

#### Command

Run a command repeatedly until it exits with code 0. The command must be
idempotent (safe to run multiple times).

```yaml
ready:
  type: command
  cmd: curl -sS http://localhost:3000/health
  intervalMs: 1000    # optional, default: 1000
  timeoutMs: 30000    # optional, default: 60000
```

### Environment Variables

Global `env` applies to all processes. Per-process `env` overrides global values
for that process only. Readiness check commands also receive these variables.

```yaml
env:
  NODE_ENV: development

processes:
  api:
    kind: service
    cmd: npm run dev
    env:
      PORT: "3000"     # overrides NODE_ENV? No — merged on top
```

### Multi-line Commands

Use YAML's `|` for multi-line scripts:

```yaml
api:
  kind: service
  cmd: |
    echo "starting API..."
    cd ./packages/api
    npm run dev
```

---

## Examples

### Web Project

```yaml
processes:
  install:
    kind: task
    cmd: npm install

  migrate:
    kind: task
    cmd: npx prisma migrate dev
    dependsOn:
      install: success

  api:
    kind: service
    cmd: npm run dev
    dependsOn:
      migrate: success
    ready:
      type: log
      pattern: "Server is listening"

  frontend:
    kind: service
    cmd: npm run dev --workspace=frontend
    dependsOn:
      api: ready
    ready:
      type: http
      url: http://localhost:5173
```

### Python + Redis

```yaml
env:
  PYTHONUNBUFFERED: "1"

processes:
  install:
    kind: task
    cmd: pip install -r requirements.txt

  redis:
    kind: service
    cmd: redis-server
    ready:
      type: command
      cmd: redis-cli ping

  api:
    kind: service
    cmd: uvicorn main:app --reload --port 8000
    dependsOn:
      install: success
      redis: ready
    ready:
      type: http
      url: http://localhost:8000/health

  worker:
    kind: service
    cmd: celery -A tasks worker --loglevel=info
    dependsOn:
      redis: ready
    ready:
      type: log
      pattern: "celery@.* ready"
      regex: true
```

---

## Integrated Terminal

Each project window includes an integrated terminal opened in the project
directory for running one-off commands (tests, linting, scripts) without leaving
the app.

## Editing Configs

Diavola offers two modes for editing `diavola.yml`:

- **Form mode** — structured fields for each option; no YAML knowledge needed
- **Raw YAML mode** — built-in text editor with full YAML support and
  validation

## Launching

```bash
deno task app path/to/diavola.yml   # launch with a config
deno task app                     # launch, then import a project from the UI
```

From the desktop window you can import project directories, create configs from
scratch, and start/stop your project.

## Tips

- Always use a `ready` check for services — without one, Diavola moves on
  immediately
- Prefer `type: log` when your service prints a startup message; use `type:
  http` for services with a health endpoint
- `timeoutMs` defaults to **60 seconds**; increase it if your service starts
  slowly
- Failed tasks stop everything; fix the error and restart
- Multi-project: open multiple Diavola windows for separate projects
