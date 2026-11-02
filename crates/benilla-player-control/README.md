# Benilla player control

`Controller` owns one offline actor's feet position, facing, persistent kinematics, bounded 60 Hz
clock, queued jump and collision-resolved locomotion snapshot. It delegates every physical step to
`benilla-movement` with the caller's existing `WorldCollision` and prepared capsule. It does not
own gameplay, networking, models, combat timing, saves or camera entities.

Collect input with `decode_input` (or `decode_input_with_mouse_look` after cursor/focus/UI policy).
Pass the resulting `ControllerInput`, effective forward run speed and optional absolute right-drag
camera yaw to `Controller::advance` once per render frame. Publish `position()` and `facing()` to the
actor root once after the call; consume `ControllerFrame::keyboard_yaw` in the camera only once.
The model-facing root turns only with arrow keys or accepted mouse look. An application's optional
skeletal body pose remains separate; strafing never steers the root.

`enabled = false`, incomplete collision readiness, missing collision resources, or invalid frame
time suspends integration and clears pending jump/catch-up time. It retains the physical arc so a
modal or focus loss resumes safely. Call `reset(position, facing)` at run entry or reconstruction.
Zero-step render frames retain the last collision-resolved animation facts; releasing input clears
ground locomotion immediately. Jump and landing events are reported once per integration call.

Bindings and the run/walk/backward speed ratios come from the WS-ENGINE-022 offline POC at
`feb4fa601e5706afdfe5127de73563b0ca0c6bb3`, based on Benilla revision
`ed28f15c013dfe287ec1cf0c2e079295665b5d8d`. The original POC and client use the same input/direction,
mouse-look and keyboard-rate helpers. The production controller adds the fixed-step orchestration
previously held by its consumer; the physical mover and collision implementation are unchanged.

The crate inherits Benilla's `MIT OR Apache-2.0` license. Preserve the repository license and
copyright notices when sharing derived source. It contains no WoW assets or AzerothCore code.
