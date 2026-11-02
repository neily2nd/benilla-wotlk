//! Client-independent player-control semantics extracted from Benilla's player pipeline.
//!
//! Callers own bindings and actor lifecycle. This crate maps input into movement intent, advances
//! the rendered body heading, and selects shared animation facts without owning a network session.

use bevy::prelude::{ButtonInput, KeyCode, MouseButton, Quat, Vec3};

mod controller;
pub use controller::{
    Controller, ControllerConfig, ControllerFrame, ControllerInput, FIXED_STEP_SECONDS,
};

impl ControlInput {
    pub fn moving(self) -> bool {
        self.forward != 0.0 || self.strafe != 0.0
    }

    pub fn movement_flags(self) -> u32 {
        use benilla_animation::movement_flags as f;
        let mut flags = 0;
        if self.forward > 0.0 {
            flags |= f::FORWARD;
        }
        if self.forward < 0.0 {
            flags |= f::BACKWARD;
        }
        if self.strafe < 0.0 {
            flags |= f::STRAFE_LEFT;
        }
        if self.strafe > 0.0 {
            flags |= f::STRAFE_RIGHT;
        }
        flags
    }
}

/// The POC's speed policy, scaled by an application's effective forward run speed.
pub fn movement_speed(run_speed: f32, input: ControlInput) -> f32 {
    run_speed.max(0.0)
        * if input.walking {
            2.5 / 7.0
        } else if input.forward < 0.0 {
            4.5 / 7.0
        } else {
            1.0
        }
}

/// Benilla's keyboard yaw step. Mouse look commits absolute camera facing instead.
pub fn keyboard_turn_delta(input: ControlInput, dt: f32, turn_rate: f32, airborne: bool) -> f32 {
    if input.mouse_look || !dt.is_finite() || dt <= 0.0 || !turn_rate.is_finite() {
        return 0.0;
    }
    input.turn
        * turn_rate.max(0.0)
        * dt
        * if input.moving() || airborne {
            0.75
        } else {
            1.0
        }
}

/// Configurable key bindings, separated from the movement semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlBindings {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub strafe_left: KeyCode,
    pub strafe_right: KeyCode,
    pub turn_left: KeyCode,
    pub turn_right: KeyCode,
    pub walk: KeyCode,
    pub walk_alternate: Option<KeyCode>,
    pub jump: KeyCode,
    pub mouse_look: MouseButton,
}

impl Default for ControlBindings {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            strafe_left: KeyCode::KeyA,
            strafe_right: KeyCode::KeyD,
            turn_left: KeyCode::ArrowLeft,
            turn_right: KeyCode::ArrowRight,
            walk: KeyCode::ShiftLeft,
            walk_alternate: Some(KeyCode::ShiftRight),
            jump: KeyCode::Space,
            mouse_look: MouseButton::Right,
        }
    }
}

/// One frame of decoded movement intent. Axes are signed and bounded to `[-1, 1]`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlInput {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub walking: bool,
    pub jumping: bool,
    pub mouse_look: bool,
}

/// Decode keyboard and right-mouse state using caller-provided bindings.
pub fn decode_input(
    keys: &ButtonInput<KeyCode>,
    buttons: &ButtonInput<MouseButton>,
    bindings: ControlBindings,
) -> ControlInput {
    decode_input_with_mouse_look(keys, bindings, buttons.pressed(bindings.mouse_look))
}

/// Decode after the application's cursor/focus/UI policy has accepted or rejected mouse look.
pub fn decode_input_with_mouse_look(
    keys: &ButtonInput<KeyCode>,
    bindings: ControlBindings,
    mouse_look: bool,
) -> ControlInput {
    let axis =
        |positive, negative| f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative));
    ControlInput {
        forward: axis(bindings.forward, bindings.backward),
        strafe: axis(bindings.strafe_right, bindings.strafe_left),
        turn: if mouse_look {
            0.0
        } else {
            axis(bindings.turn_left, bindings.turn_right)
        },
        walking: keys.pressed(bindings.walk)
            || bindings.walk_alternate.is_some_and(|key| keys.pressed(key)),
        jumping: keys.just_pressed(bindings.jump),
        mouse_look,
    }
}

/// A unit-length planar direction in the character's facing frame.
pub fn movement_direction(yaw: f32, input: ControlInput) -> Vec3 {
    let facing = Quat::from_rotation_y(yaw);
    let forward = (facing * Vec3::NEG_Z).with_y(0.0).normalize_or_zero();
    let right = (facing * Vec3::X).with_y(0.0).normalize_or_zero();
    (forward * input.forward + right * input.strafe).normalize_or_zero()
}

/// Per-frame character and rendered-body yaw state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Heading {
    pub facing: f32,
    pub body: f32,
}

/// Apply Benilla's shared mouse-look yaw/pitch deltas and return the yaw delta.
///
/// The app still owns cursor lock, button latching, focus changes, and camera-pivot pitch routing.
pub fn mouse_look_deltas(
    raw_motion: Vec3,
    yaw_rate: f32,
    pitch_rate: f32,
    invert_pitch: bool,
) -> (f32, f32) {
    let yaw_delta = -raw_motion.x * yaw_rate;
    let dy = if invert_pitch {
        -raw_motion.y
    } else {
        raw_motion.y
    };
    let pitch_delta = -dy * pitch_rate;
    (yaw_delta, pitch_delta)
}

/// Advance keyboard turn and rendered strafe/body orientation using Benilla's gait rules.
///
/// `turn_rate` is the mover's own angular rate; translating turns at 75% of that rate. During
/// strafe, body yaw eases toward facing with the authored lateral offset; moving forward aligns the
/// body immediately; at rest, the body closes the remaining facing gap after a 90-degree ceiling.
pub fn advance_heading(
    heading: &mut Heading,
    input: ControlInput,
    dt: f32,
    turn_rate: f32,
    stationary_chase_rate: f32,
    move_flags: u32,
    swimming: bool,
    moving: bool,
    airborne: bool,
    steering: bool,
) -> u32 {
    if !dt.is_finite() || dt <= 0.0 || !turn_rate.is_finite() {
        return move_flags;
    }

    let turn_step = keyboard_turn_delta(
        ControlInput {
            forward: if moving { 1.0 } else { 0.0 },
            ..input
        },
        dt,
        turn_rate,
        airborne,
    );
    // Benilla stores the accumulated facing unwrapped; only comparisons and rendered body yaw fold.
    heading.facing += turn_step;

    let offset = strafe_body_offset(move_flags);

    let body_step = if swimming {
        heading.body = heading.facing;
        0.0
    } else if offset != 0.0 {
        let current_offset = wrap_pi(heading.body - heading.facing);
        let eased_offset = current_offset + (offset - current_offset) * (1.0 - (-17.26 * dt).exp());
        heading.body = wrap_pi(heading.facing + eased_offset);
        0.0
    } else if moving || airborne {
        heading.body = heading.facing;
        0.0
    } else {
        let delta = wrap_pi(heading.facing - heading.body);
        let mut magnitude = (delta.abs() - std::f32::consts::FRAC_PI_2).max(0.0);
        if !steering {
            magnitude += dt * turn_rate.max(0.0) * stationary_chase_rate.max(0.0);
        }
        let step = magnitude.min(delta.abs()).copysign(delta);
        heading.body = wrap_pi(heading.body + step);
        step
    };

    let mut flags = move_flags;
    if !swimming && !moving && !airborne {
        flags &= !(benilla_animation::movement_flags::TURN_LEFT
            | benilla_animation::movement_flags::TURN_RIGHT);
        if body_step > 1.0e-5 {
            flags |= benilla_animation::movement_flags::TURN_LEFT;
        } else if body_step < -1.0e-5 {
            flags |= benilla_animation::movement_flags::TURN_RIGHT;
        }
    }
    flags
}

fn wrap_pi(mut angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    angle = (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    angle
}

fn strafe_body_offset(flags: u32) -> f32 {
    use benilla_animation::movement_flags as f;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
    let left = flags & f::STRAFE_LEFT != 0;
    let right = flags & f::STRAFE_RIGHT != 0;
    if left == right {
        return 0.0;
    }
    let diagonal = flags & (f::FORWARD | f::BACKWARD) != 0;
    let magnitude = if diagonal { FRAC_PI_4 } else { FRAC_PI_2 };
    let back = flags & f::BACKWARD != 0;
    if left != back {
        magnitude
    } else {
        -magnitude
    }
}

/// Build the animation facts Benilla's shared animation crate uses for locomotion resolution.
pub fn locomotion_facts(
    speed: f32,
    flags: u32,
    walk_speed: f32,
) -> benilla_animation::LocomotionFacts {
    benilla_animation::LocomotionFacts {
        speed,
        flags,
        walk_speed,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opposite_inputs_cancel_and_rejected_mouse_capture_does_not_suppress_arrow_turn() {
        let mut keys = ButtonInput::default();
        for key in [
            KeyCode::KeyW,
            KeyCode::KeyS,
            KeyCode::KeyA,
            KeyCode::KeyD,
            KeyCode::ArrowLeft,
        ] {
            keys.press(key);
        }
        let input = decode_input_with_mouse_look(&keys, Default::default(), false);
        assert!(!input.moving());
        assert_eq!(input.turn, 1.0);
        assert_eq!(input.movement_flags(), 0);
        assert_eq!(
            decode_input_with_mouse_look(&keys, Default::default(), true).turn,
            0.0
        );
    }

    #[test]
    fn walk_run_and_backward_speed_share_the_proven_poc_policy() {
        assert_eq!(movement_speed(7.0, ControlInput::default()), 7.0);
        assert_eq!(
            movement_speed(
                7.0,
                ControlInput {
                    forward: -1.0,
                    ..Default::default()
                }
            ),
            4.5
        );
        for forward in [-1.0, 0.0, 1.0] {
            assert_eq!(
                movement_speed(
                    7.0,
                    ControlInput {
                        forward,
                        walking: true,
                        ..Default::default()
                    }
                ),
                2.5
            );
        }
    }

    #[test]
    fn requested_default_bindings_strafe_and_turn_independently() {
        let mut keys = ButtonInput::default();
        let buttons = ButtonInput::default();
        keys.press(KeyCode::KeyA);
        keys.press(KeyCode::KeyW);
        let input = decode_input(&keys, &buttons, ControlBindings::default());
        assert_eq!(input.forward, 1.0);
        assert_eq!(input.strafe, -1.0);
        assert_eq!(input.turn, 0.0);
        assert!((movement_direction(0.0, input).length() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn right_mouse_turns_facing_by_mouse_look_policy_not_keyboard_turn() {
        let mut keys = ButtonInput::default();
        let mut buttons = ButtonInput::default();
        keys.press(KeyCode::ArrowLeft);
        buttons.press(MouseButton::Right);
        let input = decode_input(&keys, &buttons, ControlBindings::default());
        assert!(input.mouse_look);
        assert_eq!(input.turn, 0.0);
    }

    #[test]
    fn walking_jump_and_custom_bindings_are_independent() {
        let mut keys = ButtonInput::default();
        let mut buttons = ButtonInput::default();
        keys.press(KeyCode::KeyI);
        keys.press(KeyCode::KeyJ);
        keys.press(KeyCode::ShiftRight);
        keys.press(KeyCode::Space);
        buttons.press(MouseButton::Middle);
        let bindings = ControlBindings {
            forward: KeyCode::KeyI,
            strafe_left: KeyCode::KeyJ,
            walk: KeyCode::ShiftRight,
            mouse_look: MouseButton::Middle,
            ..Default::default()
        };
        let input = decode_input(&keys, &buttons, bindings);
        assert!(input.walking);
        assert!(input.jumping);
        assert!(input.mouse_look);
        assert_eq!(input.forward, 1.0);
        assert_eq!(input.strafe, -1.0);
    }

    #[test]
    fn either_shift_key_activates_the_default_walk_mode() {
        let mut keys = ButtonInput::default();
        let buttons = ButtonInput::default();
        keys.press(KeyCode::ShiftRight);
        assert!(decode_input(&keys, &buttons, ControlBindings::default()).walking);
    }

    #[test]
    fn heading_turns_body_and_flags_strafe() {
        let mut heading = Heading::default();
        let input = ControlInput {
            turn: 1.0,
            ..Default::default()
        };
        let flags = advance_heading(
            &mut heading,
            input,
            0.1,
            3.0,
            8.0,
            0,
            false,
            false,
            false,
            false,
        );
        assert!(heading.facing > 0.0);
        assert_ne!(flags & benilla_animation::movement_flags::TURN_LEFT, 0);
    }

    #[test]
    fn diagonal_strafe_uses_quarter_turn_body_offset() {
        let mut heading = Heading::default();
        advance_heading(
            &mut heading,
            ControlInput::default(),
            1.0 / 60.0,
            3.0,
            8.0,
            benilla_animation::movement_flags::STRAFE_LEFT
                | benilla_animation::movement_flags::FORWARD,
            false,
            true,
            false,
            true,
        );
        assert!(heading.body > 0.0 && heading.body < std::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn mouse_look_rotates_camera_and_clamps_pitch() {
        let (delta, pitch_delta) = mouse_look_deltas(Vec3::new(2.0, 200.0, 0.0), 0.01, 0.02, false);
        assert!((delta + 0.02).abs() < 1.0e-6);
        let yaw = wrap_pi(delta);
        let pitch = pitch_delta.clamp(-1.2, 1.2);
        assert!((yaw + 0.02).abs() < 1.0e-6);
        assert_eq!(pitch, -1.2);
    }

    #[test]
    fn camera_yaw_and_character_forward_share_one_heading() {
        let (yaw_delta, _) = mouse_look_deltas(Vec3::new(-100.0, 0.0, 0.0), 0.01, 0.01, false);
        let facing = wrap_pi(yaw_delta);
        let forward = movement_direction(
            facing,
            ControlInput {
                forward: 1.0,
                ..Default::default()
            },
        );
        assert!(
            forward.x < -0.8,
            "mouse yaw and actor forward must agree: {forward:?}"
        );
    }

    #[test]
    fn locomotion_facts_select_the_existing_walk_run_and_idle_clips() {
        use benilla_animation::movement_flags::FORWARD;
        let walk =
            benilla_animation::gait_candidates(locomotion_facts(2.5, FORWARD, 2.5), None, None);
        let run =
            benilla_animation::gait_candidates(locomotion_facts(7.0, FORWARD, 2.5), None, None);
        assert_eq!(walk, &[4, 0]);
        assert_eq!(run, &[5, 4, 0]);
        assert_eq!(
            benilla_animation::gait_candidates(Default::default(), None, None),
            &[0]
        );
    }
}
