# Engineering constraints

These are repository invariants, not suggestions.

- Production source files must stay at or below 300 lines. Split by responsibility before crossing the limit.
- Tests belong beside the unit they specify or in a dedicated test module; test volume does not justify a giant source file.
- UI components own one coherent surface. Shell, panes, timeline records, inspectors, editors, and overlays are separate modules.
- Protocol models, transport, event reduction, and rendering remain independent layers.
- Unbounded collections are virtualized or explicitly bounded. Streaming updates are coalesced to one invalidation per frame.
- Direct interaction paths, including selection, expansion, focus, and pane transitions, must complete in under one millisecond of CPU time. Network access, parsing, formatting, and large payload preparation stay off GPUI's thread and require benchmark evidence.
- Unknown protocol variants and fields are preserved. Newer servers must degrade visibly rather than fail deserialization.
- Every feature has a typed contract test and, where user-facing, an actual-usage smoke test.
- The OpenCode server owns execution, credentials, permissions, plugins, MCP, persistence, and snapshots.
- The client never reads OpenCode's database directly or duplicates server-side permission enforcement.
- `cargo fmt`, strict Clippy, tests, and release builds must pass before each push.
