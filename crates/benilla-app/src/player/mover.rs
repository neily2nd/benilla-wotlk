//! Thin adapter between Benilla's network-oriented player and the reusable movement crate.

use avian3d::prelude::Collider;
use bevy::prelude::{Time, Vec3};

use super::{move_trace, step_probe, Player, CAPSULE_HEIGHT, CAPSULE_RADIUS, SKIN_WIDTH};
use benilla_movement::{
    CollisionConfig, MovementInput, MovementModes, MovementOutcome, MovementState,
};

pub(crate) use benilla_movement::{
    airborne_step, fall_step, grounded_step, step_up, StepVerdict, Support,
};

/// The liquid surface may be used as ground only under the upstream movement-mode gates.
pub(super) fn water_floor(
    water_walking: bool,
    swimming: bool,
    mover_pitch: f32,
    surface_y: Option<f32>,
) -> Option<f32> {
    benilla_movement::water_floor(water_walking, swimming, mover_pitch, surface_y)
}

/// Result fields consumed by the existing movement flag and transport integration.
pub(super) struct Outcome {
    pub held: bool,
    pub grounded: bool,
    pub jumped: bool,
    pub knocked: bool,
    pub air_nudged: bool,
    pub ground: Option<bevy::prelude::Entity>,
}

/// Copy only kinematic fields across the application boundary, advance the shared mover, and copy
/// the state back. The `Player` remains private to the application and is never exposed by the
/// movement crate.
pub(super) fn step(
    player: &mut Player,
    time: &Time,
    world: &benilla_world::collision::WorldCollision<'_, '_>,
    capsule: &Collider,
    moving: bool,
    dir: Vec3,
    speed: f32,
    want_jump: bool,
    wire_jump: bool,
    knockback: Option<Vec3>,
    water_floor: Option<f32>,
    nudge_speed: f32,
) -> Outcome {
    let mut state = MovementState {
        pos: player.pos,
        vel_y: player.vel_y,
        horiz_vel: player.horiz_vel,
        airborne_since: player.airborne_since,
        wedged: player.wedged,
        wedge_still: player.wedge_still,
        steep_support: player.steep_support,
        step_start: player.step_start,
        launch_vz: player.launch_vz,
        arc_dirs_set: player.arc_dirs_set,
        knock_arc: player.knock_arc,
        airborne_prev: player.airborne_prev,
        settling: player.settling,
        settle_lift: player.settle_lift,
        modes: MovementModes {
            rooted: player.modes.rooted,
            water_walking: player.modes.water_walking,
            feather_fall: player.modes.feather_fall,
            hover: player.modes.hover,
            levitating: player.modes.levitating,
        },
    };
    let result = benilla_movement::advance(
        &mut state,
        time.delta(),
        time.elapsed_secs(),
        world,
        CollisionConfig {
            collider: capsule,
            capsule_radius: CAPSULE_RADIUS,
            capsule_height: CAPSULE_HEIGHT,
            skin_width: SKIN_WIDTH,
        },
        MovementInput {
            moving,
            direction: dir,
            speed,
            want_jump,
            wire_jump,
            knockback_velocity: knockback,
            water_floor,
            air_nudge_speed: nudge_speed,
        },
    );
    player.pos = state.pos;
    player.vel_y = state.vel_y;
    player.horiz_vel = state.horiz_vel;
    player.airborne_since = state.airborne_since;
    player.wedged = state.wedged;
    player.wedge_still = state.wedge_still;
    player.steep_support = state.steep_support;
    player.step_start = state.step_start;
    player.launch_vz = state.launch_vz;
    player.arc_dirs_set = state.arc_dirs_set;
    player.knock_arc = state.knock_arc;
    player.airborne_prev = state.airborne_prev;
    player.settling = state.settling;
    player.settle_lift = state.settle_lift;
    emit_diagnostics(world, capsule, result);
    Outcome {
        held: result.held,
        grounded: result.grounded,
        jumped: result.jumped,
        knocked: result.knocked,
        air_nudged: result.air_nudged,
        ground: result.ground,
    }
}

fn emit_diagnostics(
    world: &benilla_world::collision::WorldCollision<'_, '_>,
    capsule: &Collider,
    result: MovementOutcome,
) {
    let diagnostics = result.diagnostics;
    if let Some(up) = diagnostics.settle_lift {
        benilla_assets::trace::line("move", &format!("settle lift {up:+.2}"));
    }
    if let Some((point, normal)) = diagnostics.step_contact {
        if benilla_assets::trace::enabled_for("step") {
            benilla_assets::trace::line(
                "step",
                &format!(
                    "hit ({:.2},{:.2},{:.2}) n=({:+.2},{:+.2},{:+.2})",
                    point.x, point.y, point.z, normal.x, normal.y, normal.z
                ),
            );
        }
    }
    if let Some(request) = diagnostics.step_probe {
        step_probe::watch(
            world,
            capsule,
            request.from_center,
            request.to_center,
            request.horizontal_velocity,
            request.delta_seconds,
            request.elapsed_seconds,
        );
    }
    if let Some(position) = diagnostics.wedge_rest_position {
        benilla_assets::trace::line(
            "move",
            &format!(
                "wedge rest at ({:8.2},{:7.2},{:8.2}) -> landed standing",
                position.x, position.y, position.z
            ),
        );
    }
    if let Some(frame) = diagnostics.trace {
        move_trace::frame(move_trace::Frame {
            y_in: frame.feet_y_in,
            y_out: frame.feet_y_out,
            dx: frame.horizontal_distance,
            grounded: frame.grounded,
            on_walkable: frame.on_walkable,
            moving: frame.moving,
            vel_y: frame.vertical_velocity,
            snap: frame.snap,
            climb: frame.climb,
            anchored: frame.anchored,
        });
    }
}
