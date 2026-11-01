# benilla-movement

Reusable, collision-backed kinematic movement extracted from the pinned Benilla player mover.

The application supplies a `MovementState`, `MovementModes`, one `MovementInput`, an already-built
`WorldCollision`, and a borrowed `CollisionConfig`. The crate returns the frame outcome and bounded
diagnostic facts. It does not know about the app's `Player`, networking, camera, renderer, or app
lifecycle. Capsule collider construction remains the caller's responsibility.

`CollisionConfig::upstream` selects Benilla's current 1/3-yard radius and 2.0277777-yard height;
the public dimensions permit later evaluation of a different capsule without changing the mover.
The physics implementation retains the upstream ground classification, slide, step-up/down, jump,
fall, snap, and landing paths.

The 53 upstream mover unit tests are retained here with the extracted implementation. They pass
with the upstream capsule, but are not a substitute for running the accepted POC's real Goldshire
geometry trajectories at 30, 60, and 120 Hz. That POC previously recorded two failures with the
smaller Survivors capsule: `a_long_frame_rides_no_faster_than_the_budget` and
`a_weightless_body_walks_off_a_ledge_level_and_cannot_jump`. Keep those assertions intact during
future capsule evaluation.

The POC's previous movement-step timing (88.8 microseconds mean of per-scenario averages for the
upstream capsule) is a reference measurement, not a measurement of this extracted crate. A current
like-for-like real-geometry performance comparison remains outstanding.
