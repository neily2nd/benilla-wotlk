use std::time::Duration;

use benilla_assets::{AnimClip, AnimRng, ModelAnimations, ResolvedAnim};
use benilla_formats::AnimDataCatalog;
use bevy::animation::graph::AnimationNodeIndex;
use bevy::animation::transition::AnimationTransitions;
use bevy::animation::RepeatAnimation;
use bevy::prelude::AnimationPlayer;

#[derive(Clone, Copy, Debug)]
pub struct ResolvedClip {
    pub requested_id: u16,
    pub resolved: ResolvedAnim,
    pub clip_node: AnimationNodeIndex,
}

pub fn resolve_clip(
    animations: &ModelAnimations,
    requested_id: u16,
    catalog: &AnimDataCatalog,
) -> Option<ResolvedClip> {
    let resolved = animations.resolve(requested_id, catalog);
    let clip = animations.find(resolved.id)?;
    Some(ResolvedClip {
        requested_id,
        resolved,
        clip_node: clip.node,
    })
}

pub fn apply_direction(
    player: &mut AnimationPlayer,
    clip: &AnimClip,
    resolved: ResolvedAnim,
    rate: f32,
) {
    if resolved.dir_flags != 0 {
        if let Some(active) = player.animation_mut(clip.node) {
            resolved.apply(active, rate, clip.duration);
        }
    }
}

/// Play an already selected clip with the same blend, repeat and rate semantics used by the client.
pub fn play_clip(
    transitions: &mut AnimationTransitions,
    player: &mut AnimationPlayer,
    clip: &AnimClip,
    repeat: RepeatAnimation,
    rate: f32,
) {
    play_clip_with_blend(
        transitions,
        player,
        clip,
        repeat,
        rate,
        Duration::from_secs_f32(clip.blend_time.max(0.0)),
    );
}

pub fn play_clip_with_blend(
    transitions: &mut AnimationTransitions,
    player: &mut AnimationPlayer,
    clip: &AnimClip,
    repeat: RepeatAnimation,
    rate: f32,
    blend: Duration,
) {
    play_node_with_blend(transitions, player, clip.node, repeat, rate, blend);
}

/// Play a graph node with the client's cross-fade, repeat and playback-rate semantics.
///
/// The node form is useful to adapters that have already validated and indexed a model graph.
pub fn play_node_with_blend(
    transitions: &mut AnimationTransitions,
    player: &mut AnimationPlayer,
    node: AnimationNodeIndex,
    repeat: RepeatAnimation,
    rate: f32,
    blend: Duration,
) {
    let active = transitions.play(player, node, blend);
    active.set_repeat(repeat);
    active.set_speed(rate);
}

/// Start a validated M2 upper-body mask node while leaving the base locomotion track active.
///
/// This is the same key-bone overlay primitive used by the Benilla creature driver; clip identity,
/// interruption, and cleanup remain owned by the caller.
pub fn play_upper_body_overlay(
    player: &mut AnimationPlayer,
    node: AnimationNodeIndex,
    seek_seconds: f32,
    rate: f32,
    weight: f32,
) {
    let active = player.play(node);
    active.replay();
    active.set_repeat(RepeatAnimation::Never);
    active.seek_to(seek_seconds.max(0.0));
    active.set_speed(rate);
    active.set_weight(weight.max(0.0));
}

pub fn roll_one_shot<'a>(
    animations: &'a ModelAnimations,
    head: &'a AnimClip,
    rng: &mut AnimRng,
) -> (&'a AnimClip, RepeatAnimation) {
    let clip = animations
        .pick_variation(head.anim_id, rng.draw())
        .unwrap_or(head);
    let repeat = match rng.replay_count(clip.replay) {
        count if count > 1 => RepeatAnimation::Count(count),
        _ => RepeatAnimation::Never,
    };
    (clip, repeat)
}

pub fn roll_loop<'a>(
    animations: &'a ModelAnimations,
    head: &'a AnimClip,
    relaxed: bool,
    rng: &mut AnimRng,
) -> (&'a AnimClip, u32) {
    let clip = if relaxed {
        animations
            .pick_variation(head.anim_id, rng.draw())
            .unwrap_or(head)
    } else {
        head
    };
    let replay_count = rng.replay_count(clip.replay);
    (clip, replay_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_formats::{AnimDataCatalog, PlayableAnim};

    fn clip(anim_id: u16, node: usize) -> AnimClip {
        AnimClip {
            anim_id,
            seq_index: 0,
            node: AnimationNodeIndex::new(node),
            looping: true,
            duration: 1.0,
            move_speed: 7.0,
            blend_time: 0.0,
            bounds_center: bevy::prelude::Vec3::ZERO,
            bounds_radius: 0.0,
            bounds_min: bevy::prelude::Vec3::ZERO,
            bounds_max: bevy::prelude::Vec3::ZERO,
            events: Vec::new().into(),
            arm_nodes: None,
            upper_node: None,
            frequency: 0,
            replay: (0, 0),
            poses_bones: true,
        }
    }

    fn model(
        clips: Vec<AnimClip>,
        playable_animation_lookup: Vec<PlayableAnim>,
    ) -> ModelAnimations {
        ModelAnimations {
            graph: Default::default(),
            clips,
            hand_close: [None, None],
            playable_animation_lookup,
            animation_lookup: Vec::new(),
            global_bones: Vec::new(),
            first_seq: None,
            pose: Default::default(),
        }
    }

    #[test]
    fn clip_resolution_uses_model_fallback_and_returns_none_for_missing_models() {
        let mut playable = vec![
            PlayableAnim {
                resolved_id: 0,
                dir_flags: 0
            };
            19
        ];
        playable[18] = PlayableAnim {
            resolved_id: 16,
            dir_flags: 1,
        };
        let animations = model(vec![clip(16, 2)], playable);
        let resolved = resolve_clip(&animations, 18, &AnimDataCatalog::from_rows([]))
            .expect("the model's validated fallback clip is present");
        assert_eq!(resolved.requested_id, 18);
        assert_eq!(resolved.resolved.id, 16);
        assert_eq!(resolved.resolved.dir_flags, 1);
        assert_eq!(resolved.clip_node, AnimationNodeIndex::new(2));

        let empty = model(Vec::new(), Vec::new());
        assert!(resolve_clip(&empty, 18, &AnimDataCatalog::from_rows([])).is_none());
    }

    #[test]
    fn locomotion_rate_uses_the_client_rate_scaled_sequence_rules() {
        let run = clip(5, 0);
        assert_eq!(super::super::locomotion_rate(&run, 14.0, 2.0), Some(1.0));
        let attack = clip(57, 1);
        assert_eq!(super::super::locomotion_rate(&attack, 14.0, 2.0), None);
    }

    #[test]
    fn node_playback_applies_rate_and_replaces_the_transition_track() {
        let node = AnimationNodeIndex::new(1);
        let mut player = AnimationPlayer::default();
        let mut transitions = AnimationTransitions::new();
        play_node_with_blend(
            &mut transitions,
            &mut player,
            node,
            RepeatAnimation::Never,
            1.25,
            Duration::ZERO,
        );
        let active = player
            .animation(node)
            .expect("requested graph node is armed");
        assert_eq!(active.speed(), 1.25);
        assert_eq!(active.repeat_mode(), RepeatAnimation::Never);
    }

    #[test]
    fn upper_body_overlay_uses_the_authored_mask_node_and_requested_weight() {
        let node = AnimationNodeIndex::new(7);
        let mut player = AnimationPlayer::default();
        play_upper_body_overlay(&mut player, node, 0.125, 0.75, 8.0);
        let active = player.animation(node).expect("mask node is armed");
        assert_eq!(active.seek_time(), 0.125);
        assert_eq!(active.speed(), 0.75);
        assert_eq!(active.weight(), 8.0);
        assert_eq!(active.repeat_mode(), RepeatAnimation::Never);
    }
}
