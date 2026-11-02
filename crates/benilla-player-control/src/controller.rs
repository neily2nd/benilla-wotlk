//! Authoritative offline root motion. The Benilla mover remains the only physical integrator.
//! Body-pose offsets are deliberately separate from root facing; strafing never steers the root.

use std::time::Duration;

use benilla_animation::{
    movement_flags as flags, LocomotionFacts, LocomotionSnapshot, VerticalMotion,
};
use benilla_movement::{CollisionConfig, MovementInput, MovementOutcome, MovementState};
use benilla_world::collision::WorldCollision;
use bevy::prelude::Vec3;

use crate::{keyboard_turn_delta, movement_direction, movement_speed, ControlInput};

pub const FIXED_STEP_SECONDS: f32 = 1.0 / 60.0;
const MAX_FRAME_DELTA: f32 = 0.25;

#[derive(Clone, Copy, Debug)]
pub struct ControllerConfig {
    /// Effective forward run speed after application-owned modifiers.
    pub run_speed: f32,
    pub turn_rate: f32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            run_speed: 7.0,
            turn_rate: std::f32::consts::PI,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ControllerInput {
    pub control: ControlInput,
    /// Absolute camera yaw on right drag, including its first frame with zero mouse motion.
    pub camera_facing: Option<f32>,
    /// The application's focus, UI, life and pause gates, already resolved.
    pub enabled: bool,
    pub collision_ready: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ControllerFrame {
    pub steps: usize,
    pub jumped: bool,
    pub landed: bool,
    /// Keyboard rotation only; the camera can consume this once after root publication.
    pub keyboard_yaw: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Controller {
    state: MovementState,
    facing: f32,
    accumulator: f32,
    elapsed: f32,
    queued_jump: bool,
    airborne: bool,
    locomotion: LocomotionSnapshot,
}

impl Controller {
    pub fn at(position: Vec3, facing: f32) -> Self {
        Self {
            state: MovementState {
                pos: position,
                ..Default::default()
            },
            facing,
            ..Default::default()
        }
    }

    pub fn reset(&mut self, position: Vec3, facing: f32) {
        *self = Self::at(position, facing);
    }
    pub fn position(&self) -> Vec3 {
        self.state.pos
    }
    pub fn facing(&self) -> f32 {
        self.facing
    }
    pub fn vertical_velocity(&self) -> f32 {
        self.state.vel_y
    }
    pub fn airborne(&self) -> bool {
        self.airborne
    }
    pub fn locomotion(&self) -> LocomotionFacts {
        self.locomotion.facts()
    }
    pub fn vertical_motion(&self) -> VerticalMotion {
        self.locomotion.vertical()
    }

    /// Drop latent input and catch-up time during a modal, focus loss or streaming hold.
    /// Position and the current physical arc are retained for safe continuation.
    pub fn suspend(&mut self) {
        self.accumulator = 0.0;
        self.queued_jump = false;
        self.locomotion.render_update(true, 0);
    }

    pub fn advance(
        &mut self,
        frame_delta: f32,
        mut input: ControllerInput,
        config: ControllerConfig,
        world: Option<&WorldCollision<'_, '_>>,
        collision: CollisionConfig<'_>,
    ) -> ControllerFrame {
        input.collision_ready &= world.is_some();
        self.advance_with(frame_delta, input, config, |state, dt, elapsed, intent| {
            benilla_movement::advance(
                state,
                dt,
                elapsed,
                world.expect("collision readiness checked"),
                collision,
                intent,
            )
        })
    }

    fn advance_with(
        &mut self,
        frame_delta: f32,
        input: ControllerInput,
        config: ControllerConfig,
        mut step: impl FnMut(&mut MovementState, Duration, f32, MovementInput) -> MovementOutcome,
    ) -> ControllerFrame {
        let mut frame = ControllerFrame::default();
        if !input.enabled
            || !input.collision_ready
            || !frame_delta.is_finite()
            || frame_delta <= 0.0
        {
            self.suspend();
            return frame;
        }
        let control = input.control;
        let previous_facing = self.facing;
        if control.mouse_look {
            if let Some(yaw) = input.camera_facing.filter(|yaw| yaw.is_finite()) {
                self.facing = yaw;
            }
        }
        self.queued_jump |= control.jumping;
        self.accumulator += frame_delta.min(MAX_FRAME_DELTA);
        let steps = ((self.accumulator + FIXED_STEP_SECONDS * 1.0e-4) / FIXED_STEP_SECONDS).floor()
            as usize;
        self.accumulator = (self.accumulator - steps as f32 * FIXED_STEP_SECONDS).max(0.0);
        let speed = movement_speed(config.run_speed, control);
        let walk_speed = config.run_speed * (2.5 / 7.0);
        for _ in 0..steps {
            let yaw =
                keyboard_turn_delta(control, FIXED_STEP_SECONDS, config.turn_rate, self.airborne);
            self.facing += yaw;
            frame.keyboard_yaw += yaw;
            let before = self.position();
            let was_airborne = self.airborne;
            self.elapsed += FIXED_STEP_SECONDS;
            let outcome = step(
                &mut self.state,
                Duration::from_secs_f32(FIXED_STEP_SECONDS),
                self.elapsed,
                MovementInput {
                    moving: control.moving(),
                    direction: movement_direction(self.facing, control),
                    speed,
                    want_jump: std::mem::take(&mut self.queued_jump),
                    air_nudge_speed: walk_speed,
                    ..Default::default()
                },
            );
            self.airborne = !outcome.grounded || outcome.jumped;
            if self.airborne {
                self.state.airborne_since.get_or_insert(self.elapsed);
            } else {
                self.state.airborne_since = None;
            }
            let distance = (self.position() - before).with_y(0.0).length();
            let accepted = distance * distance > 1.0e-5;
            self.locomotion.resolve_step(
                LocomotionFacts {
                    speed: distance / FIXED_STEP_SECONDS,
                    flags: control.movement_flags(),
                    walk_speed,
                    ..Default::default()
                },
                accepted,
            );
            frame.jumped |= outcome.jumped;
            frame.landed |= was_airborne && !self.airborne;
            frame.steps += 1;
        }
        let yaw_delta = self.facing - previous_facing;
        let turn_flags = if self.airborne {
            0
        } else if yaw_delta > 1.0e-5 {
            flags::TURN_LEFT
        } else if yaw_delta < -1.0e-5 {
            flags::TURN_RIGHT
        } else {
            0
        };
        self.locomotion.render_update(!control.moving(), turn_flags);
        self.locomotion.set_vertical(if frame.landed {
            VerticalMotion::Landing
        } else if self.airborne {
            if self.state.vel_y > 0.0 {
                VerticalMotion::Jumping
            } else {
                VerticalMotion::Falling
            }
        } else {
            VerticalMotion::Grounded
        });
        frame
    }
}

#[cfg(test)]
mod tests;
