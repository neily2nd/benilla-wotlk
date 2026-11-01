use avian3d::prelude::Collider;
use bevy::prelude::{Entity, Vec3};

/// Persistent kinematic fields required by the upstream mover across frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementState {
    pub pos: Vec3,
    pub vel_y: f32,
    pub horiz_vel: Vec3,
    pub airborne_since: Option<f32>,
    pub wedged: bool,
    pub wedge_still: u8,
    pub steep_support: bool,
    pub step_start: Option<f32>,
    pub launch_vz: f32,
    pub arc_dirs_set: bool,
    pub knock_arc: bool,
    pub airborne_prev: bool,
    pub settling: bool,
    pub settle_lift: bool,
    pub modes: MovementModes,
}

/// Movement modes that change the upstream kinematic integration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MovementModes {
    pub rooted: bool,
    pub water_walking: bool,
    pub feather_fall: bool,
    pub hover: bool,
    pub levitating: bool,
}

/// Per-frame application input consumed by the movement integrator.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementInput {
    pub moving: bool,
    pub direction: Vec3,
    pub speed: f32,
    pub want_jump: bool,
    pub wire_jump: bool,
    pub knockback_velocity: Option<Vec3>,
    pub water_floor: Option<f32>,
    pub air_nudge_speed: f32,
}

/// Collision shape and dimensions. The collider is prepared by the caller and borrowed for the
/// duration of the movement step; nothing is reconstructed in the frame path.
#[derive(Clone, Copy)]
pub struct CollisionConfig<'a> {
    pub collider: &'a Collider,
    pub capsule_radius: f32,
    pub capsule_height: f32,
    pub skin_width: f32,
}

impl<'a> CollisionConfig<'a> {
    /// Upstream Benilla's default player capsule configuration.
    pub const fn upstream(collider: &'a Collider) -> Self {
        Self {
            collider,
            capsule_radius: crate::CAPSULE_RADIUS,
            capsule_height: crate::CAPSULE_HEIGHT,
            skin_width: crate::SKIN_WIDTH,
        }
    }
}

/// Application-visible result of one movement integration step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementOutcome {
    pub held: bool,
    pub grounded: bool,
    pub jumped: bool,
    pub knocked: bool,
    pub air_nudged: bool,
    pub ground: Option<Entity>,
    pub diagnostics: MovementDiagnostics,
}

/// Bounded diagnostic facts produced by the mover without depending on app logging or assets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementDiagnostics {
    pub settle_lift: Option<f32>,
    pub wedge_rest_position: Option<Vec3>,
    pub step_contact: Option<(Vec3, Vec3)>,
    pub step_probe: Option<StepProbeRequest>,
    pub trace: Option<TraceFrame>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepProbeRequest {
    pub from_center: Vec3,
    pub to_center: Vec3,
    pub horizontal_velocity: Vec3,
    pub delta_seconds: f32,
    pub elapsed_seconds: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TraceFrame {
    pub feet_y_in: f32,
    pub feet_y_out: f32,
    pub horizontal_distance: f32,
    pub grounded: bool,
    pub on_walkable: bool,
    pub moving: bool,
    pub vertical_velocity: f32,
    pub snap: Option<(f32, Option<(f32, f32)>)>,
    pub climb: Option<f32>,
    pub anchored: bool,
}
