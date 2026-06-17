# Architecture

jet sits between an editor (Neovim, eventually) and a Jupyter kernel. It does
not run kernels itself — that's [kallichore](https://github.com/posit-dev/kallichore)'s
job. jet is the thin client that drives a session and renders its output.

## The four processes

```
   ┌─────────┐  spawn / attach   ┌──────┐  HTTP + WS   ┌──────────┐  ZMQ   ┌────────┐
   │ Neovim  │ ────────────────▶ │ jet  │ ───────────▶ │ kcserver │ ─────▶ │ kernel │
   └─────────┘                   └──────┘              └──────────┘        └────────┘
       editor                     client                supervisor          ipykernel
                                                       (kallichore)         / ark / …
```

- **kernel** — a Jupyter kernel process (ipykernel, ark, etc.). Speaks the
  Jupyter wire protocol over ZeroMQ. Knows nothing about jet, nvim, or HTTP.
- **kcserver** — the kallichore supervisor. Spawns kernels, owns the ZMQ
  sockets, exposes a clean HTTP + WebSocket API to clients. Long-running by
  design: sessions outlive any one client connection.
- **jet** — the client. Speaks HTTP + WebSocket to kcserver. Reads stdin,
  sends `execute_request`s, renders kernel output (text, errors, plots) to
  stdout. Has no kernel knowledge beyond the Jupyter message protocol.
- **Neovim** (eventually) — drives jet as a plugin. Sends code, displays
  output. Owns the editing experience.

The split is deliberate: each process has one job. A kernel runs code. kcserver
manages kernels. jet renders. Neovim edits.

## Who owns what

**Sessions live on kcserver, not on jet.** A session is a server-side resource
with its own id and lifecycle. Clients come and go; the session stays:

1. `PUT /sessions` — register: argv, language, working dir, env. Kernel not
   running yet.
2. `POST /sessions/{id}/start` — launch the kernel. Status moves
   `Uninitialized → Starting → Ready → Idle`.
3. `GET /sessions/{id}/channels` — websocket upgrade. The only client-specific
   resource. Closing it does **not** end the session.
4. `POST /sessions/{id}/{interrupt,restart,kill}` — lifecycle controls. Any
   client can issue them.
5. `DELETE /sessions/{id}` — remove the bookkeeping.

This is the property that makes the editor integration interesting: jet can
exit and reattach later, or a fresh nvim can pick up an existing session by id.

**Working directory and env are captured at create-time, by jet.** kcserver
launches the kernel with whatever cwd/env jet sent in the `new_session` body.
Today that's `std::env::current_dir()` and the inherited environment of
whichever shell ran jet. For the nvim plugin, this means the kernel's project
context is decided at session-creation, not by where kcserver itself runs.

**kcserver lifetime is decoupled from jet.** Today jet's `ChildGuard` kills
the server it spawned on drop, which is the simple-CLI case. The intended
shape for the nvim plugin is the opposite: kcserver is a detached daemon,
jet is one of possibly many transient clients, and the supervisor keeps
running across editor restarts. kallichore supports this directly via
`--idle-shutdown-hours` (auto-exit when no one's around).

## A request, end to end

1. nvim → jet: "run this code in session S."
2. jet builds an `execute_request` with a fresh `msg_id` and sends it on the
   shell channel (websocket → kcserver → kernel via ZMQ).
3. kernel runs the code, emits messages on iopub: `status: busy`, then any
   `stream` / `display_data` / `error` / `execute_result`, then
   `status: idle` with `parent_header.msg_id == msg_id`.
4. kcserver forwards each iopub message to jet over the websocket.
5. jet renders text/errors/plots to stdout as they arrive (eventually:
   forwards them to the nvim plugin instead).
6. jet's REPL loop notices the matching `Idle` and considers the request
   complete; the editor (or REPL prompt) is free to send the next one.

The kernel does not know anything about jet, the websocket, or the editor.
jet does not know anything about ZMQ. kcserver bridges the two protocols.

## Where the editor plugin will plug in

The Neovim plugin is **not** a fourth concept — it's a different driver of the
same `jet` client. The interesting decisions for the plugin are about
*lifecycle*, not *protocol*:

- **One kcserver per project**, keyed on the git root (or cwd). Plugin
  probes a stable connection-file path; connects if reachable, otherwise
  spawns kcserver detached with `--idle-shutdown-hours` and walks away.
  Kernels survive `:q`; the supervisor self-cleans when truly idle.
- **Sessions identified by id**, so the plugin can reattach to a kernel it
  started in a previous nvim run. `jet --session-id <id>` opens
  `/channels` on an existing session instead of creating a new one.
- **The plugin owns environment.** It captures cwd / venv / env at session
  creation, so the kernel's project context matches the editor's, not
  kcserver's.

The protocol path stays the same; what changes is who's at the top of the
diagram and how long they stick around.
