# OpenCode parity plan

This client consumes the OpenCode HTTP/SSE protocol. OpenCode remains responsible for execution,
credentials, configuration resolution, plugins, MCP, permissions, snapshots, and persistence. Direct
database access is intentionally out of scope because it couples the UI to unstable internals and
cannot safely coexist with other clients.

## Performance invariants

- Hydrate bounded metadata, virtualize every unbounded collection, and never render per token.
- Keep HTTP, SSE parsing, syntax work, and disk access off GPUI's platform thread.
- Coalesce streaming deltas once per frame and reconcile by stable protocol IDs.
- Recover from SSE loss by rehydrating authoritative REST state; do not assume event replay.
- Preserve unknown tagged-union members so newer servers degrade visibly rather than crash.
- Scope caches by server and directory. Never leak credentials into logs or persisted UI state.

## Stages

| Stage | Deliverable | Status |
| --- | --- | --- |
| 0 | Reproducible Rust/GPUI build, CI, protocol client, health negotiation | complete |
| 1 | Virtualized project/session browser, create/rename/delete, status, child sessions | active |
| 2 | Virtualized timeline, extensible message parts, SSE reconciliation, abort/retry | planned |
| 3 | Composer, attachments, commands, shell, file/reference completion | planned |
| 4 | Permissions/questions, todos, tool details, notifications | planned |
| 5 | Agents, providers/models/variants, auth, MCP/LSP/formatter status | planned |
| 6 | Diffs, snapshots, revert/fork, file tree/search, VCS, sharing | planned |
| 7 | Tabs/drafts/layout persistence, themes/keybindings, export, updates, PTY | planned |
| 8 | Cross-platform packaging, accessibility, localization, profiling gates | planned |

## Compatibility profile

The design explicitly supports large session histories, parent/child agent trees, long-running watcher
agents, direct and OpenAI-compatible providers, reasoning variants, custom agents/commands/skills,
local and remote MCP servers, per-agent permission overrides, system theming, visible thinking/tool
details, cost/token accounting, and multiple directories per project. These are protocol-driven features;
the GPUI process does not evaluate plugin code or duplicate server-side permission enforcement.

The live server's `/doc` contract is authoritative. Features absent from an older server must be hidden
or disabled through capability detection, not version-string guesses.
