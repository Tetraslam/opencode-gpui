use gpui::{FontWeight, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    api::{McpStatus, SidebarSnapshot, Todo},
    model::Message,
    theme::{MONO_FONT, color},
};

use super::{ServerState, Workspace, sidebar_state::SidebarState};

impl Workspace {
    pub(super) fn render_session_context(&self) -> gpui::AnyElement {
        let Some(tab) = self.active_tab() else {
            return div().into_any_element();
        };
        let title = tab.timeline.title().unwrap_or_else(|| "session".into());
        let context = context_summary(tab);
        let sidebar_body = match &tab.sidebar {
            SidebarState::Ready(snapshot) => render_snapshot(snapshot, context.as_ref()),
            SidebarState::Loading => muted("loading context..."),
            SidebarState::Failed(error) => muted(error),
            SidebarState::Empty => muted("context unavailable"),
        };
        let version = tab
            .timeline
            .session_id()
            .and_then(|session_id| self.session(session_id))
            .map_or("unknown", |session| session.version.as_str());
        div()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(rgb(color::BORDER))
            .bg(rgb(color::SURFACE))
            .font_family(MONO_FONT)
            .text_sm()
            .child(
                div()
                    .mb_3()
                    .whitespace_normal()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(color::TEXT_BRIGHT))
                    .child(title),
            )
            .child(sidebar_body)
            .child(
                div()
                    .mt_4()
                    .pt_2()
                    .border_t_1()
                    .border_color(rgb(color::BORDER_SUBTLE))
                    .text_xs()
                    .text_color(rgb(color::TEXT_MUTED))
                    .child(format!("{}\nopencode {version}", tab.directory)),
            )
            .into_any_element()
    }

    fn session(&self, session_id: &str) -> Option<&opencode_gpui::model::Session> {
        let ServerState::Ready { sessions } = &self.server_state else {
            return None;
        };
        sessions.iter().find(|session| session.id == session_id)
    }
}

#[derive(Clone)]
struct ContextSummary {
    tokens: u64,
    cost: f64,
    model: String,
}

fn context_summary(tab: &super::tabs::DirectoryTab) -> Option<ContextSummary> {
    let super::TimelineState::Ready { messages, .. } = &tab.timeline else {
        return None;
    };
    messages
        .iter()
        .rev()
        .find_map(|message| match &message.info {
            Message::Assistant(message) if message.tokens.output > 0 => Some(ContextSummary {
                tokens: message.tokens.used(),
                cost: message.cost,
                model: format!("{}/{}", message.provider_id, message.model_id),
            }),
            Message::User(_) | Message::Assistant(_) => None,
        })
}

fn render_snapshot(
    snapshot: &SidebarSnapshot,
    context: Option<&ContextSummary>,
) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(render_context(snapshot, context))
        .child(render_mcp(snapshot))
        .child(render_lsp(snapshot))
        .children((!snapshot.todos.is_empty()).then(|| render_todos(&snapshot.todos)))
        .children((!snapshot.files.is_empty()).then(|| render_files(snapshot)))
        .into_any_element()
}

fn render_context(
    snapshot: &SidebarSnapshot,
    context: Option<&ContextSummary>,
) -> gpui::AnyElement {
    let tokens = context.map_or(0, |context| context.tokens);
    let cost = context.map_or(0.0, |context| context.cost);
    let percent = context
        .and_then(|context| snapshot.context_limits.get(&context.model))
        .filter(|limit| **limit > 0)
        .map(|limit| tokens.saturating_mul(100) / limit);
    section("context")
        .child(
            div()
                .text_color(rgb(color::TEXT_DIM))
                .child(format!("{} tokens", format_number(tokens)))
                .children(percent.map(|percent| div().child(format!("{percent}% used"))))
                .child(format!("${cost:.2} spent")),
        )
        .into_any_element()
}

fn render_mcp(snapshot: &SidebarSnapshot) -> gpui::AnyElement {
    let mut servers = snapshot.mcp.iter().collect::<Vec<_>>();
    servers.sort_unstable_by_key(|(name, _)| *name);
    section("mcp")
        .children(servers.into_iter().map(|(name, status)| {
            let (label, status_color) = match status {
                McpStatus::Connected => ("connected", color::GREEN),
                McpStatus::Disabled => ("disabled", color::TEXT_MUTED),
                McpStatus::Failed { .. } => ("failed", color::RED),
                McpStatus::NeedsAuth => ("needs auth", color::YELLOW),
                McpStatus::NeedsClientRegistration { .. } => ("needs registration", color::RED),
            };
            status_row(name, label, status_color)
        }))
        .into_any_element()
}

fn render_lsp(snapshot: &SidebarSnapshot) -> gpui::AnyElement {
    section("lsp")
        .children(snapshot.lsp.is_empty().then(|| muted("lsps are disabled")))
        .children(snapshot.lsp.iter().map(|lsp| {
            status_row(
                &lsp.name,
                &lsp.status,
                if lsp.status == "connected" {
                    color::GREEN
                } else {
                    color::RED
                },
            )
        }))
        .into_any_element()
}

fn render_todos(todos: &[Todo]) -> gpui::AnyElement {
    section("todo")
        .children(todos.iter().map(|todo| {
            let (marker, item_color) = match todo.status.as_str() {
                "completed" => ("[v]", color::TEXT_MUTED),
                "in_progress" => ("[.]", color::YELLOW),
                "cancelled" => ("[-]", color::TEXT_MUTED),
                _ => ("[ ]", color::TEXT_DIM),
            };
            div()
                .flex()
                .items_start()
                .gap_2()
                .text_color(rgb(item_color))
                .child(div().w(px(28.0)).flex_none().child(marker))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .whitespace_normal()
                        .child(todo.content.clone()),
                )
        }))
        .into_any_element()
}

fn render_files(snapshot: &SidebarSnapshot) -> gpui::AnyElement {
    section("modified files")
        .children(snapshot.files.iter().map(|file| {
            div()
                .flex()
                .gap_2()
                .child(div().min_w_0().flex_1().truncate().child(file.file.clone()))
                .child(
                    div()
                        .flex_none()
                        .child(format!("+{} -{}", file.additions, file.deletions)),
                )
        }))
        .into_any_element()
}

fn section(title: &'static str) -> gpui::Div {
    div().flex().flex_col().gap_1().child(
        div()
            .mb_1()
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(color::TEXT))
            .child(title),
    )
}

fn status_row(name: &str, status: &str, status_color: u32) -> gpui::AnyElement {
    div()
        .flex()
        .gap_2()
        .text_color(rgb(color::TEXT_DIM))
        .child(div().text_color(rgb(status_color)).child("."))
        .child(SharedString::from(name.to_owned()))
        .child(SharedString::from(status.to_owned()))
        .into_any_element()
}

fn muted(text: impl Into<SharedString>) -> gpui::AnyElement {
    div()
        .text_color(rgb(color::TEXT_MUTED))
        .child(text.into())
        .into_any_element()
}

fn format_number(number: u64) -> String {
    let digits = number.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(character);
    }
    output
}
