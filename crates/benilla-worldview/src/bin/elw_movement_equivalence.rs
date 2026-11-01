//! Deterministic real-world replay for the extracted mover. This runs inside the normal
//! `benilla-worldview` bootstrap so its WotLK collision/terrain streaming matches the POC.

use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use avian3d::prelude::*;
use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_movement::{advance, CollisionConfig, MovementInput, MovementState};
use benilla_world::{
    build_id::BuildId, collision::WorldCollision, schedule::WorldStage,
    terrain_stream::WorldLoadProgress, worldview,
};
use bevy::{prelude::*, time::Real};

const SPEED: f32 = 3.5;
const RADIUS: f32 = 1.0 / 3.0;
const HEIGHT: f32 = 2.027_777_7;
const SKIN: f32 = benilla_movement::SKIN_WIDTH;
const LOAD_TIMEOUT_SECS: f32 = 120.0;
const MAX_STEPS_PER_UPDATE: usize = 64;

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    start_wow: [f32; 3],
    direction_wow: [f32; 2],
    hold_secs: f32,
    move_secs: f32,
    jump_once: bool,
}

// Replay every fixed trajectory from the preserved upstream-capsule Goldshire POC, followed by
// its captured terrain_ledge trajectory. Fence cases and directional variations are retained.
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
        "fence_direct_approach",
        [-9563.15, 252.5, 49.95],
        [0.0, 1.0],
        0.25,
        4.5,
    ),
    scenario(
        "fence_parallel_and_recover",
        [-9567.4, 255.8, 49.84],
        [0.0, 1.0],
        0.25,
        3.5,
    ),
    scenario(
        "fence_recover",
        [-9563.15, 255.8, 49.95],
        [0.0, -1.0],
        0.25,
        1.5,
    ),
    scenario(
        "blacksmith_floor_hold",
        [-9455.0, 100.0, 58.34],
        [0.0, 0.0],
        3.0,
        0.0,
    ),
    scenario(
        "blacksmith_stairs_down",
        [-9458.89, 104.918, 58.385],
        [-7.0, 8.0],
        0.25,
        1.5,
    ),
    scenario(
        "blacksmith_stairs_up",
        [-9462.0, 108.0, 57.66],
        [7.0, -8.0],
        0.25,
        1.5,
    ),
    scenario(
        "blacksmith_wall_x_positive",
        [-9455.0, 100.0, 58.34],
        [1.0, 0.0],
        0.25,
        2.5,
    ),
    scenario(
        "blacksmith_wall_x_negative",
        [-9455.0, 100.0, 58.34],
        [-1.0, 0.0],
        0.25,
        2.5,
    ),
    scenario(
        "blacksmith_wall_y_positive",
        [-9455.0, 100.0, 58.34],
        [0.0, 1.0],
        0.25,
        2.5,
    ),
    scenario(
        "blacksmith_wall_y_negative",
        [-9455.0, 100.0, 58.34],
        [0.0, -1.0],
        0.25,
        2.5,
    ),
    scenario(
        "terrain_ledge",
        [-9306.0, 79.0, 66.54599],
        [0.0, 1.0],
        0.25,
        2.0,
    ),
    Scenario {
        name: "goldshire_jump_and_land",
        start_wow: [-9482.59, 45.0218, 57.132965],
        direction_wow: [0.0, 0.0],
        hold_secs: 0.25,
        move_secs: 2.0,
        jump_once: true,
    },
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
        jump_once: false,
    }
}

#[derive(Resource)]
struct Replay {
    capsule: Collider,
    state: MovementState,
    scenario_index: usize,
    rate_index: usize,
    tick: usize,
    elapsed: f32,
    sim_time: Time,
    start: Vec3,
    grounded: usize,
    airborne: usize,
    airborne_episodes: usize,
    jumps: usize,
    jump_requested: bool,
    final_grounded: bool,
    sample_count: usize,
    contact_ticks: usize,
    obstacle_contact_ticks: usize,
    timings_ns: Vec<u64>,
    traces: Vec<String>,
    samples: Vec<String>,
    summaries: Vec<String>,
    load_started: Option<f32>,
    finished: bool,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            capsule: Collider::capsule(RADIUS, HEIGHT - 2.0 * RADIUS),
            state: MovementState::default(),
            scenario_index: 0,
            rate_index: 0,
            tick: 0,
            elapsed: 0.0,
            sim_time: Time::default(),
            start: Vec3::ZERO,
            grounded: 0,
            airborne: 0,
            airborne_episodes: 0,
            jumps: 0,
            jump_requested: false,
            final_grounded: false,
            sample_count: 0,
            contact_ticks: 0,
            obstacle_contact_ticks: 0,
            timings_ns: Vec::with_capacity(24_000),
            traces: vec!["scenario\trate_hz\ttick\telapsed_s\twow_x\twow_y\twow_z\tintent_x\tintent_z\tgrounded\tjumped\tvel_x\tvel_y\tvel_z\tcontact\tobstacle_contact".into()],
            samples: vec!["scenario\trate_hz\tsample\ttick\telapsed_s\twow_x\twow_y\twow_z\tintent_x\tintent_z\tgrounded\tvel_x\tvel_y\tvel_z".into()],
            summaries: vec!["scenario\trate_hz\tticks\tgrounded_ticks\tairborne_ticks\tairborne_episodes\tjumps\tcontact_ticks\tobstacle_contact_ticks\tfinal_grounded\tfinal_wow_x\tfinal_wow_y\tfinal_wow_z\tfinal_vel_x\tfinal_vel_y\tfinal_vel_z\tmean_step_us\tmedian_step_us\tp95_step_us\tmax_step_us".into()],
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
    mut exit: MessageWriter<AppExit>,
) {
    if replay.finished {
        return;
    }
    if replay.scenario_index >= SCENARIOS.len() {
        if let Err(error) = write_outputs(&replay) {
            error!("ELW_EQ could not write replay outputs: {error:#}");
            exit.write(AppExit::error());
        } else {
            info!("ELW_EQ complete; output={}", output_dir().display());
            exit.write(AppExit::Success);
        }
        replay.finished = true;
        return;
    }

    let Some(progress) = progress else { return };
    if !progress.presentable() {
        let started = *replay.load_started.get_or_insert(real_time.elapsed_secs());
        if real_time.elapsed_secs() - started > LOAD_TIMEOUT_SECS {
            error!(
                "ELW_EQ world not ready: ready={}/{}, focus={}, colliders={}, placements={}, merge={}, gx={}",
                progress.ready,
                progress.total,
                progress.focus_resident,
                progress.colliders_pending,
                progress.placements_pending,
                progress.merge_pending,
                progress.gx_pending
            );
            replay.finished = true;
            exit.write(AppExit::error());
        }
        return;
    }

    if replay.tick == 0 && replay.elapsed == 0.0 {
        begin(&mut replay);
    }

    for _ in 0..MAX_STEPS_PER_UPDATE {
        if replay.elapsed >= total_secs(SCENARIOS[replay.scenario_index]) {
            let previous_rate = replay.rate_index;
            finish_scenario(&mut replay);
            if replay.scenario_index >= SCENARIOS.len() {
                break;
            }
            if replay.rate_index != previous_rate {
                begin(&mut replay);
                break;
            }
            begin(&mut replay);
        }

        let scenario = SCENARIOS[replay.scenario_index];
        let hz = [30_u16, 60, 120][replay.rate_index];
        let dt = 1.0 / f32::from(hz);
        let moving = replay.elapsed >= scenario.hold_secs && scenario.move_secs > 0.0;
        let move_elapsed = (replay.elapsed - scenario.hold_secs).max(0.0);
        let direction = wow_direction(scenario_direction_wow(scenario, move_elapsed));
        let speed = if moving { SPEED } else { 0.0 };
        let center = replay.state.pos + Vec3::Y * (HEIGHT * 0.5);
        let displacement = if moving {
            direction * (speed * dt)
        } else {
            Vec3::ZERO
        };
        let contact_hit = moving
            .then(|| collision.cast_mover(&replay.capsule, center, displacement, SKIN))
            .flatten();
        let contact = contact_hit.is_some();
        let obstacle_contact = contact_hit.is_some_and(|hit| hit.normal1.y.abs() < 0.7);
        if contact {
            replay.contact_ticks = replay.contact_ticks.saturating_add(1);
        }
        if obstacle_contact {
            replay.obstacle_contact_ticks = replay.obstacle_contact_ticks.saturating_add(1);
        }

        replay.sim_time.advance_by(Duration::from_secs_f32(dt));
        let timing_started = Instant::now();
        let want_jump =
            scenario.jump_once && replay.elapsed >= scenario.hold_secs && !replay.jump_requested;
        replay.jump_requested |= want_jump;
        let step_delta = replay.sim_time.delta();
        let elapsed_secs = replay.sim_time.elapsed_secs();
        let outcome = {
            let Replay { state, capsule, .. } = &mut *replay;
            advance(
                state,
                step_delta,
                elapsed_secs,
                &collision,
                CollisionConfig {
                    collider: capsule,
                    capsule_radius: RADIUS,
                    capsule_height: HEIGHT,
                    skin_width: SKIN,
                },
                MovementInput {
                    moving,
                    direction,
                    speed,
                    want_jump,
                    wire_jump: false,
                    knockback_velocity: None,
                    water_floor: None,
                    air_nudge_speed: SPEED,
                },
            )
        };
        replay
            .timings_ns
            .push(timing_started.elapsed().as_nanos() as u64);

        if outcome.grounded {
            replay.grounded = replay.grounded.saturating_add(1);
            replay.state.airborne_since = None;
        } else {
            if replay.state.airborne_since.is_none() {
                replay.airborne_episodes = replay.airborne_episodes.saturating_add(1);
            }
            replay.airborne = replay.airborne.saturating_add(1);
            replay.state.airborne_since.get_or_insert(elapsed_secs);
        }
        replay.final_grounded = outcome.grounded;
        if outcome.jumped {
            replay.jumps = replay.jumps.saturating_add(1);
        }

        replay.elapsed += dt;
        replay.tick = replay.tick.saturating_add(1);
        let wow = bevy_to_wow(replay.state.pos);
        let rate = hz;
        let trace = format!(
            "{}\t{}\t{}\t{:.9}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}",
            scenario.name,
            rate,
            replay.tick,
            replay.elapsed,
            wow[0],
            wow[1],
            wow[2],
            direction.x,
            direction.z,
            outcome.grounded,
            outcome.jumped,
            replay.state.horiz_vel.x,
            replay.state.vel_y,
            replay.state.horiz_vel.z,
            contact,
            obstacle_contact
        );
        replay.traces.push(trace);

        if replay.tick == 1
            || (replay.elapsed * 4.0) as usize > ((replay.elapsed - dt) * 4.0) as usize
        {
            let sample = replay.sample_count;
            replay.sample_count += 1;
            let sample_row = format!(
                "{}\t{}\t{}\t{}\t{:.9}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}",
                scenario.name,
                rate,
                sample,
                replay.tick,
                replay.elapsed,
                wow[0],
                wow[1],
                wow[2],
                direction.x,
                direction.z,
                outcome.grounded,
                replay.state.horiz_vel.x,
                replay.state.vel_y,
                replay.state.horiz_vel.z
            );
            replay.samples.push(sample_row);
        }
    }
}

fn begin(replay: &mut Replay) {
    let scenario = SCENARIOS[replay.scenario_index];
    replay.state = MovementState::default();
    replay.state.pos = wow_to_bevy(scenario.start_wow);
    replay.start = replay.state.pos;
    replay.tick = 0;
    replay.elapsed = 0.0;
    replay.sim_time = Time::default();
    replay.grounded = 0;
    replay.airborne = 0;
    replay.airborne_episodes = 0;
    replay.jumps = 0;
    replay.jump_requested = false;
    replay.final_grounded = false;
    replay.sample_count = 0;
    replay.contact_ticks = 0;
    replay.obstacle_contact_ticks = 0;
    replay.timings_ns.clear();
}

fn finish_scenario(replay: &mut Replay) {
    let scenario = SCENARIOS[replay.scenario_index];
    let mut timings = replay.timings_ns.clone();
    timings.sort_unstable();
    let mean = if timings.is_empty() {
        0.0
    } else {
        timings.iter().map(|value| *value as f64).sum::<f64>() / timings.len() as f64 / 1_000.0
    };
    let percentile = |p: f64| -> f64 {
        if timings.is_empty() {
            0.0
        } else {
            timings[((timings.len() - 1) as f64 * p).round() as usize] as f64 / 1_000.0
        }
    };
    let max = timings.last().copied().unwrap_or_default() as f64 / 1_000.0;
    let median = percentile(0.5);
    let p95 = percentile(0.95);
    let wow = bevy_to_wow(replay.state.pos);
    replay.summaries.push(format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
        scenario.name,
        [30, 60, 120][replay.rate_index],
        replay.tick,
        replay.grounded,
        replay.airborne,
        replay.airborne_episodes,
        replay.jumps,
        replay.contact_ticks,
        replay.obstacle_contact_ticks,
        replay.final_grounded,
        wow[0],
        wow[1],
        wow[2],
        replay.state.horiz_vel.x,
        replay.state.vel_y,
        replay.state.horiz_vel.z,
        mean,
        median,
        p95,
        max
    ));
    replay.scenario_index += 1;
    if replay.scenario_index == SCENARIOS.len() {
        replay.rate_index += 1;
        if replay.rate_index < 3 {
            replay.scenario_index = 0;
        }
    }
}

fn scenario_direction_wow(scenario: Scenario, elapsed: f32) -> [f32; 2] {
    match scenario.name {
        "fence_direct_approach" if elapsed >= 3.0 => [0.0, -1.0],
        "fence_parallel_and_recover" if elapsed < 0.25 => [0.0, 1.0],
        "fence_parallel_and_recover" if elapsed < 2.5 => [1.0, 0.0],
        "fence_parallel_and_recover" => [0.0, -1.0],
        _ => scenario.direction_wow,
    }
}

fn wow_direction(direction: [f32; 2]) -> Vec3 {
    Vec3::new(-direction[1], 0.0, -direction[0]).normalize_or_zero()
}

fn total_secs(scenario: Scenario) -> f32 {
    scenario.hold_secs + scenario.move_secs
}

fn output_dir() -> PathBuf {
    std::env::var_os("ELW_EQ_OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/elw-collision-014b1a-extracted"))
}

fn write_outputs(replay: &Replay) -> anyhow::Result<()> {
    let dir = output_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("extracted-trace.tsv"), replay.traces.join("\n"))?;
    fs::write(dir.join("extracted-samples.tsv"), replay.samples.join("\n"))?;
    fs::write(
        dir.join("extracted-summary.tsv"),
        replay.summaries.join("\n"),
    )?;
    fs::write(
        dir.join("run-info.txt"),
        format!(
            "Extracted benilla-movement replay\nCapsule radius={RADIUS}; height={HEIGHT}; skin={SKIN}\nRates=30/60/120 Hz\nWorld initialized by benilla-worldview and local WotLK data\nScenarios={} (20 fixed POC scenarios, pinned terrain_ledge, one supplementary jump/landing scenario)\n",
            SCENARIOS.len()
        ),
    )?;
    Ok(())
}
