# Product masterlist

Status: approved on 2026-08-19. Active milestone: complete, polished composer parity.

This document is the acceptance baseline and durable progress ledger. Do not delete, weaken, or silently
reinterpret a requirement. Strike through a requirement only after implementation, automated checks,
and the relevant manual test pass; keep the completed text visible. Amend scope only when the user
explicitly changes it. Continue implementing, testing with the user, and iterating until the entire
document is complete.

## Product principles

- Performance is a feature. Direct manipulation and key-repeat paths stay below 1 ms of CPU work and
  never perform server, filesystem, parsing, image, or persistence work on GPUI's thread.
- OpenCode's server and event stream are authoritative. Indicators reflect real protocol state rather
  than decorative animation, and the client never reads OpenCode's database directly.
- Geometry is systematic. Pane edges, gutters, markers, headers, scrollbars, composer controls, and
  inspector sections derive from shared dimensions rather than per-component offsets.
- Color communicates hierarchy. The dark base remains visible; panel, elevated, hover, selected,
  border, status, and semantic colors have distinct jobs.
- Desktop interactions behave like desktop interactions: text selects, links open, diagrams pan and
  zoom, controls expose hover/focus state, and state persists when persistence is promised.
- Background updates retain useful content. Loading or refresh work must not flash, blank, jump, or
  replace already-rendered state.
- Long content and expensive output remain bounded without making ordinary content feel constrained.

## Immediate approval scope

These are the next implementation tasks requested in the latest manual test pass.

### Current correction pass

- ~~Position composer attachment descriptors without overlap or crowding. Keep distinct backgrounds for
  the type and filename descriptors, rename clipboard-generated files and labels to `image`, and apply
  the same descriptor treatment to image attachments in the conversation trace.~~
- Restore equal outer composer spacing. Keep the real session-status indicator, but integrate it into
  the composer's internal grid instead of rendering a separate left-side card.
- Surface rejected shell submissions and other composer errors in a prominent visible error region;
  silently refusing a submission is not acceptable.
- ~~Keep directory-picker title and search controls pinned while result rows scroll. Cycling from the
  first row to the last must never scroll the search field out of view.~~
- ~~Never truncate expanded tool output or inspector output. A collapsed preview may summarize content,
  but explicit expansion means the user requested the complete result, regardless of line count.~~
- ~~Match OpenCode TUI's message hierarchy: user turns are cohesive subtly differentiated blocks;
  assistant content is the default flat reading flow; assistant agent/mode, model, and elapsed duration
  appear as restrained metadata after the completed response rather than as a repeated leading banner.~~
- Vertically center the assistant completion icon with its metadata text.
- ~~Keep the persistent right inspector visible at narrower desktop widths when enough usable content
  space remains. Use pane minimum widths and available layout space rather than the current coarse
  fixed breakpoint.~~
- Persist the ordered set of open directories and the active workspace across launches. Validate stale
  paths off GPUI's thread, preserve an intentionally empty window, and keep `OPENCODE_DIRECTORY` as an
  explicit startup override.
- Complete directory-picker paths with Tab using shell-style common-prefix and trailing-slash behavior.
  Keep directory enumeration off GPUI's thread.
- Keep Ctrl+Tab and Ctrl+Shift+Tab workspace cycling comfortably inside the direct-interaction budget,
  including sustained key repeat with many sessions, drafts, and active statuses.
- Preload every restored workspace's selected timeline and derived Markdown, image, diff, and sidebar
  state in the background. Cycling tabs must never trigger a lazy session or timeline load.
- Coalesce workspace content redraw, editor focus transfer, and persistence until Ctrl+Tab key repeat
  settles. Individual cycle events update only the in-memory target index and never invalidate the full
  workbench view.
- Render the tab strip as an isolated lightweight view so every Ctrl+Tab event highlights its target
  immediately while conversation content remains settle-debounced.
- Identify MCP calls by server and tool in both collapsed summaries and expanded detail. Friendly
  result titles supplement identity rather than replacing it.
- When tail following is disabled, expanding or collapsing trace detail preserves the current scroll
  offset and grows downward instead of preserving the viewport's lower edge.

Acceptance:

- ~~Composer and conversation image descriptors are aligned, readable, and never overlap their remove
  control or preview.~~
- Composer outer top, bottom, left, and right gaps are equal, and the status indicator reads as part of
  the composer rather than a second card.
- Rejected shell attachments produce a clearly visible error.
- ~~Holding Up on the first directory result selects the last result while the search field stays pinned.~~
- ~~Expanding a 400+ line shell/tool result exposes every line in both timeline detail and inspector.~~
- ~~User and assistant turns can be distinguished at a glance without repetitive role headers.~~
- The assistant completion icon and metadata text share one visual baseline.
- ~~The right inspector remains visible at the tested split-window width shown in the manual screenshot.~~
- Restarting restores every open directory in order and selects the workspace that was active before
  shutdown; closing every workspace and restarting returns to the directory picker.
- Tab extends ambiguous directory paths to their common prefix and completes a unique directory with a
  trailing slash so repeated Tab presses can descend through the tree.
- Holding Ctrl+Tab or Ctrl+Shift+Tab cycles workspaces immediately without dropped input or visible lag;
  the measured switch path remains below 1 ms at p99.
- After startup hydration completes, every restored tab opens from memory while autocycling; no tab
  displays a new loading state or starts network work because it became active.
- Sustained autocycling does not enqueue one render, focus transfer, or disk write per key event; the
  measured dispatch path remains comfortably below 1 ms at p99.
- Every tab crossed during autocycling visibly highlights in sequence without triggering a conversation
  render for each key event.
- A GitHub MCP result visibly names `github / <tool>` before expansion and repeats that identity in the
  expanded tool section.
- Expanding and collapsing a visible trace part while scrolled above the tail does not move the content
  above it or leave the viewport displaced afterward.

### Selectable text everywhere

- Make user and assistant prose selectable with normal pointer dragging.
- Preserve selection across Markdown emphasis, links, inline code, paths, task text, headings, lists,
  quotes, tables, code blocks, and streamed content.
- Make relevant chrome text selectable: session rows, pane headers, status lines, persistent context,
  MCP/LSP state, todos, modified files, tool summaries/output, diff content, and inspector details.
- Keep buttons, resize handles, activity controls, disclosure controls, and image manipulation as
  controls rather than selectable labels.
- Use the normal arrow cursor over links instead of a hand cursor. Clicking a link still opens it and
  does not select the parent trace row.
- Preserve normal click-drag selection without the trace row stealing the gesture to select/expand a
  part. Part selection must move to a dedicated affordance or a non-drag click path.
- Support copying the selected text with the platform-standard shortcut.
- Allow one selection to span separate timeline parts and pane boundaries. Design the selection as one
  coherent desktop interaction rather than a collection of isolated selectable labels.
- Study OpenCode TUI's source implementation before choosing the cross-part selection architecture.

Acceptance:

- A drag can select text from an assistant paragraph and copy the exact visible text.
- Selection works inside code, tables, tool output, diffs, todos, and inspector JSON.
- Session rows, headers, status/context values, MCP/LSP entries, modified files, and streamed prose can
  be selected and copied with the platform-standard shortcut.
- Dragging over a link selects it; clicking without dragging opens it with an arrow cursor throughout.
- Selection does not unexpectedly expand tools, open the inspector, or jump the timeline.

### Composer and activity gutter

This is the active milestone. Target functional and visual parity as one continuous implementation
tranche rather than a sequence of placeholder menus.

- Replace the awkward empty space to the left of the composer with a fixed activity cell aligned to
  the trace marker/kind gutter.
- Drive that cell from OpenCode's real session status and events: idle, busy/working, retrying, and
  failed. Do not fake activity with an unrelated cosmetic loader.
- Show enough state to distinguish active model work, active tool execution when available, retry
  delay/attempt state, completion, and failure without turning the gutter into a dashboard.
- When the active or expanded trace point exposes it, show elapsed time and tool name in the activity
  cell.
- Keep the prompt text column aligned with conversation content while making the composer/activity row
  read as one intentional full-width structure.
- Give top, bottom, and outer-edge spacing one shared value.
- Align the composer's right boundary with the conversation viewport's right boundary at the outer
  edge of the scrollbar gutter. Reserve scrollbar width in the conversation layout rather than letting
  the card terminate at the scrollbar centerline.
- Keep attachment cards, completion overlays, multiline growth, IME bounds, and editor hit-testing
  aligned with the resulting composer geometry.
- Audit the current OpenCode server schema, SDK, TUI source, commands, and capability endpoints before
  declaring parity. Implement every composer-adjacent menu the server exposes, including agent,
  provider/model, variant, command, skill, MCP command, file/reference, and attachment flows.
- Match upstream menu availability, labels, grouping, aliases, disabled/error states, keyboard
  navigation, selection preservation, and submission semantics instead of hard-coding a partial list.
- Cover normal prompts, shell mode, server slash commands, local commands, file mentions, image
  attachments, multiline editing, history/drafts, abort, model/agent switching, and capability changes
  that arrive while the app is running.
- Keep completion filtering and repeated keyboard navigation within the direct-interaction performance
  budget regardless of menu size.

Acceptance:

- Idle and actively working sessions show different states based on server data.
- Tool execution, retry delay/attempt, completion, and failure are distinct when the protocol exposes
  enough state; retry state reflects the server event rather than a generic spinner.
- The left activity cell, prompt card, conversation content, and scrollbar form one visible grid.
- Composer and conversation right edges line up at every supported pane width.
- Every composer menu and mode exposed by the connected OpenCode version is reachable, keyboard
  navigable, correctly submitted, and visually integrated rather than represented by a placeholder.

### Mermaid viewport quality

- Research current Mermaid rendering practices and renderer capabilities before changing the pipeline.
- Remove the white diagram canvas. The SVG background is transparent or uses the application base
  surface, and all nodes, labels, edges, clusters, and controls meet dark-theme contrast requirements.
- Correctly theme every supported Mermaid diagram family rather than only flowchart defaults.
- Render diagrams in an interactive viewport with pointer-drag panning, wheel/trackpad zoom centered on
  the pointer, explicit zoom in/out, fit-to-view, and reset controls.
- Clamp zoom to useful bounds and preserve crisp SVG rendering at every scale.
- Prevent diagram gestures from scrolling or selecting the parent timeline until the gesture reaches a
  viewport boundary where propagation is intentional.
- Make the viewport keyboard focusable and provide keyboard pan/zoom equivalents.
- Preserve the bounded background renderer, input/output limits, cache invalidation, and styled source
  fallback on unsupported or invalid diagrams.
- Persist a diagram's view only while its source and session part identity remain unchanged; source
  changes reset to fit-to-view.
- Diagram viewport state is process-local and does not survive an app restart.

Acceptance:

- The current Crunchyroll flowchart has no white strip or low-contrast black nodes.
- Representative flowchart, sequence, state, class, ER, and timeline diagrams use the same dark-theme
  hierarchy; unsupported diagram families visibly fall back to source.
- Labels remain readable and the full graph initially fits the available width.
- Mouse, trackpad, and keyboard users can pan, zoom, fit, and reset without moving the timeline.
- Streaming or reopening a session never blocks GPUI's thread to render Mermaid.
- Invalid and oversized sources preserve styled fallback, and changing source resets the viewport to
  fit rather than reusing stale pan/zoom state.

### Stable sidebar refresh

- Keep the last successful session-context snapshot visible while a refresh is in flight.
- Do not replace `Ready` with `Loading` for background refreshes.
- Update MCP, LSP, todo, context, and modified-file sections independently when practical.
- Preserve scroll position, selected inspector detail, expansion state, and section geometry across
  refreshes.
- Ignore snapshots for stale session/directory requests.
- Only notify GPUI when visible data actually changed.
- Show an unobtrusive stale/error state when refresh fails without blanking good data.

Acceptance:

- Todo and MCP changes update in place without a flash, blank frame, or scroll jump.
- Refresh preserves selected detail, expansion, section geometry, and last-good content on failure.
- Rapid events coalesce into bounded refresh work.
- Switching sessions during a refresh cannot paint the old session's context.

### Key-repeat performance

- Profile sustained Up/Down key repeat in command, directory, completion, session, and similar menus.
- Keep selection movement O(1) and free of cloning proportional to menu size.
- Avoid reparsing queries, rebuilding unchanged row models, server calls, persistence, or redundant full
  workspace notifications on each repeat event.
- Coalesce scroll-to-selected work to the display frame when repeated input outpaces rendering.
- Keep mouse hover and keyboard selection synchronized without generating feedback loops.
- Add a repeat-path performance regression test with a realistic result count.

Acceptance:

- Holding Down cycles continuously without visible input lag or delayed key release.
- A selection step remains below the direct-interaction CPU budget.
- Menu movement performs no network or filesystem activity.
- Mouse hover and keyboard selection remain synchronized, and a sustained-repeat regression test
  covers a realistically large result set.

## Previously requested follow-up scope

These requests remain part of the product direction but follow the immediate approval scope unless the
user reprioritizes them.

### Composer completeness

- Smooth explicit-newline and soft-wrap growth/shrink up to the eight-line cap.
- Preserve multiline drafts, mentions, files, and images per directory/session across restarts.
- Retain word movement/deletion, selection, undo/redo, IME, clipboard images, completion ownership of
  Enter, and all supported newline shortcuts.
- Keep image preparation, filesystem completion, and server submission off GPUI's thread.

### Markdown and rich content

- Preserve current headings, paragraphs, quotes, rules, lists, tables, code, links, emphasis,
  strikethrough, task states, paths, Mermaid, inline/display math, and partial-streaming behavior.
- Add semantic footnotes with navigable references and return links.
- Add complete `<kbd>` components rather than text styling alone.
- Add accessible plaintext/source alternatives for rendered diagrams and math.
- Support additional diagram languages only through explicit bounded renderers; do not label unsupported
  Graphviz, DOT, or PlantUML source as rendered output.

### Attachments

- Keep optimistic/server image reconciliation at exactly one timeline item.
- Refine attachment cards around one preview, filename, state, and removal affordance without nested
  decorative badges.
- ~~Keep distinct backgrounds behind the attachment type descriptor and filename descriptor.~~
- Preserve native clipboard paste, background encoding, draft restoration, normal prompt/slash
  transport, timeline previews, limits, and clear error states.

### Tool calls and diffs

- Keep compact tool rows distinct from prose and detect tools by protocol content rather than one kind.
- Keep Read/Grep/Glob/bash output bounded, aligned, and readable.
- Keep completed patch diffs expanded by default with a persisted command-palette preference and an
  explicit per-part collapse override.
- Extend the diff viewer with file navigation, unified/split modes, hunk folding, syntax highlighting,
  stable anchors, comments-ready geometry, and keyboard navigation.
- Do not lose streamed diff updates to stale prepared-detail caches.
- Pressing Escape while a trace part is expanded collapses it and clears its inspector selection.
  Without Escape, an expanded part may remain expanded and focused in the inspector.

### Persistent session context

- Keep session title and share/workspace metadata at the top.
- Show context tokens, model limit percentage, cost, MCP states, LSP states, live todos, and modified
  files with additions/deletions.
- Add abbreviated directory, VCS branch, and OpenCode version to the footer.
- Preserve selected-part details in the same continuous inspector parent instead of replacing session
  context.
- Hide or collapse sections with no useful content while keeping layout stable.

### Tabs and directory identity

- Make `Ctrl+T` create a new tab even when that directory is already open.
- Introduce stable `TabId`; directory remains server scope and never serves as tab identity.
- Route editor state, loads, completion, drafts, images, details, and caches by `TabId`.
- Fan directory-scoped SSE events into every matching tab while sharing one directory connection.
- Mark already-open directory results and provide a separate focus-existing action.
- Decide shared-draft behavior explicitly for duplicate tabs on the same session.

### Slash-command and capability parity

- Retain `/sessions`, `/resume`, `/continue`, `/new`, `/clear`, `/workspaces`, `/move`, `/help`,
  `/exit`, `/quit`, and `/q`.
- Add capability-aware `/models`, `/agents`, `/mcps`, `/variants`, `/connect`, `/org`, `/status`,
  `/debug`, and `/themes` experiences, including `/mo`, `/orgs`, and `/switch-org` aliases.
- Add `/share`, `/unshare`, `/rename`, `/timeline`, `/fork`, `/compact`, `/summarize`, session
  `/undo`, session `/redo`, `/copy`, `/export`, `/timestamps`, and `/thinking`; `/summarize` aliases
  `/compact`, `/toggle-timestamps` aliases `/timestamps`, and `/toggle-thinking` aliases `/thinking`.
- Add `/editor`, `/skills`, and experimental `/warp` when the server exposes the capability.
- Inventory runtime commands from `command.list`; distinguish config, MCP, skill, plugin, and built-in
  sources rather than hard-coding extension commands.
- Implement each command as its real feature/dialog/action, not as a placeholder entry.

### Navigation, persistence, and desktop integration

- Complete keyboard navigation and focus handling across tabs, panes, overlays, timeline parts, tool
  details, diagrams, and the inspector.
- Persist user-facing layout, theme, keybinding, expansion, and visibility preferences with versioned,
  atomic config writes.
- Add export, update, sharing, notifications, permissions/questions, formatter state, file tree/search,
  VCS state, snapshots, revert/fork, and PTY support according to server capabilities.
- Validate desktop and narrow-window layouts without hiding required state or overlapping controls.

### File explorer

- Add a fast native file explorer to the left sidebar alongside the session explorer.
- Show hidden files and directories by default.
- Use background, cancellable directory enumeration and incremental updates; never walk the project on
  GPUI's thread.
- Support expand/collapse, open, rename, create file/directory, duplicate, delete with confirmation,
  copy path, copy relative path, reveal externally, and refresh through intuitive context menus.
- Add keyboard navigation for next/previous row, parent/child traversal, expand/collapse, open, rename,
  delete, context menu, type-to-filter, and focus transfer between session and file explorers.
- Preserve expansion, selection, and scroll state per directory tab. Reconcile filesystem changes
  without rebuilding or flashing the whole tree.
- Respect ignore data only as an optional filter; hidden and ignored entries remain discoverable by
  default unless the user changes the setting.
- Reuse the shared selection/menu infrastructure rather than introducing a file-explorer-only menu
  system.

Acceptance:

- A large repository opens without blocking interaction, and expanding one directory enumerates only
  the required subtree.
- Dotfiles and hidden directories are visible on first use.
- Keyboard and context-menu operations are complete without requiring the mouse.
- External file changes update the affected subtree while preserving tree position and expansion.

## Already shipped and retained

These behaviors are not new tasks, but later work must not regress them.

- Directory tabs, server bootstrap, directory-scoped SSE, session lists, timeline loading/history, and
  live streamed-part reconciliation.
- Native multiline composer, session drafts, filesystem mentions, clipboard images, image restoration,
  attachment reconciliation, and prompt/slash transport.
- Markdown parsing and streaming stability, clickable links, visual task states, Mermaid and LaTeX SVG
  rendering, tables, paths, code blocks, and styled emphasis.
- Tokyo Night semantic palette, visible base surface, stronger structural hierarchy, shared trace gutter,
  aligned markers, and continuous inspector composition.
- Compact tool rows, bounded output, structured patch diffs, contrast-tested line numbers, default
  expansion, persisted expansion preference, and streamed-detail refresh.
- Persistent context snapshot with context usage, MCP/LSP, todos, modified files, unabbreviated
  directory, and OpenCode version. Abbreviation and VCS branch remain follow-up work.
- Source-size enforcement, direct-interaction performance tests, formatting, strict Clippy, unit/API
  tests, release builds, CI, and post-build manual testing.

## Approval questions and decisions

1. Is the immediate scope ordered correctly: selection, composer/activity geometry, Mermaid viewport,
   sidebar stability, then key-repeat performance?

   Answer: Yes, but composer parity is now the active milestone and each implementation pass should
   complete substantially more scope. Composer parity includes auditing and implementing the menus and
   capabilities exposed by OpenCode, with polish rather than placeholders.

2. Should selected text be allowed to span across separate timeline parts and pane boundaries, or is
   complete selection within each rendered block sufficient?

   Answer: Selection must span separate parts and pane boundaries. Study and follow OpenCode TUI's
   proven implementation behavior.

3. Should the activity gutter show only state or also elapsed time/tool name when protocol data exists?

   Answer: Show elapsed time and tool name when an expanded trace point exposes them. Pressing Escape
   while a trace point is expanded must collapse it and clear its inspector selection; otherwise it may
   remain expanded and focused in the inspector.

4. Should diagram pan/zoom state survive app restarts, or only rerenders during the current process?

   Answer: Keep diagram viewport state only during the current process. It does not survive restarts.

5. Should automatic diff expansion remain enabled by default for every workspace?

   Answer: Yes. Automatic diff expansion remains enabled by default for every workspace.

6. Which previously requested follow-up section should become the next milestone after the immediate
   scope?

   Answer: Composer parity is the current priority. Continue through the rest of this document after
   composer parity, striking completed requirements while retaining their text.

Additional approved decision: attachment type and filename descriptors retain distinct background
surfaces.
