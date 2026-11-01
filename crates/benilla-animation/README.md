# benilla-animation

Network-independent animation selection and playback primitives shared by Warcraft actors.

The crate reuses the client implementation's model-aware locomotion candidate tables, one-shot
track routing, clip-direction rules, locomotion-rate rules, variation/replay selection, Bevy
cross-fade setup, and key-bone upper-body overlay setup. `benilla-app` remains the client adapter:
it supplies packet-resolved movement facts, actor state, animation requests, and its established
interruption/lifecycle behavior. Applications such as Warcraft Survivors supply their own
collision-resolved persistent movement snapshots and gameplay animation requests.

This crate has no dependency on `benilla-app`, networking, sessions, authentication, or game
content. It does not own movement, combat, action timing, or actor lifecycle. Its consumers retain
those authorities and use the public helpers to resolve loaded `ModelAnimations` metadata and
apply playback consistently. Optional clips remain optional; resolving a missing clip returns
`None` so the adapter can apply its own validated fallback.
