use std::time::Duration;

use gpui::{Context, Timer};

use super::Workspace;

#[derive(Debug, Eq, PartialEq)]
enum InterruptTransition {
    Arm,
    Abort,
}

fn interrupt_transition(armed_session: Option<&str>, session_id: &str) -> InterruptTransition {
    if armed_session == Some(session_id) {
        InterruptTransition::Abort
    } else {
        InterruptTransition::Arm
    }
}

impl Workspace {
    pub(super) fn arm_or_abort(&mut self, session_id: String, cx: &mut Context<Self>) {
        if interrupt_transition(self.interrupt_session.as_deref(), &session_id)
            == InterruptTransition::Abort
        {
            self.clear_interrupt();
            self.abort_session(session_id, cx);
            return;
        }
        self.interrupt_generation = self.interrupt_generation.wrapping_add(1);
        let generation = self.interrupt_generation;
        self.interrupt_session = Some(session_id.clone());
        self.interrupt_reset = Some(cx.spawn(async move |workspace, cx| {
            Timer::after(Duration::from_secs(2)).await;
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.interrupt_generation == generation
                    && workspace.interrupt_session.as_deref() == Some(session_id.as_str())
                {
                    workspace.interrupt_session = None;
                    workspace.interrupt_reset = None;
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    pub(super) fn clear_interrupt(&mut self) {
        self.interrupt_generation = self.interrupt_generation.wrapping_add(1);
        self.interrupt_session = None;
        self.interrupt_reset = None;
    }

    pub(super) fn clear_interrupt_for(&mut self, session_id: &str) {
        if self.interrupt_session.as_deref() == Some(session_id) {
            self.clear_interrupt();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_press_arms_and_same_session_second_press_aborts() {
        assert_eq!(interrupt_transition(None, "a"), InterruptTransition::Arm);
        assert_eq!(
            interrupt_transition(Some("a"), "a"),
            InterruptTransition::Abort
        );
        assert_eq!(
            interrupt_transition(None, "a"),
            InterruptTransition::Arm,
            "an expired arm makes the next press a first press"
        );
    }

    #[test]
    fn a_different_session_never_inherits_the_arm() {
        assert_eq!(
            interrupt_transition(Some("a"), "b"),
            InterruptTransition::Arm
        );
    }
}
