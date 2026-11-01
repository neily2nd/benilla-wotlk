//! Reusable model-aware animation selection and playback.
//!
//! Consumers own actor intent and gameplay timing. This crate resolves that intent against the
//! loaded model and applies the shared Bevy playback semantics.

mod intent;
mod locomotion;
mod playback;
mod request;

pub use intent::{LocomotionSnapshot, VerticalMotion};
pub use locomotion::{
    base_candidates, claimed_candidates, gait_candidates, idle_candidates, locomotion_rate,
    route_one_shot, BaseCandidates, LocomotionFacts,
};
pub use playback::{
    apply_direction, play_clip, play_clip_with_blend, play_node_with_blend,
    play_upper_body_overlay, resolve_clip, roll_loop, roll_one_shot, ResolvedClip,
};

pub mod movement_flags {
    pub const FORWARD: u32 = 0x1;
    pub const BACKWARD: u32 = 0x2;
    pub const STRAFE_LEFT: u32 = 0x4;
    pub const STRAFE_RIGHT: u32 = 0x8;
    pub const TURN_LEFT: u32 = 0x10;
    pub const TURN_RIGHT: u32 = 0x20;
    pub const FALLING: u32 = 0x2000;
    pub const SWIMMING: u32 = 0x20_0000;
    /// Benilla's movement-state flying bit (the client maps its wire flag before selection).
    pub const FLYING: u32 = 0x100_0000;
    pub const LEVITATING: u32 = 0x400;
    pub const HOVER: u32 = 0x4000_0000;
    pub const ANY_MOVE: u32 = FORWARD | BACKWARD | STRAFE_LEFT | STRAFE_RIGHT;
    pub const ROUTE_COMMITTED_MOVE: u32 =
        FORWARD | BACKWARD | STRAFE_LEFT | STRAFE_RIGHT | TURN_LEFT | TURN_RIGHT | SWIMMING;
}
pub use request::{
    ActionDecision, ActionPriority, ActionRequest, AnimationActionState, InterruptionPolicy,
    PlaybackLayer,
};
