use super::*;

fn input(control: ControlInput) -> ControllerInput {
    ControllerInput {
        control,
        enabled: true,
        collision_ready: true,
        ..Default::default()
    }
}
fn flat_step(
    state: &mut MovementState,
    dt: Duration,
    _: f32,
    intent: MovementInput,
) -> MovementOutcome {
    state.pos += intent.direction * intent.speed * dt.as_secs_f32();
    MovementOutcome {
        grounded: true,
        ..Default::default()
    }
}

#[test]
fn strafing_moves_laterally_without_rotating_root_and_diagonals_have_one_speed() {
    for (forward, strafe) in [(1.0, 0.0), (-1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (1.0, 1.0)] {
        let mut actor = Controller::default();
        let control = ControlInput {
            forward,
            strafe,
            ..Default::default()
        };
        for _ in 0..60 {
            actor.advance_with(
                FIXED_STEP_SECONDS,
                input(control),
                ControllerConfig::default(),
                flat_step,
            );
        }
        assert_eq!(actor.facing(), 0.0);
        assert!((actor.position().length() - movement_speed(7.0, control)).abs() < 1e-4);
        if strafe != 0.0 {
            assert_eq!(actor.position().x.signum(), strafe);
        }
        if forward != 0.0 {
            assert_eq!(actor.position().z.signum(), -forward);
        }
    }
}

#[test]
fn fixed_step_position_and_arrow_facing_match_at_multiple_render_rates() {
    let simulate = |hz: usize| {
        let mut actor = Controller::default();
        for _ in 0..hz {
            actor.advance_with(
                1.0 / hz as f32,
                input(ControlInput {
                    forward: 1.0,
                    turn: 1.0,
                    ..Default::default()
                }),
                ControllerConfig::default(),
                flat_step,
            );
        }
        actor
    };
    let reference = simulate(60);
    for hz in [30, 120, 144, 240] {
        let actor = simulate(hz);
        assert!(
            actor.position().distance(reference.position()) < 1e-4,
            "{hz}"
        );
        assert!((actor.facing() - reference.facing()).abs() < 1e-5, "{hz}");
    }
}

#[test]
fn camera_facing_is_committed_before_same_frame_translation() {
    let mut actor = Controller::default();
    let mut request = input(ControlInput {
        forward: 1.0,
        mouse_look: true,
        turn: 1.0,
        ..Default::default()
    });
    request.camera_facing = Some(std::f32::consts::FRAC_PI_2);
    let frame = actor.advance_with(
        FIXED_STEP_SECONDS,
        request,
        ControllerConfig::default(),
        flat_step,
    );
    assert_eq!(frame.keyboard_yaw, 0.0);
    assert!(actor.position().x < -0.1 && actor.position().z.abs() < 1e-6);
    assert_eq!(actor.facing(), request.camera_facing.unwrap());
}

#[test]
fn zero_step_frames_preserve_motion_release_clears_it_and_collision_blocked_steps_idle() {
    let mut actor = Controller::default();
    let request = input(ControlInput {
        strafe: 1.0,
        ..Default::default()
    });
    actor.advance_with(
        FIXED_STEP_SECONDS,
        request,
        ControllerConfig::default(),
        flat_step,
    );
    let facts = actor.locomotion();
    assert_eq!(
        actor
            .advance_with(0.001, request, ControllerConfig::default(), flat_step)
            .steps,
        0
    );
    assert_eq!(actor.locomotion(), facts);
    actor.advance_with(
        0.001,
        input(ControlInput::default()),
        ControllerConfig::default(),
        flat_step,
    );
    assert_eq!(actor.locomotion().speed, 0.0);
    actor.advance_with(
        FIXED_STEP_SECONDS,
        request,
        ControllerConfig::default(),
        |_, _, _, _| MovementOutcome {
            grounded: true,
            ..Default::default()
        },
    );
    assert!(actor.locomotion().physically_blocked);
    assert_eq!(actor.locomotion().flags, 0);
}

#[test]
fn jump_is_queued_once_and_airborne_strafe_retains_facing_and_landing_edge() {
    let mut actor = Controller::default();
    let mut request = input(ControlInput {
        strafe: -1.0,
        jumping: true,
        ..Default::default()
    });
    actor.advance_with(
        FIXED_STEP_SECONDS * 0.5,
        request,
        ControllerConfig::default(),
        flat_step,
    );
    request.control.jumping = false;
    let mut jump_requests = 0;
    let frame = actor.advance_with(
        FIXED_STEP_SECONDS * 2.5,
        request,
        ControllerConfig::default(),
        |state, dt, t, intent| {
            jump_requests += usize::from(intent.want_jump);
            flat_step(state, dt, t, intent);
            state.vel_y = 1.0;
            MovementOutcome {
                jumped: intent.want_jump,
                ..Default::default()
            }
        },
    );
    assert!(frame.jumped);
    assert_eq!(jump_requests, 1);
    assert!(actor.airborne());
    assert_eq!(actor.facing(), 0.0);
    assert_eq!(actor.vertical_motion(), VerticalMotion::Jumping);
    actor.state.vel_y = -1.0;
    actor.advance_with(
        FIXED_STEP_SECONDS,
        request,
        ControllerConfig::default(),
        |_, _, _, _| MovementOutcome::default(),
    );
    assert_eq!(actor.vertical_motion(), VerticalMotion::Falling);
    assert!(
        actor
            .advance_with(
                FIXED_STEP_SECONDS,
                request,
                ControllerConfig::default(),
                flat_step
            )
            .landed
    );
    assert!(
        !actor
            .advance_with(
                FIXED_STEP_SECONDS,
                request,
                ControllerConfig::default(),
                flat_step
            )
            .landed
    );
}

#[test]
fn focus_pause_or_collision_hold_drops_pending_jump_and_reset_clears_run_state() {
    for disabled in [true, false] {
        let mut actor = Controller::default();
        actor.advance_with(
            0.001,
            input(ControlInput {
                jumping: true,
                ..Default::default()
            }),
            ControllerConfig::default(),
            flat_step,
        );
        let mut request = input(ControlInput::default());
        request.enabled = !disabled;
        request.collision_ready = disabled;
        actor.advance_with(2.0, request, ControllerConfig::default(), |_, _, _, _| {
            panic!("suspended controller stepped")
        });
        assert_eq!(actor.accumulator, 0.0);
        assert!(!actor.queued_jump);
        actor.reset(Vec3::new(1.0, 2.0, 3.0), 0.5);
        assert_eq!(actor.position(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(actor.facing(), 0.5);
        assert!(!actor.airborne());
        assert_eq!(actor.locomotion(), LocomotionFacts::default());
    }
}
