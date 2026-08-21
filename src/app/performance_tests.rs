use std::{
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

use super::*;
use crate::app::PartSelection;

#[gpui::test]
fn direct_interaction_paths_stay_sub_millisecond(cx: &mut TestAppContext) {
    let sessions = (0..1_000)
        .map(|index| {
            session_in(
                &format!("session-{index}"),
                if index % 2 == 0 { "/work/a" } else { "/work/b" },
                index,
            )
        })
        .collect();
    let workspace = workspace(cx, sessions, TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].directory = "/work/a".into();
        workspace.open_directory("/work/b".into(), cx);
        let selection = PartSelection {
            message_id: "message".into(),
            part_id: "part".into(),
        };
        let mut samples = Vec::with_capacity(20_000);
        for iteration in 0..20_000 {
            let started = Instant::now();
            workspace.active_tab = iteration % 2;
            let tab = &mut workspace.tabs[workspace.active_tab];
            if !tab.expanded_parts.remove(&selection) {
                tab.expanded_parts.insert(selection.clone());
            }
            black_box(workspace.directory_session_count(if iteration % 2 == 0 {
                "/work/a"
            } else {
                "/work/b"
            }));
            samples.push(started.elapsed());
        }
        samples.sort_unstable();
        let p99 = samples[samples.len() * 99 / 100];
        eprintln!("direct interaction p99: {p99:?}");
        assert!(
            p99 < Duration::from_millis(1),
            "direct interaction p99 {p99:?} exceeded 1 ms"
        );
    });
}

#[gpui::test]
fn sustained_menu_navigation_stays_sub_millisecond(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.overlay = Overlay::Directory;
        workspace.directory_suggestions = Arc::new(
            (0..10_000)
                .map(|index| format!("/workspace/project-{index}"))
                .collect(),
        );
        let mut samples = Vec::with_capacity(20_000);
        for _ in 0..20_000 {
            let started = Instant::now();
            workspace.move_overlay_selection(1, cx);
            samples.push(started.elapsed());
        }
        samples.sort_unstable();
        let p99 = samples[samples.len() * 99 / 100];
        assert!(
            p99 < Duration::from_millis(1),
            "menu navigation p99 {p99:?} exceeded 1 ms"
        );
    });
}

#[gpui::test]
fn sustained_workspace_switching_stays_sub_millisecond(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].directory = "/work/a".into();
        workspace.open_directory("/work/b".into(), cx);
        for (index, tab) in workspace.tabs.iter_mut().enumerate() {
            tab.timeline = TimelineState::Loading {
                session_id: format!("session-{index}"),
                title: "performance".into(),
            };
        }
        workspace.layout_save = None;
        workspace.focus_editor_on_render = false;
        let mut samples = Vec::with_capacity(10_000);
        for iteration in 0..10_000 {
            let started = Instant::now();
            workspace.switch_directory(iteration % 2, cx);
            samples.push(started.elapsed());
        }
        samples.sort_unstable();
        let p99 = samples[samples.len() * 99 / 100];
        eprintln!("workspace switching p99: {p99:?}");
        assert!(workspace.directory_switch.is_some());
        assert_eq!(workspace.tab_bar.read(cx).active(), workspace.active_tab);
        assert!(workspace.layout_save.is_none());
        assert!(!workspace.focus_editor_on_render);
        assert!(
            p99 < Duration::from_millis(1),
            "workspace switching p99 {p99:?} exceeded 1 ms"
        );
    });
}
