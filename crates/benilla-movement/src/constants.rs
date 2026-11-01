//! Authored movement constants shared with Benilla's application and movement integration.

pub const CAPSULE_RADIUS: f32 = 1.0 / 3.0;
pub const CAPSULE_HEIGHT: f32 = 2.027_777_7;
pub const GRAVITY: f32 = 19.291_105;
pub const JUMP_SPEED: f32 = 7.955_547;
pub const TERMINAL_VELOCITY: f32 = 60.148_003;
pub const FEATHER_TERMINAL_VELOCITY: f32 = 7.0;
pub const HOVER_HEIGHT: f32 = 1.0;
pub const HOVER_CLIMB_RATE: f32 = 7.0;
pub const GROUND_COS: f32 = 0.642_788;
pub const GROUND_PROBE: f32 = 0.2;
pub const STEP_SLOPE_RATIO: f32 = 1.849_399;
pub const STEP_SNAP_SLACK: f32 = 0.027_777_8;
pub const STEP_UP_HEIGHT: f32 = 1.0;
pub const STEP_UP_ADVANCE: f32 = 1.191_753_6;
pub const STEP_UP_ADVANCE_PER_YARD: f32 = STEP_UP_ADVANCE / STEP_UP_HEIGHT;
pub const FOOT_CONE_HEIGHT: f32 = CAPSULE_RADIUS * STEP_SLOPE_RATIO;
pub const LAND_PROBE: f32 = 0.05;
pub const WEDGE_STILL_FRAMES: u8 = 3;
pub const WEDGE_STALL_RATIO: f32 = 0.15;
pub const WEDGE_MIN_FALL: f32 = 1.0;
pub const AIR_NUDGE_SPEED: f32 = 2.5;
pub const FALL_FAR_DROP: f32 = 0.111_11;
pub const FALL_FAR_TIME: f32 = 0.5;
pub const SKIN_WIDTH: f32 = 0.02;
pub const WATER_WALK_PITCH_FLOOR: f32 = -0.645_771_8;
