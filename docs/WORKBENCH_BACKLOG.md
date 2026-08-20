# Workbench backlog

This is the durable source of truth for product feedback, upstream parity, and manual acceptance
criteria. Raw investigation notes belong in `tmp/`; resolved facts and decisions belong here.

## Current polish pass

### Composer

- [x] Grow immediately when a newline is inserted with Shift+Enter, Ctrl+Enter, Alt+Enter, or Ctrl+J.
- [x] Preserve comfortable top and bottom padding at every height.
- [ ] Smooth growth and shrinkage for explicit newlines and soft wrapping up to the eight-line cap.
- [x] Preserve multiline drafts, file mentions, and images across sessions and restarts.
- [x] Support standard word movement, deletion, selection, undo, and redo.

Acceptance:

- The second line is visible in the same frame as Shift+Enter.
- The final baseline and cursor never touch or cross the composer border.
- Slash and mention completion still own Enter while open and work after newlines.

### Attachments

- [x] Reconcile optimistic and authoritative file parts so one sent image renders once.
- [x] Render attachment names as deliberate pills/chips rather than loose metadata.
- [x] Paste native clipboard images and prepare data URLs off GPUI's thread.
- [x] Persist image drafts and render removable composer thumbnails.
- [x] Send image parts through normal prompts and server slash commands.

Acceptance:

- One attached image produces one timeline image before and after server reconciliation.
- Attachment chips communicate type, filename, processing state, and removal affordance.

### Tool calls

- [x] Keep tool calls in their compact tool-row presentation; never pass tool payloads through the
      general message Markdown renderer.
- [x] Detect tool parts by protocol content as well as the current literal `tool` kind.
- [x] Bound and format expanded Read/Grep/Glob output as tool detail rather than an unstructured wall.
- [x] Improve patch and diff presentation with file grouping, line numbers, hunk headers, and distinct
      added/removed/context surfaces.

### Markdown and rich text

- [x] Headings, paragraphs, quotes, rules, lists, tables, fenced code, links, bold, italic,
      strikethrough, inline code, task markers, footnote references, and partial streaming fences.
- [x] Verify and refine visible bold and italic treatment against the OpenCode TUI hierarchy.
- [x] Recognize bare URLs and safe file paths without turning arbitrary punctuation into links.
- [x] Render file paths as file/directory chips with a secondary path surface.
- [x] Improve task states for `[ ]`, `[x]`, and in-progress/custom markers.
- [ ] Add semantic footnotes, `<kbd>` chips, and LaTeX rendering.
- [ ] Render Mermaid diagrams rather than only labeling their source fences.
- [x] Improve table row separation and scanning contrast without dense grid noise.

Automatic detection requirements:

- URL parsing must use a real URL parser and trim balanced trailing punctuation.
- File-path detection must require path-like structure or a known project result; ordinary prose with
  slashes, dots, email addresses, or version numbers must not become links.
- Explicit Markdown links always win over automatic detection.

### Theme and readability

The user's OpenCode TUI uses `theme: system`, resolved from the Omarchy Tokyo Night terminal palette.
The GPUI client should match these semantic roles rather than its current unrelated palette.

Resolved core palette:

| Role | Color |
| --- | --- |
| base | `#1a1b26` |
| text | `#a9b1d6` |
| primary/cyan | `#449dab` |
| secondary/magenta | `#ad8ee6` |
| blue | `#7aa2f7` |
| green | `#9ece6a` |
| yellow | `#e0af68` |
| red | `#f7768e` |
| panel | `#282a3b` |
| element/menu | `#2f3145` |
| subtle border | `#444764` |
| border | `#4b4e6e` |
| active border | `#525578` |

- [x] Replace hard-coded visual colors with semantic Tokyo Night system-theme roles.
- [x] Use luminance, spacing, and weight for hierarchy before adding hue.
- [x] Keep body text at least 4.5:1; target stronger contrast for persistent prose and code.
- [x] Make structural table boundaries and active controls visibly distinct from adjacent surfaces.
- [x] Reserve strong accent colors for focus, status, links, and meaningful semantic markers.
- [x] Use proportional text for long prose and monospace for code, paths, commands, and aligned data.

### Persistent context sidebar

- [ ] Keep session title and share/workspace metadata at the top.
- [x] Show context tokens, percentage used, and cost from the latest assistant usage and model limit.
- [x] Show MCP states: connected, failed, disabled, needs auth, and registration required.
- [x] Show LSP states and the correct empty-state explanation.
- [x] Show the live OpenCode todo list and update on `todo.updated`.
- [x] Show modified files with additions/deletions when available.
- [ ] Show abbreviated directory, VCS branch, and OpenCode version in the footer.

Todo presentation reference: use a fixed marker column with completed `[v]`, active `[.]`, pending
`[ ]`, and cancelled `[-]`; completed and pending text stay muted while the active item uses warning
color. Wrapped lines align with the task text rather than the marker.

The panel remains persistent; sections with no useful content may collapse or hide. It must not replace
the current resizable inspector's selected-part details without a deliberate combined layout.

## Duplicate directory tabs

Decision direction: `Ctrl+T` means new tab, so selecting an already-open directory should create a new
tab. Reusing an existing tab should be a separate explicit action such as `focus existing tab`.

Required architecture before enabling duplicates:

- [ ] Add a stable `TabId`; directory is server scope, not tab identity.
- [ ] Route editor subscriptions, timeline loads, completion, drafts, images, details, and caches by
      `TabId` instead of `find(directory)`.
- [ ] Continue fanning directory-scoped SSE events into every tab for that directory.
- [ ] Share one directory connection/event stream instead of opening one SSE stream per duplicate tab.
- [ ] Decide whether duplicate tabs on the same session share a draft. Default proposal: drafts remain
      keyed by directory/session for restart continuity, while editor mutation always targets a TabId.
- [ ] Mark already-open directory results and offer a modifier/action to focus an existing tab.

Do not implement duplicate tabs by simply removing the current directory deduplication; that would
silently route asynchronous results to the first matching tab.

## Slash-command parity

### App and navigation

- [x] `/sessions`, aliases `/resume`, `/continue`
- [x] `/new`, alias `/clear`
- [x] `/workspaces`, `/move`
- [x] `/help`
- [x] `/exit`, aliases `/quit`, `/q`
- [ ] `/models`, alias `/mo`
- [ ] `/agents`
- [ ] `/mcps`
- [ ] `/variants`
- [ ] `/connect`
- [ ] `/org`, aliases `/orgs`, `/switch-org`
- [ ] `/status`
- [ ] `/debug`
- [ ] `/themes`

### Session actions

- [ ] `/share` and `/unshare`
- [ ] `/rename`
- [ ] `/timeline`
- [ ] `/fork`
- [ ] `/compact`, alias `/summarize`
- [ ] `/undo` and `/redo` for session revert state
- [ ] `/copy`
- [ ] `/export`
- [ ] `/timestamps`, alias `/toggle-timestamps`
- [ ] `/thinking`, alias `/toggle-thinking`

### Prompt and extension actions

- [ ] `/editor` with bundled-editor design tracked separately.
- [ ] `/skills` backed by `app.skills`, including global and MCP/server-provided skills.
- [ ] `/warp` when experimental workspaces are available.
- [ ] Distinguish config, MCP, and skill command sources; render MCP commands with the upstream suffix.
- [ ] Inventory runtime commands from `command.list` rather than hard-coding user/plugin commands.

Provider/model/agent dialogs, editor integration, export, fork/revert, compact, skills, and status panels
are features, not placeholder slash entries. Hide commands only when the server capability is absent.

## Later rich rendering

- [ ] Mermaid-to-SVG pipeline with bounded background rendering and cache invalidation.
- [ ] LaTeX inline/block rendering with accessible plaintext fallback.
- [ ] Diff viewer inspired by dedicated review tools: file tree, unified/split modes, hunk folding,
      syntax highlighting, comments-ready anchors, and keyboard navigation.
- [ ] `<kbd>` and semantic footnote components.

## Verification

- [ ] Same-frame composer growth and shrink tests.
- [ ] Optimistic/server attachment reconciliation test.
- [ ] Tool-part routing regression test using Read output.
- [ ] Theme role contrast tests for every foreground/surface pair used by body text and controls.
- [ ] Sidebar contract tests for context, MCP, LSP, todo, diff, VCS, and version data.
- [ ] Full formatting, source-size check, tests, strict Clippy, release build, manual smoke, CI, and
      automatic review before each completed pass.
