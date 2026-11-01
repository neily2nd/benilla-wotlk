//! Same-world diagnostic replay of the current playable mover snapshot and extracted mover.
//! The legacy source snapshot is byte-for-byte copied from the read-only POC checkout; this
//! harness and its outputs live only in the integration checkout.

use std::{fs, path::PathBuf, time::Duration};

use avian3d::prelude::*;
use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_movement::{
    advance, CollisionConfig, MovementInput, MovementState, FEATHER_TERMINAL_VELOCITY,
    FOOT_CONE_HEIGHT, GRAVITY, GROUND_COS, GROUND_PROBE, HOVER_CLIMB_RATE, HOVER_HEIGHT,
    JUMP_SPEED, LAND_PROBE, SKIN_WIDTH, STEP_SLOPE_RATIO, STEP_SNAP_SLACK,
    STEP_UP_ADVANCE_PER_YARD, STEP_UP_HEIGHT, TERMINAL_VELOCITY, WATER_WALK_PITCH_FLOOR,
    WEDGE_MIN_FALL, WEDGE_STALL_RATIO, WEDGE_STILL_FRAMES,
};
use benilla_world::{
    build_id::BuildId,
    collision::{GroundDecalSurface, WorldCollision},
    schedule::WorldStage,
    terrain_stream::WorldLoadProgress,
    worldview,
};
use bevy::{prelude::*, time::Real};

const SPEED: f32 = 3.5;
const RADIUS: f32 = 1.0 / 3.0;
const HEIGHT: f32 = 2.027_777_7;
const MAX_STEPS_PER_UPDATE: usize = 64;
const LOAD_TIMEOUT: f32 = 120.0;
const CAPSULE_RADIUS: f32 = benilla_movement::CAPSULE_RADIUS;
const CAPSULE_HEIGHT: f32 = benilla_movement::CAPSULE_HEIGHT;

// The exact source state required by the copied POC mover is also the extracted crate's public
// state. Both copies start from `Default` and receive the same physically ray-grounded feet pose.
type Player = MovementState;
// The byte-identical snapshot retains its upstream unit tests; provide only their former app
// aliases when clippy checks this diagnostic binary with `cfg(test)` enabled.
#[cfg(test)]
mod state {
    pub type MoveModes = benilla_movement::MovementModes;
}

#[cfg(test)]
mod player {
    pub use benilla_movement::{AIR_NUDGE_SPEED, CAPSULE_RADIUS};

    pub mod state {
        pub const STEP_UP_ADVANCE: f32 = benilla_movement::STEP_UP_ADVANCE;
    }
}

mod move_trace {
    #[allow(dead_code)]
    pub(super) struct Frame {
        pub y_in: f32,
        pub y_out: f32,
        pub dx: f32,
        pub grounded: bool,
        pub on_walkable: bool,
        pub moving: bool,
        pub vel_y: f32,
        pub snap: Option<(f32, Option<(f32, f32)>)>,
        pub climb: Option<f32>,
        pub anchored: bool,
    }

    pub(super) fn frame(_frame: Frame) {}
}

mod step_probe {
    use avian3d::prelude::Collider;
    use bevy::prelude::Vec3;

    pub(super) fn watch(
        _world: &benilla_world::collision::WorldCollision<'_, '_>,
        _capsule: &Collider,
        _from: Vec3,
        _to: Vec3,
        _velocity: Vec3,
        _dt: f32,
        _now: f32,
    ) {
    }
}

#[path = "snapshots/legacy_poc_mover_snapshot.rs"]
#[allow(dead_code)]
mod legacy_poc_mover;

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    start_wow: [f32; 3],
    direction_wow: [f32; 2],
    hold_secs: f32,
    move_secs: f32,
}

const SCENARIOS: &[Scenario] = &[
    scenario(
        "tree_a_hold",
        [-9482.59, 45.0218, 57.132965],
        [0.0, 0.0],
        3.0,
        0.0,
    ),
    scenario(
        "tree_a_x_positive",
        [-9482.59, 45.0218, 57.132965],
        [1.0, 0.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_a_x_negative",
        [-9482.59, 45.0218, 57.132965],
        [-1.0, 0.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_a_y_positive",
        [-9482.59, 45.0218, 57.132965],
        [0.0, 1.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_a_y_negative",
        [-9482.59, 45.0218, 57.132965],
        [0.0, -1.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_b_hold",
        [-9483.09, 44.7718, 57.30507],
        [0.0, 0.0],
        3.0,
        0.0,
    ),
    scenario(
        "tree_b_x_positive",
        [-9483.09, 44.7718, 57.30507],
        [1.0, 0.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_b_x_negative",
        [-9483.09, 44.7718, 57.30507],
        [-1.0, 0.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_b_y_positive",
        [-9483.09, 44.7718, 57.30507],
        [0.0, 1.0],
        0.25,
        3.0,
    ),
    scenario(
        "tree_b_y_negative",
        [-9483.09, 44.7718, 57.30507],
        [0.0, -1.0],
        0.25,
        3.0,
    ),
    scenario(
        "fence_parallel_and_recover",
        [-9567.4, 255.8, 49.84],
        [0.0, 1.0],
        0.25,
        3.5,
    ),
];

const fn scenario(
    name: &'static str,
    start_wow: [f32; 3],
    direction_wow: [f32; 2],
    hold_secs: f32,
    move_secs: f32,
) -> Scenario {
    Scenario {
        name,
        start_wow,
        direction_wow,
        hold_secs,
        move_secs,
    }
}

#[derive(Resource)]
struct Replay {
    capsule: Collider,
    legacy: MovementState,
    extracted: MovementState,
    scenario_index: usize,
    rate_index: usize,
    tick: usize,
    elapsed: f32,
    sim_time: Time,
    grounded_at: Vec3,
    grounded_from_offset: Vec2,
    grounded_entity: Entity,
    ray_origin: Vec3,
    ray_distance: f32,
    ray_normal: Vec3,
    legacy_grounded: usize,
    extracted_grounded: usize,
    legacy_air_episodes: usize,
    extracted_air_episodes: usize,
    traces: Vec<String>,
    starts: Vec<String>,
    readiness: Vec<String>,
    poc_gate_recorded: bool,
    summaries: Vec<String>,
    load_started: Option<f32>,
    finished: bool,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            capsule: Collider::capsule(RADIUS, HEIGHT - 2.0 * RADIUS),
            legacy: MovementState::default(),
            extracted: MovementState::default(),
            scenario_index: 0,
            rate_index: 0,
            tick: 0,
            elapsed: 0.0,
            sim_time: Time::default(),
            grounded_at: Vec3::ZERO,
            grounded_from_offset: Vec2::ZERO,
            grounded_entity: Entity::PLACEHOLDER,
            ray_origin: Vec3::ZERO,
            ray_distance: 0.0,
            ray_normal: Vec3::Y,
            legacy_grounded: 0,
            extracted_grounded: 0,
            legacy_air_episodes: 0,
            extracted_air_episodes: 0,
            traces: vec!["scenario\thz\ttick\ttime\tdir_x\tdir_z\tlegacy_x\tlegacy_y\tlegacy_z\tlegacy_vx\tlegacy_vy\tlegacy_vz\tlegacy_grounded\tlegacy_ground\tlegacy_airborne_since\tlegacy_contact\tlegacy_obstacle\textracted_x\textracted_y\textracted_z\textracted_vx\textracted_vy\textracted_vz\textracted_grounded\textracted_ground\textracted_airborne_since\textracted_contact\textracted_obstacle".into()],
            starts: vec!["scenario\thz\trequested_wow\tactual_wow\tbevy_feet\tcandidate_offset\tray_origin\tray_distance\tray_hit_point\tray_normal\thit_entity\tground_decal\tinitial_fields_default\tcandidate_attempts".into()],
            readiness: vec!["gate\tready\ttotal\tfocus_resident\tscene_ready\tpinned_collision_ready\tcolliders_pending\tmerge_pending\tplacements_pending\tgx_pending\traw_legacy_grounded\traw_legacy_ground\traw_legacy_feet\traw_extracted_grounded\traw_extracted_ground\traw_extracted_feet".into()],
            poc_gate_recorded: false,
            summaries: vec!["scenario\thz\tticks\tlegacy_grounded_ticks\textracted_grounded_ticks\tlegacy_air_episodes\textracted_air_episodes\tmax_position_delta_yd\tmax_velocity_delta_yd_s\tfirst_divergent_tick\tground_state_mismatches\tairborne_time_mismatches\tcontact_probe_mismatches\tlegacy_contact_ticks\textracted_contact_ticks\tlegacy_obstacle_ticks\textracted_obstacle_ticks\tverdict".into()],
            load_started: None,
            finished: false,
        }
    }
}

struct ReplayPlugin;
impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Replay::default())
            .add_systems(Update, drive.in_set(WorldStage::Present));
    }
}

fn main() -> benilla_world::AppExit {
    worldview::run_with(
        BuildId {
            version: env!("CARGO_PKG_VERSION"),
            describe: env!("BENILLA_GIT_DESCRIBE"),
            sha: env!("BENILLA_GIT_SHA"),
            short: env!("BENILLA_GIT_SHORT"),
            date: env!("BENILLA_GIT_DATE"),
            profile: env!("BENILLA_PROFILE"),
            project_dir: env!("BENILLA_PROJECT_DIR"),
            ..Default::default()
        },
        |app| {
            app.add_plugins(ReplayPlugin);
        },
    )
}

fn drive(
    mut replay: ResMut<Replay>,
    progress: Option<Res<WorldLoadProgress>>,
    real_time: Res<Time<Real>>,
    collision: WorldCollision,
    ground_surfaces: Query<(), With<GroundDecalSurface>>,
    mut exit: MessageWriter<AppExit>,
) {
    if replay.finished {
        return;
    }
    if replay.scenario_index >= SCENARIOS.len() {
        if let Err(error) = write_outputs(&replay) {
            error!("ELW_014B1C output failure: {error:#}");
            exit.write(AppExit::error());
        } else {
            info!("ELW_014B1C complete output={}", output_dir().display());
            exit.write(AppExit::Success);
        }
        replay.finished = true;
        return;
    }
    let Some(progress) = progress else { return };
    if !replay.poc_gate_recorded && poc_presentable(progress.as_ref()) {
        capture_poc_gate(&mut replay, progress.as_ref(), &collision);
    }
    if !progress.presentable() {
        let started = *replay.load_started.get_or_insert(real_time.elapsed_secs());
        if real_time.elapsed_secs() - started > LOAD_TIMEOUT {
            error!(
                "ELW_014B1C world timeout ready={}/{} focus={} colliders={}",
                progress.ready, progress.total, progress.focus_resident, progress.colliders_pending
            );
            replay.finished = true;
            exit.write(AppExit::error());
        }
        return;
    }
    if replay.tick == 0 && replay.elapsed == 0.0 {
        if let Err(reason) = begin(&mut replay, &collision, &ground_surfaces) {
            error!("ELW_014B1C cannot establish physical start: {reason}");
            replay.finished = true;
            exit.write(AppExit::error());
            return;
        }
    }

    for _ in 0..MAX_STEPS_PER_UPDATE {
        if replay.elapsed >= total_secs(SCENARIOS[replay.scenario_index]) {
            finish_scenario(&mut replay);
            if replay.scenario_index >= SCENARIOS.len() {
                break;
            }
            if let Err(reason) = begin(&mut replay, &collision, &ground_surfaces) {
                error!("ELW_014B1C cannot establish physical start: {reason}");
                replay.finished = true;
                exit.write(AppExit::error());
                return;
            }
        }
        let scenario = SCENARIOS[replay.scenario_index];
        let hz = [30_u16, 60, 120][replay.rate_index];
        let dt = 1.0 / f32::from(hz);
        let moving = replay.elapsed >= scenario.hold_secs && scenario.move_secs > 0.0;
        let direction_wow = scenario_direction_wow(scenario, replay.elapsed - scenario.hold_secs);
        let direction = wow_direction(direction_wow);
        let speed = if moving { SPEED } else { 0.0 };
        let legacy_contact = contact_probe(
            &collision,
            &replay.capsule,
            replay.legacy.pos,
            direction,
            speed,
            dt,
            moving,
        );
        let extracted_contact = contact_probe(
            &collision,
            &replay.capsule,
            replay.extracted.pos,
            direction,
            speed,
            dt,
            moving,
        );
        replay.sim_time.advance_by(Duration::from_secs_f32(dt));
        let elapsed_secs = replay.sim_time.elapsed_secs();
        let legacy_outcome = {
            let Replay {
                legacy,
                capsule,
                sim_time,
                ..
            } = &mut *replay;
            legacy_poc_mover::step(
                legacy, sim_time, &collision, capsule, moving, direction, speed, false, false,
                None, None, SPEED,
            )
        };
        let extracted_outcome = {
            let Replay {
                extracted,
                capsule,
                sim_time,
                ..
            } = &mut *replay;
            advance(
                extracted,
                sim_time.delta(),
                elapsed_secs,
                &collision,
                CollisionConfig {
                    collider: capsule,
                    capsule_radius: RADIUS,
                    capsule_height: HEIGHT,
                    skin_width: SKIN_WIDTH,
                },
                MovementInput {
                    moving,
                    direction,
                    speed,
                    want_jump: false,
                    wire_jump: false,
                    knockback_velocity: None,
                    water_floor: None,
                    air_nudge_speed: SPEED,
                },
            )
        };
        update_airborne(&mut replay.legacy, legacy_outcome.grounded, elapsed_secs);
        update_airborne(
            &mut replay.extracted,
            extracted_outcome.grounded,
            elapsed_secs,
        );
        if legacy_outcome.grounded {
            replay.legacy_grounded += 1;
        } else if replay.legacy.airborne_since == Some(elapsed_secs) {
            replay.legacy_air_episodes += 1;
        }
        if extracted_outcome.grounded {
            replay.extracted_grounded += 1;
        } else if replay.extracted.airborne_since == Some(elapsed_secs) {
            replay.extracted_air_episodes += 1;
        }
        replay.elapsed += dt;
        replay.tick += 1;
        let l = replay.legacy;
        let e = replay.extracted;
        let lpos = bevy_to_wow(l.pos);
        let epos = bevy_to_wow(e.pos);
        let tick = replay.tick;
        let elapsed = replay.elapsed;
        replay.traces.push(format!(
            "{}\t{}\t{}\t{:.9}\t{:.6}\t{:.6}\t{:.7}\t{:.7}\t{:.7}\t{:.7}\t{:.7}\t{:.7}\t{}\t{:?}\t{:?}\t{}\t{}\t{:.7}\t{:.7}\t{:.7}\t{:.7}\t{:.7}\t{:.7}\t{}\t{:?}\t{:?}\t{}\t{}",
            scenario.name, hz, tick, elapsed, direction.x, direction.z,
            lpos[0], lpos[1], lpos[2], l.horiz_vel.x, l.vel_y, l.horiz_vel.z,
            legacy_outcome.grounded, legacy_outcome.ground, l.airborne_since,
            legacy_contact.0, legacy_contact.1,
            epos[0], epos[1], epos[2], e.horiz_vel.x, e.vel_y, e.horiz_vel.z,
            extracted_outcome.grounded, extracted_outcome.ground, e.airborne_since,
            extracted_contact.0, extracted_contact.1,
        ));
    }
}

fn poc_presentable(progress: &WorldLoadProgress) -> bool {
    progress.scene_ready
        && progress.colliders_pending == 0
        && progress.merge_pending == 0
        && progress.gx_pending == 0
}

fn capture_poc_gate(replay: &mut Replay, progress: &WorldLoadProgress, collision: &WorldCollision) {
    let base = wow_to_bevy(SCENARIOS[0].start_wow);
    let mut legacy = MovementState {
        pos: base,
        ..Default::default()
    };
    let mut extracted = legacy;
    let mut time = Time::default();
    time.advance_by(Duration::from_secs_f32(1.0 / 60.0));
    let legacy_out = legacy_poc_mover::step(
        &mut legacy,
        &time,
        collision,
        &replay.capsule,
        false,
        Vec3::ZERO,
        0.0,
        false,
        false,
        None,
        None,
        SPEED,
    );
    let extracted_out = advance(
        &mut extracted,
        time.delta(),
        time.elapsed_secs(),
        collision,
        CollisionConfig {
            collider: &replay.capsule,
            capsule_radius: RADIUS,
            capsule_height: HEIGHT,
            skin_width: SKIN_WIDTH,
        },
        MovementInput {
            air_nudge_speed: SPEED,
            ..Default::default()
        },
    );
    replay.readiness.push(format!(
        "POC_presentable\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{}\t{:?}\t{:?}",
        progress.ready,
        progress.total,
        progress.focus_resident,
        progress.scene_ready,
        progress.pinned_collision_ready,
        progress.colliders_pending,
        progress.merge_pending,
        progress.placements_pending,
        progress.gx_pending,
        legacy_out.grounded,
        legacy_out.ground,
        legacy.pos,
        extracted_out.grounded,
        extracted_out.ground,
        extracted.pos,
    ));
    replay.poc_gate_recorded = true;
}

fn begin(
    replay: &mut Replay,
    collision: &WorldCollision,
    ground_surfaces: &Query<(), With<GroundDecalSurface>>,
) -> Result<(), String> {
    let scenario = SCENARIOS[replay.scenario_index];
    let base = wow_to_bevy(scenario.start_wow);
    let mut candidates = vec![Vec2::ZERO];
    for radius in [0.5, 1.0, 1.5] {
        candidates.extend([
            Vec2::new(radius, 0.0),
            Vec2::new(0.0, radius),
            Vec2::new(-radius, 0.0),
            Vec2::new(0.0, -radius),
            Vec2::new(radius, radius),
            Vec2::new(-radius, radius),
            Vec2::new(radius, -radius),
            Vec2::new(-radius, -radius),
        ]);
    }
    let mut raw_legacy = MovementState {
        pos: base,
        ..Default::default()
    };
    let mut raw_extracted = raw_legacy;
    let mut raw_time = Time::default();
    raw_time.advance_by(Duration::from_secs_f32(1.0 / 60.0));
    let raw_legacy_out = legacy_poc_mover::step(
        &mut raw_legacy,
        &raw_time,
        collision,
        &replay.capsule,
        false,
        Vec3::ZERO,
        0.0,
        false,
        false,
        None,
        None,
        SPEED,
    );
    let raw_extracted_out = advance(
        &mut raw_extracted,
        raw_time.delta(),
        raw_time.elapsed_secs(),
        collision,
        CollisionConfig {
            collider: &replay.capsule,
            capsule_radius: RADIUS,
            capsule_height: HEIGHT,
            skin_width: SKIN_WIDTH,
        },
        MovementInput {
            air_nudge_speed: SPEED,
            ..Default::default()
        },
    );
    let raw_probe = format!(
        "raw_at_authored_pose=legacy_grounded:{},ground:{:?},feet:{:?};extracted_grounded:{},ground:{:?},feet:{:?}",
        raw_legacy_out.grounded,
        raw_legacy_out.ground,
        raw_legacy.pos,
        raw_extracted_out.grounded,
        raw_extracted_out.ground,
        raw_extracted.pos,
    );
    let mut selected = None;
    let mut attempts = Vec::new();
    for offset in candidates {
        let point = base + Vec3::new(offset.x, 0.0, offset.y);
        let origin = point + Vec3::Y * 12.0;
        let Some(hit) = collision.ray_body(origin, Dir3::NEG_Y, 24.0) else {
            attempts.push(format!("off={offset:?}:ray=none"));
            continue;
        };
        let decal = ground_surfaces.get(hit.entity).is_ok();
        if hit.normal.y < GROUND_COS || !decal {
            attempts.push(format!(
                "off={offset:?}:ray=hit(entity={:?},d={:.6},ny={:.6}),walkable_n={},ground_decal={decal}",
                hit.entity,
                hit.distance,
                hit.normal.y,
                hit.normal.y >= GROUND_COS
            ));
            continue;
        }
        let feet = origin - Vec3::Y * hit.distance;
        // Both movers must certify this support on their first actual movement step; no state bit
        // is forced. Reject this candidate if a zero-input step cannot recognize its grounded hit.
        let mut legacy = MovementState {
            pos: feet,
            ..Default::default()
        };
        let mut extracted = legacy;
        let mut test_time = Time::default();
        test_time.advance_by(Duration::from_secs_f32(1.0 / 60.0));
        let lo = legacy_poc_mover::step(
            &mut legacy,
            &test_time,
            collision,
            &replay.capsule,
            false,
            Vec3::ZERO,
            0.0,
            false,
            false,
            None,
            None,
            SPEED,
        );
        let eo = advance(
            &mut extracted,
            test_time.delta(),
            test_time.elapsed_secs(),
            collision,
            CollisionConfig {
                collider: &replay.capsule,
                capsule_radius: RADIUS,
                capsule_height: HEIGHT,
                skin_width: SKIN_WIDTH,
            },
            MovementInput {
                air_nudge_speed: SPEED,
                ..Default::default()
            },
        );
        attempts.push(format!(
            "off={offset:?}:ray=hit(entity={:?},d={:.6},ny={:.6}),ground_decal={decal},probe=legacy({}, {:?})/extracted({}, {:?})",
            hit.entity, hit.distance, hit.normal.y, lo.grounded, lo.ground, eo.grounded, eo.ground
        ));
        if !lo.grounded
            || !eo.grounded
            || lo.ground != Some(hit.entity)
            || eo.ground != Some(hit.entity)
        {
            continue;
        }
        selected = Some((feet, offset, origin, hit.distance, hit.normal, hit.entity));
        break;
    }
    let Some((feet, offset, origin, distance, normal, entity)) = selected else {
        return Err(format!(
            "{} at {:?}: no nearby ray hit accepted by both movers as walkable support",
            scenario.name, scenario.start_wow
        ));
    };
    replay.legacy = MovementState {
        pos: feet,
        ..Default::default()
    };
    replay.extracted = replay.legacy;
    replay.grounded_at = feet;
    replay.grounded_from_offset = offset;
    replay.grounded_entity = entity;
    replay.ray_origin = origin;
    replay.ray_distance = distance;
    replay.ray_normal = normal;
    replay.tick = 0;
    replay.elapsed = 0.0;
    replay.sim_time = Time::default();
    replay.legacy_grounded = 0;
    replay.extracted_grounded = 0;
    replay.legacy_air_episodes = 0;
    replay.extracted_air_episodes = 0;
    let actual = bevy_to_wow(feet);
    replay.starts.push(format!(
        "{}\t{}\t{:?}\t{:.7},{:.7},{:.7}\t{:.7},{:.7},{:.7}\t{:.2},{:.2}\t{:.7},{:.7},{:.7}\t{:.7}\t{:.7},{:.7},{:.7}\t{:?}\t{:?}\ttrue\ttrue\t{raw_probe};{}",
        scenario.name, [30, 60, 120][replay.rate_index], scenario.start_wow,
        actual[0], actual[1], actual[2], feet.x, feet.y, feet.z, offset.x, offset.y,
        origin.x, origin.y, origin.z, distance, (origin - Vec3::Y * distance).x,
        (origin - Vec3::Y * distance).y,
        (origin - Vec3::Y * distance).z,
        normal,
        entity,
        attempts.join(";"),
    ));
    Ok(())
}

fn update_airborne(state: &mut MovementState, grounded: bool, elapsed: f32) {
    if grounded {
        state.airborne_since = None;
    } else {
        state.airborne_since.get_or_insert(elapsed);
    }
}

fn contact_probe(
    collision: &WorldCollision,
    capsule: &Collider,
    feet: Vec3,
    direction: Vec3,
    speed: f32,
    dt: f32,
    moving: bool,
) -> (bool, bool) {
    if !moving {
        return (false, false);
    }
    let center = feet + Vec3::Y * (HEIGHT * 0.5);
    collision
        .cast_mover(capsule, center, direction * (speed * dt), SKIN_WIDTH)
        .map(|hit| (true, hit.normal1.y.abs() < 0.7))
        .unwrap_or((false, false))
}

fn finish_scenario(replay: &mut Replay) {
    let scenario = SCENARIOS[replay.scenario_index];
    let hz = [30, 60, 120][replay.rate_index];
    let mut max_pos = 0.0f32;
    let mut max_vel = 0.0f32;
    let mut first_divergence = None;
    let mut position_failures = 0usize;
    let mut velocity_failures = 0usize;
    let mut ground_mismatches = 0usize;
    let mut airborne_time_mismatches = 0usize;
    let mut contact_mismatches = 0usize;
    let mut legacy_contacts = 0usize;
    let mut extracted_contacts = 0usize;
    let mut legacy_obstacles = 0usize;
    let mut extracted_obstacles = 0usize;
    for row in replay.traces.iter().skip(1) {
        let columns: Vec<_> = row.split('\t').collect();
        if columns[0] != scenario.name || columns[1].parse::<usize>().ok() != Some(hz) {
            continue;
        }
        let v = |first: usize| {
            Vec3::new(
                columns[first].parse().unwrap_or_default(),
                columns[first + 1].parse().unwrap_or_default(),
                columns[first + 2].parse().unwrap_or_default(),
            )
        };
        let position_delta = (v(6) - v(17)).length();
        let velocity_delta = (v(9) - v(20)).length();
        max_pos = max_pos.max(position_delta);
        max_vel = max_vel.max(velocity_delta);
        let position_bad = position_delta > 0.001;
        let velocity_bad = velocity_delta > 0.001;
        let ground_bad = columns[12] != columns[23] || columns[13] != columns[24];
        let airborne_time_bad = columns[14] != columns[25];
        let contact_bad = columns[15] != columns[26] || columns[16] != columns[27];
        position_failures += usize::from(position_bad);
        velocity_failures += usize::from(velocity_bad);
        ground_mismatches += usize::from(ground_bad);
        airborne_time_mismatches += usize::from(airborne_time_bad);
        contact_mismatches += usize::from(contact_bad);
        legacy_contacts += usize::from(columns[15] == "true");
        extracted_contacts += usize::from(columns[26] == "true");
        legacy_obstacles += usize::from(columns[16] == "true");
        extracted_obstacles += usize::from(columns[27] == "true");
        if first_divergence.is_none()
            && (position_bad || velocity_bad || ground_bad || airborne_time_bad || contact_bad)
        {
            first_divergence = Some(columns[2]);
        }
    }
    let passed = position_failures == 0
        && velocity_failures == 0
        && ground_mismatches == 0
        && airborne_time_mismatches == 0
        && contact_mismatches == 0;
    replay.summaries.push(format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9}\t{:.9}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        scenario.name,
        hz,
        replay.tick,
        replay.legacy_grounded,
        replay.extracted_grounded,
        replay.legacy_air_episodes,
        replay.extracted_air_episodes,
        max_pos,
        max_vel,
        first_divergence.unwrap_or("none"),
        ground_mismatches,
        airborne_time_mismatches,
        contact_mismatches,
        legacy_contacts,
        extracted_contacts,
        legacy_obstacles,
        extracted_obstacles,
        if passed { "PASS" } else { "FAIL" },
    ));
    replay.scenario_index += 1;
    if replay.scenario_index == SCENARIOS.len() {
        replay.rate_index += 1;
        if replay.rate_index < 3 {
            replay.scenario_index = 0;
        }
    }
}

fn scenario_direction_wow(s: Scenario, elapsed: f32) -> [f32; 2] {
    match s.name {
        "fence_parallel_and_recover" if elapsed < 0.25 => [0.0, 1.0],
        "fence_parallel_and_recover" if elapsed < 2.5 => [1.0, 0.0],
        "fence_parallel_and_recover" => [0.0, -1.0],
        _ => s.direction_wow,
    }
}
fn wow_direction(d: [f32; 2]) -> Vec3 {
    Vec3::new(-d[1], 0.0, -d[0]).normalize_or_zero()
}
fn total_secs(s: Scenario) -> f32 {
    s.hold_secs + s.move_secs
}
fn output_dir() -> PathBuf {
    std::env::var_os("ELW_014B1C_OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/elw-collision-014b1c-current"))
}
fn write_outputs(replay: &Replay) -> anyhow::Result<()> {
    let dir = output_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("paired-per-tick.tsv"), replay.traces.join("\n"))?;
    fs::write(
        dir.join("ground-initialization.tsv"),
        replay.starts.join("\n"),
    )?;
    fs::write(dir.join("readiness-probe.tsv"), replay.readiness.join("\n"))?;
    fs::write(dir.join("paired-summary.tsv"), replay.summaries.join("\n"))?;
    fs::write(dir.join("source-snapshot.txt"), "Current POC mover snapshot SHA256=6541C4E0B5DD96913F373D039A7F75DFE9D69C8E50B1A98CC8394ECFDB1729EC\nPOC source=read-only accepted golden-reference checkout\nIntegration copy=crates/benilla-worldview/src/bin/snapshots/legacy_poc_mover_snapshot.rs\nVerified walkable start ray: origin, hit distance/point/normal/entity; both movers' 60Hz zero-input probe must return grounded on the same entity before replay starts.\n")?;
    Ok(())
}
