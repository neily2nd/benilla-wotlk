//! Reusable collision-backed movement extracted from Benilla's player mover.
//!
//! This crate owns kinematic state and frame integration only. Applications supply input,
//! capsule geometry and the already-built collision world; they retain authority over networking,
//! cameras, rendering and application lifecycle.

mod constants;
mod mover;

pub use mover::{
    advance, airborne_step, buried_floor_lift, fall_step, grounded_step, step_up, water_floor,
    GroundedStep, StepAttempt, StepVerdict, Support,
};
pub use state::{
    CollisionConfig, MovementDiagnostics, MovementInput, MovementModes, MovementOutcome,
    MovementState, StepProbeRequest, TraceFrame,
};

mod state;

pub use constants::{
    AIR_NUDGE_SPEED, CAPSULE_HEIGHT, CAPSULE_RADIUS, FALL_FAR_DROP, FALL_FAR_TIME,
    FEATHER_TERMINAL_VELOCITY, FOOT_CONE_HEIGHT, GRAVITY, GROUND_COS, GROUND_PROBE,
    HOVER_CLIMB_RATE, HOVER_HEIGHT, JUMP_SPEED, LAND_PROBE, SKIN_WIDTH, STEP_SLOPE_RATIO,
    STEP_SNAP_SLACK, STEP_UP_ADVANCE, STEP_UP_ADVANCE_PER_YARD, STEP_UP_HEIGHT, TERMINAL_VELOCITY,
    WATER_WALK_PITCH_FLOOR, WEDGE_MIN_FALL, WEDGE_STALL_RATIO, WEDGE_STILL_FRAMES,
};

pub const DEFAULT_CAPSULE_RADIUS: f32 = CAPSULE_RADIUS;
pub const DEFAULT_CAPSULE_HEIGHT: f32 = CAPSULE_HEIGHT;
pub const DEFAULT_SKIN_WIDTH: f32 = SKIN_WIDTH;
