//! The rendered body heading and the animation's view of the flags, the reference's display facing.
//! [`super::control`] calls [`drive_body_heading`] once per controlled frame, after the move flags
//! are final.

use benilla_player_control::{advance_heading, ControlInput, Heading};

use super::Player;

/// Advances `model_yaw` one frame and returns the animation's view of the move flags (the wire
/// keeps the real key flags). Strafing eases the body toward `face_yaw ± 90°` (`± 45°` diagonal,
/// mirrored backpedaling) by a quarter of the gap a frame in aim-relative offset space, so a
/// left-right flip swings round the front. Moving forward or back or airborne (`flags & 0x2003`),
/// or swimming, snaps it to the aim; standing runs the frozen chase, holding the 90° ceiling while
/// the aim is steered, then sweeping onto it at 8× the turn rate. The swim pitch is applied at the
/// transform write. The turn-in-place shuffle keys on the body's own rotation (chase-step bits
/// `0x800`/`0x1000`), not the turn keys, so a deck's turn never shuffles.
pub(super) fn drive_body_heading(
    player: &mut Player,
    move_flags_now: u32,
    dt: f32,
    swimming: bool,
    moving: bool,
    airborne: bool,
    steering: bool,
    // The mover's own turn rate in rad/s: a possessed creature settles at its own pace.
    turn_rate: f32,
) -> u32 {
    let mut heading = Heading {
        facing: player.face_yaw,
        body: player.model_yaw,
    };
    let anim_flags = advance_heading(
        &mut heading,
        ControlInput::default(),
        dt,
        turn_rate,
        super::STATIONARY_CHASE_RATE,
        move_flags_now,
        swimming,
        moving,
        airborne,
        steering,
    );
    player.face_yaw = heading.facing;
    player.model_yaw = heading.body;
    anim_flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standing_body_sweeps_back_at_the_movers_own_rate() {
        let sweep = |turn_rate: f32| {
            let mut player = Player {
                face_yaw: 1.0,
                model_yaw: 0.0,
                ..Default::default()
            };
            drive_body_heading(&mut player, 0, 0.01, false, false, false, false, turn_rate);
            player.model_yaw
        };
        // Below the 90° ceiling the whole step is the sweep, so it scales with the rate.
        let slow = sweep(std::f32::consts::PI / 4.0);
        let fast = sweep(std::f32::consts::PI);
        assert!(slow > 0.0 && fast > slow);
        assert!(
            (fast / slow - 4.0).abs() < 1e-3,
            "a 4× turn rate sweeps 4× as far in a frame: {slow} vs {fast}"
        );
        // Steering freezes the chase whatever the rate.
        let mut player = Player {
            face_yaw: 1.0,
            model_yaw: 0.0,
            ..Default::default()
        };
        drive_body_heading(
            &mut player,
            0,
            0.01,
            false,
            false,
            false,
            true,
            std::f32::consts::PI,
        );
        assert_eq!(
            player.model_yaw, 0.0,
            "under the 90° ceiling a steering frame moves the body not at all"
        );
    }
}
