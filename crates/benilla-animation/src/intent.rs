use crate::LocomotionFacts;

/// Vertical phase reported by the movement owner, not inferred from animation playback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalMotion {
    #[default]
    Grounded,
    Jumping,
    Falling,
    Landing,
}

/// Persistent handoff from collision-resolved movement to model animation selection.
///
/// Applications call `resolve_step` only after an actual movement step. Render updates with no
/// movement step retain the last resolved facts. Releasing movement input returns the ground
/// facts to idle immediately, while an attempted but blocked step is explicitly marked blocked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LocomotionSnapshot {
    facts: LocomotionFacts,
    vertical: VerticalMotion,
}

impl LocomotionSnapshot {
    pub fn facts(&self) -> LocomotionFacts {
        self.facts
    }

    pub fn vertical(&self) -> VerticalMotion {
        self.vertical
    }

    /// Record the result of one real, collision-resolved movement step.
    pub fn resolve_step(&mut self, facts: LocomotionFacts, accepted_displacement: bool) {
        self.facts = facts;
        self.facts.physically_blocked = !accepted_displacement && facts.flags != 0;
        if self.facts.physically_blocked {
            self.facts.flags = 0;
            self.facts.speed = 0.0;
        }
    }

    /// Apply immediate render-frame release/turn signals without inventing a movement result.
    pub fn render_update(&mut self, movement_released: bool, turn_flags: u32) {
        if movement_released {
            self.facts.flags = turn_flags;
            self.facts.speed = 0.0;
            self.facts.physically_blocked = false;
        }
    }

    pub fn set_vertical(&mut self, vertical: VerticalMotion) {
        self.vertical = vertical;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement_flags::{BACKWARD, FORWARD, TURN_LEFT};

    fn facts(flags: u32, speed: f32) -> LocomotionFacts {
        LocomotionFacts {
            flags,
            speed,
            walk_speed: 2.5,
            ..Default::default()
        }
    }

    #[test]
    fn zero_step_render_updates_preserve_the_last_resolved_locomotion() {
        let mut snapshot = LocomotionSnapshot::default();
        snapshot.resolve_step(facts(FORWARD, 7.5), true);
        snapshot.render_update(false, 0);
        assert_eq!(snapshot.facts(), facts(FORWARD, 7.5));
    }

    #[test]
    fn input_release_returns_to_idle_without_waiting_for_a_physics_step() {
        let mut snapshot = LocomotionSnapshot::default();
        snapshot.resolve_step(facts(FORWARD, 7.5), true);
        snapshot.render_update(true, 0);
        assert_eq!(snapshot.facts().flags, 0);
        assert_eq!(snapshot.facts().speed, 0.0);
    }

    #[test]
    fn blocked_steps_do_not_claim_running_and_backward_motion_is_retained() {
        let mut snapshot = LocomotionSnapshot::default();
        snapshot.resolve_step(facts(FORWARD, 7.5), false);
        assert!(snapshot.facts().physically_blocked);
        assert_eq!(snapshot.facts().flags, 0);
        snapshot.resolve_step(facts(BACKWARD, 4.5), true);
        snapshot.render_update(false, 0);
        assert_eq!(snapshot.facts().flags, BACKWARD);
    }

    #[test]
    fn stationary_turn_is_immediate_and_run_reset_clears_vertical_state() {
        let mut snapshot = LocomotionSnapshot::default();
        snapshot.resolve_step(facts(FORWARD, 7.5), true);
        snapshot.render_update(true, TURN_LEFT);
        assert_eq!(snapshot.facts().flags, TURN_LEFT);
        snapshot.set_vertical(VerticalMotion::Jumping);
        snapshot.reset();
        assert_eq!(snapshot.vertical(), VerticalMotion::Grounded);
        assert_eq!(snapshot.facts().flags, 0);
    }
}
