PASS — current-source paired movement equivalence and fence checks pass under verified, physically supported initialization. The older isolated POC first-frame miss remains unreproduced; its historical cause is unknown.

# ELW-COLLISION-014B.1C — Ground Initialization and Current-Source Equivalence

## Result

The current playable POC mover and extracted `benilla-movement` mover were run side by side, against the same streamed WotLK collision world, with identical state, input, capsule and fixed timestep. All 30 Tree A/B trajectory-rate pairs passed at 30, 60 and 120 Hz. The 120 Hz `fence_parallel_and_recover` run also matched at every tick, including both movement state and the bounded collision-contact probes.

The 014B.1B airborne-first-frame observation did not reproduce in this same-world run. The current POC mover snapshot reports grounded at its raw authored Tree A pose, as does the extracted mover. The source-level initialization difference is confirmed: the POC's playable initializer ray-validates a ground start, while the historical scripted comparison runner assigns the authored position directly. That difference is real, but it does not by itself explain the old `ground=None` observation. This report therefore does not claim that the older isolated query failure has a proven cause.

**Overall status: PASS for current-source equivalence.** Both implementations start from the same physically validated support and agree across all required trajectories and rates. The historical separate-process `ground=None` sample is not reproduced; its exact cause remains unknown, which is recorded as a provenance caveat rather than evidence of a current-source divergence.

## Controlled initialization

The harness waits for the current POC `presentable` terms, records them, and also waits for the integration checkout's full gate. The first recorded POC-equivalent gate had `ready=5132/5132`, focus resident, scene ready, `pinned_collision_ready=true`, and zero pending collider, merge, placement or GX work. At that gate, an unmodified default movement state at the raw Tree A pose was stepped once with zero input. Both mover implementations returned grounded on the same terrain entity and preserved the same position. No grounded flag or hidden support was set by the harness.

For the paired trajectories, the harness follows the playable POC's ground-start method: a 24-yard downward body ray from 12 yards above the authored horizontal position; only a sufficiently upward-facing `GroundDecalSurface` is accepted. The selected feet point must then be recognized as grounded by a real zero-input mover step from both implementations on the same support entity. Each run resets every `MovementState` field to `Default` and sets only the accepted feet position.

Representative Tree A evidence from the recorded launch:

| Probe | Result |
| --- | --- |
| Exact authored horizontal point `WoW (-9482.59, 45.0218)` | First point-ray hit was entity `4366v0`, distance `11.978735 yd`, Bevy normal Y `0.075194`; it was not a walkable normal or a `GroundDecalSurface`, so the playable initializer correctly rejected it. |
| First accepted candidate | `+0.50 yd` on the Bevy X axis (WoW Y decreases by 0.50); entity `1947v0`, ray distance `11.928569 yd`, normal `(-0.136534, 0.955720, -0.260689)`, tagged `GroundDecalSurface`. |
| Validated feet position | WoW `(-9482.589844, 44.521801, 57.204395)` / Bevy `(-44.521801, 57.204395, 9482.589844)`. |
| Support step at that position | Both movers returned grounded with ground entity `1947v0`. |
| Raw authored pose at the same world-ready gate | Both movers returned grounded on the same terrain support and retained WoW `(-9482.59, 45.0218, 57.132965)`. |

The hit entity IDs are per-process diagnostics, not stable map identifiers. The normal and `GroundDecalSurface` check—not an entity number—is the ground acceptance evidence. The ray at the authored point sees an intervening steep/non-ground collider; the capsule mover query, whose footprint is wider than that point ray, still finds walkable support in the current shared world. This is why the point-ray and mover support results must not be conflated.

Tree B accepts the authored horizontal point directly: the ray hit is walkable terrain and both movers agree on the first movement step.

## Tree trajectory results

The harness ran ten Tree A/B scenarios at each of 30, 60 and 120 Hz: 30 paired trajectories and **7,525** paired per-tick rows across all 33 scenarios including the fence case. The comparator uses a predeclared 0.001-yard position tolerance and 0.001-yard/second velocity tolerance; it also requires exact equality of grounded classification, ground support entity within the same process, airborne timestamp, contact-probe result and obstacle-probe result.

- Tree pairs passing: **30/30**.
- All paired scenarios passing: **33/33** (the 30 tree pairs plus the fence case at three rates).
- Maximum recorded position and velocity deltas: **0.000000000** at the trace's seven-decimal output precision.
- Grounded/support, airborne-time and contact-probe mismatches: **0**.
- Earliest divergent tick: **none** for every pair.

Complete inputs and results are in `artifacts/elw-collision-014b1c-current/paired-per-tick.tsv`, `paired-analysis.tsv`, `ground-initialization.tsv` and `readiness-probe.tsv`. A second current-source launch is preserved under `artifacts/elw-collision-014b1c-repeat/` and also passes all 33 pairs. Entity handles differ between process launches, as expected; paired support identities match within each run.

## 120 Hz fence case

`fence_parallel_and_recover` completed 450 ticks. In the current paired run, both movers had 445 grounded ticks, five airborne episodes, 196 contact-probe ticks and 115 obstacle-probe ticks. Their per-tick positions, velocities, grounded state, ground support, airborne timestamps, contact probes and obstacle probes matched. The first mismatch tick is **none**. The repeated launch produced the same within-run paired counts and no divergence.

These contact counts are the harness's explicit pre-movement capsule probes, not an internal event counter emitted by either movement algorithm. The old separate-process comparison's larger counter delta was not reproduced by the common-world harness. Since the current traces show no difference in physical movement or probe events, the old result is independently resolved for this controlled comparison, but the precise cause of the older run's counter delta cannot be reconstructed from its aggregate-only record.

## Provenance and remaining uncertainty

The snapshot `crates/benilla-worldview/src/bin/legacy_poc_mover_snapshot.rs` is byte-for-byte identical to the current read-only POC file `C:\Survivors\goldshire-movement-poc\crates\benilla-app\src\player\mover.rs` at this validation: SHA-256 `6541C4E0B5DD96913F373D039A7F75DFE9D69C8E50B1A98CC8394ECFDB1729EC`. The snapshot is only called by this integration-only diagnostic binary. Both movers receive the same `WorldCollision` system parameter in one process; no movement algorithm, collision rule, capsule constant, production player path or Benilla source was changed.

The earlier fresh POC output had a first sample with `ground=None`, while the extracted replay was grounded. That exact result is not reproducible in the current same-process replay, including its raw-pose control. The current POC's scripted runner skips the playable initializer's ground-ray validation, and the POC and integration worktrees have distinct world-streaming/residency code. These setup differences are confirmed, but the available evidence does not prove which one caused the historical sample. Current-source equivalence is nevertheless demonstrated from verified support at the matching readiness gate. The preserved historical CSVs remain untouched and their provenance remains unknown.

## Validation

- `cargo fmt --all -- --check` — passed.
- `cargo check --locked -p benilla-worldview --bin elw-014b1c-grounded-equivalence` — passed without warnings after cleanup.
- `cargo run --locked -p benilla-worldview --bin elw-014b1c-grounded-equivalence` against the configured local `WOW_DATA` — completed successfully twice; both runs wrote only under the integration checkout's new artifact directories.
- `py -3 scripts/compare_elw_014b1c_grounded.py artifacts/elw-collision-014b1c-current` — passed 33/33 paired comparisons.
- Same comparator against `artifacts/elw-collision-014b1c-repeat` — passed 33/33 paired comparisons.
- `cargo test --locked -p benilla-movement` — passed, **53 tests**, 0 failed; doc-tests 0.
- `git diff --check` — passed.

No graphical run was used as evidence. The local client data was read-only. The POC, original engine and game checkouts were not modified by this task. No commit or push was made.

## Changed files and worktree

Task additions/changes in the integration checkout:

- `crates/benilla-worldview/Cargo.toml` — registers the diagnostic executable. This file already contained unrelated in-progress modifications before this task; they were preserved.
- `crates/benilla-worldview/src/bin/elw_014b1c_grounded_equivalence.rs` — same-world paired harness, readiness trace and ray-validated initialization.
- `crates/benilla-worldview/src/bin/legacy_poc_mover_snapshot.rs` — labeled byte-identical POC mover source snapshot.
- `scripts/compare_elw_014b1c_grounded.py` — deterministic per-tick comparator and tolerance gate.
- `artifacts/elw-collision-014b1c-current/` and `artifacts/elw-collision-014b1c-repeat/` — paired traces, readiness/start probes, summaries and snapshot provenance.
- `docs/ELW-COLLISION-014B1C-FINAL-EQUIVALENCE.md` — this report.

The existing integration branch remains `codex/elw-collision-014b1`. Its pre-existing modifications and untracked files remain. The POC and game worktrees also retain their pre-existing dirty/untracked files. No destructive Git command was used. Exact final `git status --short --branch` is recorded by the completion message; this validation checkout intentionally remains uncommitted.

## Integration recommendation

The extracted movement algorithm is supported by current-source, same-world equivalence evidence when both sides start from the same physically validated pose. That evidence is sufficient to carry the extraction into the next review of Warcraft Survivors integration. Preserve the validated ground initialization in that design and keep the older `ground=None` observation classified as unresolved until it is reproduced with a bounded first-frame query trace in a common collision world. Do not alter collision or movement behavior based on the unreproduced observation.
