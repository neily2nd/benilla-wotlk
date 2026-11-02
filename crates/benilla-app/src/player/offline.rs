//! Offline adapter for the reusable Benilla player movement, heading, camera and animation code.
//! No network plugin is installed; the prototype owns one local actor and one camera.

use avian3d::prelude::Collider;
use benilla_assets::{LockRecover, ModelAnimations, WorldAssets};
use benilla_formats::{load_anim_data_catalog, AnimDataCatalog};
use benilla_player_control::{
    decode_input, locomotion_facts, movement_direction, ControlBindings, ControlInput,
};
use bevy::animation::transition::AnimationTransitions;
use bevy::animation::RepeatAnimation;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::{
    gait, mover, CameraControl, Player, PlayerCapsule, CAPSULE_HEIGHT, CAPSULE_RADIUS, TURN_RATE,
};

const GOLDSHIRE_START: [f32; 3] = [-9458.89, 58.385, 104.918];
const RUN_SPEED: f32 = 7.0;
const WALK_SPEED: f32 = 2.5;
const MAX_FRAME_HISTORY: usize = 600;

#[derive(Component)]
pub(crate) struct OfflinePlayer;

#[derive(Component, Default)]
struct OfflineAnimation {
    base_clip: Option<u16>,
    attack_remaining: f32,
}

#[derive(Resource)]
struct OfflineAnimData(AnimDataCatalog);

#[derive(Resource, Clone, Copy, Default)]
pub(crate) struct OfflineBindings(pub ControlBindings);

#[derive(Resource)]
struct OfflineCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for OfflineCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.35,
            distance: 6.0,
        }
    }
}

#[derive(Resource, Default)]
struct OfflineDiagnostics {
    elapsed: f32,
    frames: u32,
    sum_ms: f64,
    max_ms: f32,
    history: Vec<f32>,
}

pub(crate) struct OfflinePlayerPlugin;

impl Plugin for OfflinePlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OfflineBindings>()
            .init_resource::<OfflineCamera>()
            .init_resource::<OfflineDiagnostics>()
            .insert_resource(PlayerCapsule(Collider::capsule(
                CAPSULE_RADIUS,
                CAPSULE_HEIGHT - 2.0 * CAPSULE_RADIUS,
            )))
            .insert_resource(Player::default())
            .insert_resource(CameraControl {
                distance: 6.0,
                target_distance: 6.0,
                collision_distance: 6.0,
                self_fade_alpha: 1.0,
                ..default()
            })
            .add_plugins(crate::entities::OfflineEntitiesPlugin)
            .add_systems(
                Startup,
                (load_anim_data, spawn_offline_player).after(benilla_assets::AssetSet::Open),
            )
            .add_systems(
                Update,
                (
                    drive_offline_player,
                    update_offline_camera,
                    play_offline_animation,
                )
                    .chain()
                    .in_set(benilla_world::schedule::WorldStage::Input),
            )
            .add_systems(
                Update,
                report_frame_time.after(benilla_world::schedule::WorldStage::Present),
            );
    }
}

fn load_anim_data(mut commands: Commands, world_assets: Option<Res<WorldAssets>>) {
    let catalog = world_assets
        .and_then(|assets| {
            let mut chain = assets.chain.lock_recover();
            load_anim_data_catalog(&mut chain).ok()
        })
        .unwrap_or_else(|| AnimDataCatalog::from_rows([]));
    commands.insert_resource(OfflineAnimData(catalog));
}

fn spawn_offline_player(mut commands: Commands, mut player: ResMut<Player>) {
    let position = benilla_assets::coords::wow_to_bevy(GOLDSHIRE_START);
    player.pos = position;
    commands.spawn((
        crate::net::NetEntity {
            kind: benilla_protocol::EntityKind::Unit,
            display_id: Some(49), // CreatureDisplayInfo for the local HumanMale model.
            scale: 1.0,
        },
        crate::net::ObjectStore::default(),
        crate::net::Guid(0x5753_504f_4300_0001),
        OfflinePlayer,
        OfflineAnimation::default(),
        crate::creature_anim::MovementState::default(),
        benilla_world::world_unit::WorldUnit {
            wades: true,
            scale: 1.0,
            height: CAPSULE_HEIGHT,
            bound: None,
        },
        Transform::from_translation(position),
        Visibility::default(),
    ));
    info!("offline player spawned at Goldshire; waiting for world collision and model assets");
}

#[allow(clippy::too_many_arguments)]
fn drive_offline_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<OfflineBindings>,
    progress: Option<Res<benilla_world::terrain_stream::WorldLoadProgress>>,
    collision: benilla_world::collision::WorldCollision,
    capsule: Res<PlayerCapsule>,
    mut player: ResMut<Player>,
    mut camera: ResMut<OfflineCamera>,
    mut bodies: Query<
        (&mut Transform, &mut crate::creature_anim::MovementState),
        With<OfflinePlayer>,
    >,
) {
    let Ok((mut transform, mut movement)) = bodies.single_mut() else {
        return;
    };
    let input = decode_input(&keys, &buttons, bindings.0);
    let delta = time.delta_secs();
    if !delta.is_finite() || delta <= 0.0 {
        return;
    }

    let ready = collision_ready(progress.as_deref());
    if !ready {
        movement.flags = 0;
        movement.speed = 0.0;
        return;
    }

    let moving = input.forward != 0.0 || input.strafe != 0.0;
    let turn_rate = TURN_RATE;
    if input.turn != 0.0 && !input.mouse_look {
        player.face_yaw += benilla_player_control::keyboard_turn_delta(
            input,
            delta,
            turn_rate,
            player.airborne_since.is_some(),
        );
        camera.yaw = player.face_yaw;
    }

    let mut flags = 0;
    if input.forward > 0.0 {
        flags |= crate::creature_anim::move_flags::FORWARD;
    } else if input.forward < 0.0 {
        flags |= crate::creature_anim::move_flags::BACKWARD;
    }
    if input.strafe < 0.0 {
        flags |= crate::creature_anim::move_flags::STRAFE_LEFT;
    } else if input.strafe > 0.0 {
        flags |= crate::creature_anim::move_flags::STRAFE_RIGHT;
    }
    if input.walking {
        flags |= crate::creature_anim::move_flags::WALK_MODE;
    }
    if input.turn > 0.0 {
        flags |= crate::creature_anim::move_flags::TURN_LEFT;
    } else if input.turn < 0.0 {
        flags |= crate::creature_anim::move_flags::TURN_RIGHT;
    }

    let direction = movement_direction(
        player.face_yaw,
        ControlInput {
            forward: input.forward,
            strafe: input.strafe,
            ..Default::default()
        },
    );
    let speed = movement_speed(input);
    let outcome = mover::step(
        &mut player,
        &time,
        &collision,
        &capsule.0,
        moving,
        direction,
        speed,
        input.jumping,
        false,
        None,
        None,
        WALK_SPEED,
    );

    let airborne = player.airborne_since.is_some();
    let turn_flags = if input.turn > 0.0 {
        crate::creature_anim::move_flags::TURN_LEFT
    } else if input.turn < 0.0 {
        crate::creature_anim::move_flags::TURN_RIGHT
    } else {
        0
    };
    flags = gait::drive_body_heading(
        &mut player,
        flags | turn_flags,
        delta,
        false,
        moving && outcome.ground.is_some(),
        airborne,
        input.turn != 0.0 || input.mouse_look,
        turn_rate,
    );
    transform.translation = player.pos;
    transform.rotation = Quat::from_rotation_y(player.model_yaw);
    movement.flags = flags;
    movement.speed = if outcome.grounded && moving && !player.modes.rooted {
        speed
    } else {
        0.0
    };
    movement.vertical_speed = player.vel_y;
}

fn movement_speed(input: ControlInput) -> f32 {
    benilla_player_control::movement_speed(RUN_SPEED, input)
}

fn update_offline_camera(
    motion: Res<AccumulatedMouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    collision: benilla_world::collision::WorldCollision,
    mut player: ResMut<Player>,
    mut camera: ResMut<OfflineCamera>,
    mut transforms: Query<
        &mut Transform,
        (
            With<benilla_world::view::WorldCamera>,
            Without<OfflinePlayer>,
        ),
    >,
) {
    let looking = buttons.pressed(MouseButton::Right);
    if let Ok((window, mut cursor)) = windows.single_mut() {
        if window.focused {
            cursor.grab_mode = if looking {
                CursorGrabMode::Locked
            } else {
                CursorGrabMode::None
            };
            cursor.visible = !looking;
        }
    }
    if looking {
        let (dyaw, dpitch) = benilla_player_control::mouse_look_deltas(
            Vec3::new(motion.delta.x, motion.delta.y, 0.0),
            0.003,
            0.0015,
            false,
        );
        camera.yaw += dyaw;
        camera.pitch = (camera.pitch + dpitch).clamp(-1.45, 1.45);
    }
    for event in wheel.read() {
        camera.distance = (camera.distance - event.y * 0.5).clamp(2.5, 12.0);
    }
    if looking {
        // Right-drag rotates the camera and commits the same heading to the local actor.
        player.face_yaw = camera.yaw;
    }
    if let Ok(mut transform) = transforms.single_mut() {
        let pivot = player.pos + Vec3::Y * (CAPSULE_HEIGHT - CAPSULE_RADIUS);
        let rotation = Quat::from_rotation_y(camera.yaw) * Quat::from_rotation_x(camera.pitch);
        let desired = rotation * Vec3::new(0.0, 0.0, camera.distance);
        let distance = collision
            .cast_camera(pivot, desired, false)
            .map_or(camera.distance, |hit| hit.min(camera.distance).max(0.5));
        let offset = rotation * Vec3::new(0.0, 0.0, distance);
        *transform = Transform::from_translation(pivot + offset).looking_at(pivot, Vec3::Y);
    }
}

fn play_offline_animation(
    time: Res<Time>,
    catalog: Option<Res<OfflineAnimData>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut units: Query<
        (
            &ModelAnimations,
            &mut AnimationPlayer,
            &mut AnimationTransitions,
            &crate::creature_anim::MovementState,
            &mut OfflineAnimation,
        ),
        With<OfflinePlayer>,
    >,
) {
    let Ok((animations, mut player, mut transitions, motion, mut state)) = units.single_mut()
    else {
        return;
    };
    let Some(catalog) = catalog else { return };
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(clip) = animations.find(animations.resolve(16, &catalog.0).id) {
            benilla_animation::play_clip(
                &mut transitions,
                &mut player,
                clip,
                RepeatAnimation::Never,
                1.0,
            );
            state.attack_remaining = clip.duration;
            state.base_clip = None;
            return;
        }
    }
    state.attack_remaining = (state.attack_remaining - time.delta_secs()).max(0.0);
    if state.attack_remaining > 0.0 {
        return;
    }
    let facts = locomotion_facts(motion.speed, motion.flags, WALK_SPEED);
    for requested in benilla_animation::gait_candidates(facts, None, None) {
        let resolved = animations.resolve(*requested, &catalog.0);
        let Some(clip) = animations.find(resolved.id) else {
            continue;
        };
        if state.base_clip == Some(clip.anim_id) {
            return;
        }
        let rate = benilla_animation::locomotion_rate(clip, motion.speed, 1.0).unwrap_or(1.0);
        let repeat = if clip.looping {
            RepeatAnimation::Forever
        } else {
            RepeatAnimation::Never
        };
        benilla_animation::play_clip(
            &mut transitions,
            &mut player,
            clip,
            repeat,
            benilla_assets::ResolvedAnim {
                id: clip.anim_id,
                dir_flags: resolved.dir_flags,
            }
            .rate(rate),
        );
        state.base_clip = Some(clip.anim_id);
        return;
    }
}

fn report_frame_time(time: Res<Time<Real>>, mut diagnostics: ResMut<OfflineDiagnostics>) {
    let ms = (time.delta_secs_f64() * 1000.0) as f32;
    diagnostics.elapsed += time.delta_secs();
    diagnostics.frames += 1;
    diagnostics.sum_ms += ms as f64;
    diagnostics.max_ms = diagnostics.max_ms.max(ms);
    if diagnostics.history.len() < MAX_FRAME_HISTORY {
        diagnostics.history.push(ms);
    }
    if std::env::var_os("WS_POC_FPS").is_some() && diagnostics.elapsed >= 2.0 {
        let average = diagnostics.sum_ms / f64::from(diagnostics.frames.max(1));
        let fps = if average > 0.0 { 1000.0 / average } else { 0.0 };
        info!(
            "WS_POC_FRAME avg={average:.2}ms fps={fps:.1} max={:.2}ms samples={}",
            diagnostics.max_ms,
            diagnostics.history.len()
        );
        diagnostics.elapsed = 0.0;
        diagnostics.frames = 0;
        diagnostics.sum_ms = 0.0;
        diagnostics.max_ms = 0.0;
    }
}

fn collision_ready(progress: Option<&benilla_world::terrain_stream::WorldLoadProgress>) -> bool {
    progress.is_some_and(|p| {
        p.scene_ready && p.focus_resident && p.pinned_collision_ready && p.colliders_pending == 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_waits_for_focus_scene_and_all_collision_to_be_ready() {
        assert!(!collision_ready(None));
        let mut progress = benilla_world::terrain_stream::WorldLoadProgress {
            scene_ready: true,
            focus_resident: true,
            ..Default::default()
        };
        assert!(!collision_ready(Some(&progress)), "pins still resolving");
        progress.pinned_collision_ready = true;
        progress.colliders_pending = 1;
        assert!(
            !collision_ready(Some(&progress)),
            "a collider is still pending"
        );
        progress.colliders_pending = 0;
        assert!(collision_ready(Some(&progress)));
    }

    #[test]
    fn a_new_actor_starts_with_no_attack_or_previous_gait() {
        let animation = OfflineAnimation::default();
        assert_eq!(animation.base_clip, None);
        assert_eq!(animation.attack_remaining, 0.0);
    }

    #[test]
    fn movement_mode_selects_the_prototype_walk_run_and_backpedal_speeds() {
        assert_eq!(movement_speed(ControlInput::default()), RUN_SPEED);
        assert_eq!(
            movement_speed(ControlInput {
                walking: true,
                forward: -1.0,
                ..Default::default()
            }),
            WALK_SPEED
        );
        assert_eq!(
            movement_speed(ControlInput {
                forward: -1.0,
                ..Default::default()
            }),
            4.5
        );
    }
}
