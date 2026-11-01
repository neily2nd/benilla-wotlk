use benilla_assets::AnimClip;

use crate::movement_flags;

const STAND: u16 = 0;
const SHUFFLE_LEFT: u16 = 11;
const SHUFFLE_RIGHT: u16 = 12;
const STEALTH_WALK: u16 = 119;
const STEALTH_STAND: u16 = 120;
const HOVER_ANIM: u16 = 193;
const EMOTE_EAT: u16 = 61;
const FAST_RUN_SPEED: f32 = 11.0;
const RATE_SCALED: &[u16] = &[4, 5, 11, 12, 13, 37, 38, 39, 42, 43, 44, 45, 135, 143, 187];

/// Per-frame movement facts supplied by an application's movement adapter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LocomotionFacts {
    pub speed: f32,
    pub flags: u32,
    pub walk_speed: f32,
    pub stand_state: u8,
    pub stealthed: bool,
    pub flying: bool,
    /// True when the movement owner attempted a step but collision accepted no displacement.
    pub physically_blocked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseCandidates {
    Fixed(&'static [u16]),
    State([u16; 2]),
}

impl BaseCandidates {
    pub fn as_slice(&self) -> &[u16] {
        match self {
            Self::Fixed(candidates) => candidates,
            Self::State(candidates) => candidates,
        }
    }
}

/// The client's locomotion resolver, without its network-owned input acquisition.
pub fn claimed_candidates(
    f: LocomotionFacts,
    ready: Option<u16>,
    ranged_load: Option<u16>,
) -> Option<&'static [u16]> {
    use movement_flags::*;
    let flags = f.flags;
    let swimming = flags & (SWIMMING | FLYING) != 0;
    if swimming && flags & ANY_MOVE != 0 {
        return Some(if flags & (TURN_LEFT | TURN_RIGHT) != 0 {
            &[41, 0]
        } else if flags & STRAFE_LEFT != 0 {
            &[43, 42, 41, 0]
        } else if flags & STRAFE_RIGHT != 0 {
            &[44, 42, 41, 0]
        } else if flags & BACKWARD != 0 {
            &[45, 41, 0]
        } else {
            &[42, 41, 0]
        });
    }
    if f.flying {
        return Some(&[135, 0]);
    }
    if flags & BACKWARD != 0 {
        return Some(&[13, 4, 0]);
    }
    if f.stealthed && flags & ANY_MOVE != 0 {
        return Some(&[STEALTH_WALK, 4, 0]);
    }
    if flags & ANY_MOVE != 0 {
        return Some(if f.speed >= FAST_RUN_SPEED {
            &[143, 5, 4, 0]
        } else if f.speed > 2.0 * f.walk_speed {
            &[5, 4, 0]
        } else {
            &[4, 0]
        });
    }
    if !swimming && flags & TURN_LEFT != 0 {
        return Some(&[SHUFFLE_LEFT, STAND]);
    }
    if !swimming && flags & TURN_RIGHT != 0 {
        return Some(&[SHUFFLE_RIGHT, STAND]);
    }
    if let Some(ready) = ready {
        return Some(match ready {
            _ if swimming => &[41, 0],
            26 => &[26, 25, 0],
            27 => &[27, 25, 0],
            28 => &[28, 25, 0],
            _ => &[25, 0],
        });
    }
    if let Some(load) = ranged_load {
        return Some(match load {
            105 => &[105, 25, 0],
            106 => &[106, 25, 0],
            109 => &[109, 105, 25, 0],
            110 => &[110, 106, 25, 0],
            111 => &[111, 25, 0],
            112 => &[112, 25, 0],
            _ => &[25, 0],
        });
    }
    match f.stand_state {
        4 => Some(&[102, 0]),
        5 => Some(&[103, 0]),
        6 => Some(&[104, 0]),
        _ => None,
    }
}

/// Last-resort standing animation candidates, including swimming, stealth and hover states.
pub fn idle_candidates(f: LocomotionFacts) -> &'static [u16] {
    use movement_flags::*;
    if f.flags & (SWIMMING | FLYING) != 0 {
        &[41, 0]
    } else if f.stealthed {
        &[STEALTH_STAND, STAND]
    } else if f.flags
        & (if f.flying {
            movement_flags::HOVER
        } else {
            LEVITATING | movement_flags::HOVER
        })
        != 0
    {
        &[HOVER_ANIM, STAND]
    } else {
        &[STAND]
    }
}

/// The pick with no state emote.
pub fn gait_candidates(
    f: LocomotionFacts,
    ready: Option<u16>,
    ranged_load: Option<u16>,
) -> &'static [u16] {
    claimed_candidates(f, ready, ranged_load).unwrap_or_else(|| idle_candidates(f))
}

/// Full base pick: locomotion, then a supplied state emote, then the model-appropriate idle fallback.
pub fn base_candidates(
    f: LocomotionFacts,
    ready: Option<u16>,
    ranged_load: Option<u16>,
    state_anim: Option<u16>,
) -> BaseCandidates {
    if let Some(candidates) = claimed_candidates(f, ready, ranged_load) {
        return BaseCandidates::Fixed(candidates);
    }
    match state_anim {
        Some(anim) => BaseCandidates::State([anim, STAND]),
        None => BaseCandidates::Fixed(idle_candidates(f)),
    }
}

/// The client's one-shot track routing table: upper-body while lower-body state is committed.
pub fn route_one_shot(id: u16, flags: u32, stand_state: u8) -> crate::PlaybackLayer {
    use movement_flags::*;
    let class_a = matches!(id,
        2 | 8..=10 | 14..=36 | 46..=49 | 51..=74 | 76..=78 | 80..=90 | 105..=113 | 117 | 118
        | 122..=125 | 128..=130 | 133 | 134 | 136..=138 | 185 | 186 | 195);
    let combat = matches!(id, 10 | 16..=24 | 30 | 36 | 57..=59 | 85..=88 | 95 | 117 | 118);
    let forced_full = matches!(id, 1 | 6 | 131 | 132 | 57 | 58 | 118);
    if id == EMOTE_EAT {
        crate::PlaybackLayer::UpperBody
    } else if forced_full || !class_a {
        crate::PlaybackLayer::FullBody
    } else if flags & ROUTE_COMMITTED_MOVE != 0
        || stand_state != 0
        || (combat && flags & FALLING != 0)
    {
        crate::PlaybackLayer::UpperBody
    } else {
        crate::PlaybackLayer::FullBody
    }
}

/// The client's speed/sequence rate rule. Non-locomotion clips return `None` and retain their own rate.
pub fn locomotion_rate(clip: &AnimClip, speed: f32, model_scale: f32) -> Option<f32> {
    let divisor = clip.move_speed * model_scale.abs();
    (divisor > 0.0 && RATE_SCALED.contains(&clip.anim_id)).then(|| speed / divisor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement_flags::*;

    #[test]
    fn ground_gait_candidates_match_walk_run_sprint_and_backpedal_rules() {
        assert_eq!(
            gait_candidates(
                LocomotionFacts {
                    flags: FORWARD,
                    speed: 2.5,
                    walk_speed: 2.5,
                    ..LocomotionFacts::default()
                },
                None,
                None
            ),
            &[4, 0]
        );
        assert_eq!(
            gait_candidates(
                LocomotionFacts {
                    flags: FORWARD,
                    speed: 7.5,
                    walk_speed: 2.5,
                    ..LocomotionFacts::default()
                },
                None,
                None
            ),
            &[5, 4, 0]
        );
        assert_eq!(
            gait_candidates(
                LocomotionFacts {
                    flags: FORWARD,
                    speed: 11.0,
                    walk_speed: 2.5,
                    ..LocomotionFacts::default()
                },
                None,
                None
            ),
            &[143, 5, 4, 0]
        );
        assert_eq!(
            gait_candidates(
                LocomotionFacts {
                    flags: BACKWARD,
                    ..LocomotionFacts::default()
                },
                None,
                None
            ),
            &[13, 4, 0]
        );
    }

    #[test]
    fn idle_turn_and_ready_candidates_are_stable() {
        assert_eq!(
            gait_candidates(LocomotionFacts::default(), None, None),
            &[0]
        );
        assert_eq!(
            gait_candidates(
                LocomotionFacts {
                    flags: TURN_LEFT,
                    ..LocomotionFacts::default()
                },
                None,
                None
            ),
            &[11, 0]
        );
        assert_eq!(
            gait_candidates(LocomotionFacts::default(), Some(27), None),
            &[27, 25, 0]
        );
        assert_eq!(
            gait_candidates(
                LocomotionFacts {
                    flags: SWIMMING | FORWARD,
                    ..LocomotionFacts::default()
                },
                None,
                None
            ),
            &[42, 41, 0]
        );
    }

    #[test]
    fn client_one_shot_route_preserves_full_body_exceptions() {
        assert_eq!(
            route_one_shot(17, FORWARD, 0),
            crate::PlaybackLayer::UpperBody
        );
        assert_eq!(route_one_shot(17, 0, 0), crate::PlaybackLayer::FullBody);
        assert_eq!(
            route_one_shot(57, FORWARD, 0),
            crate::PlaybackLayer::FullBody
        );
        assert_eq!(route_one_shot(61, 0, 0), crate::PlaybackLayer::UpperBody);
    }
}
