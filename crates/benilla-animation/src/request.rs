/// Application-neutral priority for a transient animation action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionPriority {
    Locomotion,
    Combat,
    Reaction,
    Death,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptionPolicy {
    /// Accept a new request only when it outranks the current action, or that action completed.
    HigherPriority,
    /// Accept equal or higher priority actions; useful for repeated attacks.
    EqualOrHigher,
    /// Replace the current action regardless of priority.
    Always,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackLayer {
    UpperBody,
    FullBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionRequest {
    pub animation_id: u16,
    pub priority: ActionPriority,
    pub interruption: InterruptionPolicy,
    pub layer: PlaybackLayer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDecision {
    Start,
    Replace,
    KeepCurrent,
}

/// Small state owned by an application adapter; it tracks only the active action identity/priority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationActionState {
    active: Option<(u16, ActionPriority)>,
}

impl AnimationActionState {
    pub fn active_animation(&self) -> Option<u16> {
        self.active.map(|(id, _)| id)
    }
    pub fn priority(&self) -> Option<ActionPriority> {
        self.active.map(|(_, priority)| priority)
    }

    pub fn request(&mut self, request: ActionRequest, current_finished: bool) -> ActionDecision {
        let Some((_, priority)) = self.active else {
            self.active = Some((request.animation_id, request.priority));
            return ActionDecision::Start;
        };
        if current_finished {
            self.active = Some((request.animation_id, request.priority));
            return ActionDecision::Replace;
        }
        let accepted = match request.interruption {
            InterruptionPolicy::HigherPriority => request.priority > priority,
            InterruptionPolicy::EqualOrHigher => request.priority >= priority,
            InterruptionPolicy::Always => true,
        };
        if accepted {
            self.active = Some((request.animation_id, request.priority));
            ActionDecision::Replace
        } else {
            ActionDecision::KeepCurrent
        }
    }

    pub fn complete(&mut self, animation_id: u16) -> bool {
        if self.active_animation() == Some(animation_id) {
            self.active = None;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interruption_policy_preserves_higher_priority_actions() {
        let mut state = AnimationActionState::default();
        assert_eq!(
            state.request(
                ActionRequest {
                    animation_id: 17,
                    priority: ActionPriority::Combat,
                    interruption: InterruptionPolicy::HigherPriority,
                    layer: PlaybackLayer::UpperBody
                },
                false
            ),
            ActionDecision::Start
        );
        assert_eq!(
            state.request(
                ActionRequest {
                    animation_id: 18,
                    priority: ActionPriority::Locomotion,
                    interruption: InterruptionPolicy::HigherPriority,
                    layer: PlaybackLayer::FullBody
                },
                false
            ),
            ActionDecision::KeepCurrent
        );
        assert_eq!(
            state.request(
                ActionRequest {
                    animation_id: 121,
                    priority: ActionPriority::Death,
                    interruption: InterruptionPolicy::HigherPriority,
                    layer: PlaybackLayer::FullBody
                },
                false
            ),
            ActionDecision::Replace
        );
        assert!(state.complete(121));
        assert_eq!(state.active_animation(), None);
    }

    #[test]
    fn same_priority_replacement_is_explicit_and_reset_is_run_safe() {
        let mut state = AnimationActionState::default();
        state.request(
            ActionRequest {
                animation_id: 17,
                priority: ActionPriority::Combat,
                interruption: InterruptionPolicy::EqualOrHigher,
                layer: PlaybackLayer::UpperBody,
            },
            false,
        );
        assert_eq!(
            state.request(
                ActionRequest {
                    animation_id: 18,
                    priority: ActionPriority::Combat,
                    interruption: InterruptionPolicy::EqualOrHigher,
                    layer: PlaybackLayer::UpperBody
                },
                false
            ),
            ActionDecision::Replace
        );
        state.reset();
        assert_eq!(state.priority(), None);
    }
}
