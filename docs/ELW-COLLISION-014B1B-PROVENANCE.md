# ELW-COLLISION-014B.1B — Golden Reference Provenance Investigation

**ELW-COLLISION-014B.1B — PARTIAL**

## Finding

The immediate divergence is a start-support mismatch in the current scripted POC replay, concentrated in the two tree locations. It is visible on the first mover frame, before meaningful directional input. The preserved tree CSV and extracted replay start supported at the authored feet position; the current scripted POC does not. That small initial fall changes later tree contacts and can send some trajectories around rather than into the tree, producing errors near 10 yards. This establishes why the current POC replay differs in those tree cases at the state/output level. It does **not** establish why the current POC's initial terrain query misses support while the extracted integration replay sees it, nor which exact source/configuration generated the earlier grounded reference. Historical provenance and fully controlled current-source equivalence therefore remain unresolved.

No movement algorithm or collision behaviour was changed. The accepted interactive prototype was not modified or relaunched.

## Checkout and evidence inventory

| Checkout | HEAD / branch | Worktree at inspection | Use and boundary |
| --- | --- | --- | --- |
| `C:\Survivors\benilla-movement-integration` | `723f5b4a38b258fbaaf8789a20bebaf4c8a2f627` / `codex/elw-collision-014b1` | Existing modified extraction files, new harness/scripts/artifacts/docs; this report is the only file added by this task | Permitted diagnostic/report workspace |
| `C:\Survivors\goldshire-movement-poc` | `b96a983ebe151015c188cc285b998183587cf69c` / `main` | Existing dirty POC source, playable prototype and evidence files | Read-only; untouched |
| `C:\Survivors\elw-full-001\warcraft-survivors` | `0467ee90f7c42a0c1ffb0fcfddc4f00e4d001242` / `codex/elw-full-001-game` | Two pre-existing untracked debug logs | Read-only; untouched |
| `C:\Survivors\engine` | `b96a983ebe151015c188cc285b998183587cf69c` / `main` | Clean; branch reported two commits behind `origin/main` | Read-only; untouched |

The POC reflog contains only its clone entry at `2026-10-31 19:47:52 +0000`; there is no committed version of its untracked `crates/benilla-app/src/player/movement_poc.rs` runner to recover. That runner was created at 19:51 and last modified at 21:48 UTC. The preserved fence-sequence CSV was modified at 21:31 UTC. These timestamps establish that the current runner file changed after the artifact, but timestamps alone do not reveal the prior source contents or prove what binary emitted the artifact. The playable runtime file was also created/modified after that CSV. The integration checkout has no shared Git object for the POC HEAD hash, so its reflog cannot recover the POC runner either.

Preserved POC artifacts were not written:

| File | Modified UTC | Size | SHA-256 |
| --- | --- | ---: | --- |
| `goldshire-movement-poc-012a-fence-sequence-upstream-capsule.csv` | 2026-10-31 21:31:44 | 126,840 bytes | `CBC3E0C640941D41E85A15C43153E698BDA8C285EA05220CB2AC87D1728CF9A0` |
| `goldshire-movement-poc-012a-upstream-capsule.csv` | 2026-10-31 21:31:44 | 126,840 bytes | `CBC3E0C640941D41E85A15C43153E698BDA8C285EA05220CB2AC87D1728CF9A0` |
| `goldshire-movement-poc-012a-goldshire-upstream-capsule.csv` | 2026-10-31 21:01:37 | 100,894 bytes | `426A7CCA90F6F7E6DFEAB390E838EDA3815688542D4B244142575388F428E963` |

The POC generic upstream and fence-sequence upstream files are byte-identical. The fresh current-source output was isolated under `artifacts/elw-collision-014b1a-poc-reference-dev-1/`, not written to the POC:

- `goldshire-movement-poc-012a-upstream-capsule.csv`, modified 2026-10-31 23:46:47 UTC, 125,918 bytes, SHA-256 `A23384D6AAD518B3B9D36A403494A17A625760739C52B54CA7E95CB52521F6E5`.
- `poc.log`, `current-poc-vs-extracted.tsv`, and `preserved-vs-fresh-recheck.tsv` are colocated there.
- Extracted trace/sample/summary outputs are under `artifacts/elw-collision-014b1a-extracted-dev-2/`.

The POC replay used the upstream capsule (radius 1/3 yd, height 2.0277777 yd), 0.02-yard skin, 30/60/120 Hz, local WotLK world data, and the default non-Survivors capsule feature selection. Its fresh output and the extracted harness outputs were directed to the integration checkout. The extraction report records `WOW_DATA=C:\Wrath of the Lich King\Data`; no WotLK files were changed. Exact historical process invocation, binary hash and environment snapshot for the preserved CSV were not present, so they cannot be verified here.

## Source and setup comparison

The current scripted runner in POC `crates/benilla-app/src/player/movement_poc.rs` and the extracted harness in integration `crates/benilla-worldview/src/bin/elw_movement_equivalence.rs` use the same 20 fixed scenario definitions plus the terrain ledge, the same authored start coordinates, WoW-to-Bevy direction conversion, 3.5 yd/s movement speed, 0.25-second sample cadence, fixed rates, 64-step update cap, upstream capsule dimensions, 0.02 skin, and `presentable()` wait. Both reset their movement state to defaults and assign the fixed scenario start. Both advance one fixed timestep and call the corresponding mover entry point once per simulated step.

Important differences and qualifications:

- POC calls its current `mover::step(&mut Player, ...)`; extraction calls `benilla_movement::advance(&mut MovementState, ...)` through the new adapter. The first-frame outputs prove these calls are not observing equivalent support in the tested runs, despite matching authored pose and capsule settings.
- The current POC runner directly starts at hardcoded `scenario.start_wow`; it does not ray-ground, validate the ground collider, or delay the first mover step on an individual support check. `WorldLoadProgress::presentable()` is its only pre-run gate.
- The extracted replay's same start pose is supported on its first step. Its first Tree A sample at 30 Hz is `(-9482.589844, 45.021801, 57.132965)`, grounded, vertical velocity zero.
- The integration checkout contains terrain-residency pin additions and the extracted movement adapter absent from the current POC checkout. The equivalence harness itself does not request terrain pins. These surrounding-code differences remain possible contributors; their effect on collision-query readiness was not isolated.
- `worldview::run_with` differs slightly: POC conditionally installs the default viewer unless `DisableDefaultViewer` is inserted; integration always installs it. Neither trajectory harness inserts `DisableDefaultViewer`, so this code difference alone does not explain the observed result. The integration harness uses a stamped `BuildId`; the POC runner uses `BuildId::default()`. No causal effect of that metadata difference is known.
- POC's current capsule feature is off for the compared upstream run. The Survivors-capsule feature changes radius/height and is not part of this comparison.
- POC and integration both wait on `WorldLoadProgress::presentable()`, but the gate does not assert that a particular start capsule can already classify a walkable ground collider. It is not proof of a successful support query at every scripted start.

Current scenario definitions match for the 20 static trajectories and the pinned terrain ledge. POC discovers the ledge at runtime; the integration harness pins its captured coordinates. The supplementary jump scenario has no golden counterpart and was excluded from the 63-pair historical comparison.

## Earliest divergence and failure grouping

### Representative tree case: `tree_a_hold`, 30 Hz

- Authored start in both harnesses: WoW `(-9482.59, 45.0218, 57.132965)`.
- Capsule: radius `0.33333334`, total height `2.0277777`, skin `0.02`; no horizontal input during this hold.
- Historical CSV first sample at 0.033333 s: exact authored pose, `grounded=true`, `vy=0`, ground entity present (`4564v0` in that process).
- Extracted harness first sample at 0.033333335 s: same pose, `grounded=true`, `vy=0`.
- Fresh current POC first sample at 0.033333 s: `(-9482.586, 45.018, 57.132)`, `grounded=false`, `vy=-0.643 yd/s`, `ground=None`.
- At 0.266667 s the current POC has settled at approximately `(-9482.530, 44.963, 57.125)`, about `0.0843 yd` below the historical/extracted start. That offset persists through the hold.

This is the earliest meaningful divergence: the first support classification, before directional input and before tree contact. It is consistent with the initial support query not finding walkable support at that POC step. The runner log records `ground=None`; it does not record the raw collision query validity, candidate hit entity, hit point/normal or system-order/readiness state. Therefore the exact low-level reason for the miss (world/collider query readiness versus the fixed feet height/surface relation) is **not proven**.

For `tree_a_x_negative`, the same first-frame support miss is followed by a large trajectory divergence: at the 0.50-second sample the position delta is about 0.645 yd, and by the final sample it reaches 9.993 yd. Current POC reports substantially fewer contacts than the extracted replay (30 Hz details: 19 versus 90), demonstrating that the vertical initialization difference changes later tree collision interaction, rather than merely changing a cosmetic summary value.

### Other representative cases

- `blacksmith_stairs_down`, 30 Hz is a passing current-source control. Both runs use the same start/input and finish within sub-milliyard sampled tolerances. The current POC's first sample may be airborne briefly, but subsequent geometry traversal converges sufficiently for this trajectory's configured comparison.
- `fence_direct_approach`, 30 Hz is a passing obstacle control: sampled positions, velocities and contact counters match within tolerance, and both retain the authentic fence block/recovery behavior.
- The one non-tree failed pair is `fence_parallel_and_recover`, 120 Hz. Position max error is `0.002374 yd`, final error `0.000987 yd`, velocity max error `0.000241 yd/s`, and sampled grounded states agree. However, aggregate counters differ: grounded ticks `425/421`, airborne ticks `25/29`, airborne episodes `7/8`, contacts `200/206`, and obstacle contacts `108/120` (current POC / extracted). Its preserved and fresh POC first sample is also airborne. This is a genuine counter-equivalence failure but not evidence of a material path displacement. Its cause is unresolved and should not be attributed to the tree start mismatch.

### All 31 current-POC versus extracted failures

- 30/30 Tree A/B scenario-rate pairs fail: five trajectories for each tree at all three rates. All ten tree scenario types share the initial support miss in the current POC output. The worst later movement divergence is the tree-side route that passes on opposite sides after contact handling differs, at about 9.993 yd.
- 1/3 `fence_parallel_and_recover` pairs fail, only at 120 Hz, due to the aggregate-counter differences above; positions remain within 0.0024 yd.
- The other 32/63 pairs pass, including the direct-fence and blacksmith controls.

The preserved-vs-fresh comparison has the same 31 failing pairs and maximum sampled position error `9.992640 yd`. Current-POC-vs-extracted has maximum sampled position error `9.992791 yd`. These are separate comparisons; a passing preserved-artifact comparison does not establish the current POC source as the golden's source.

## Playable runtime relationship

The playable prototype uses the same upstream `mover::step`, `Player`, capsule constants and `WorldCollision` API. It is not the scripted runner's exact initialization or input path:

- `movement_poc/playable/runtime.rs::initialize_player` waits for `presentable()`, then casts a 24-yard downward ray from 12 yards above the authored start and nearby offsets. It accepts only a sufficiently upward-facing `GroundDecalSurface`, then places the player at the ray hit before enabling simulation.
- The scripted runner instead assigns the scenario's hardcoded three-dimensional position and immediately calls the mover; it does not perform that start-ground validation.
- Playable movement is camera/facing-relative and user-driven through a fixed-step accumulator (default 60 Hz), while the runner uses fixed world-axis inputs and deterministic durations at 30/60/120 Hz.

Thus this discrepancy directly concerns scripted start initialization and collision readiness at its tree start. It is not evidence that the user's manually accepted playable controls or runtime are broken. The interactive experience remains supported by the user's visual/gameplay acceptance, not by this trajectory comparison.

## Historical reference versus extracted mover

The existing 014B.1A artifacts show 63/63 scenario-rate pairs and 807/807 quarter-second samples matching the preserved fence-sequence CSV, with maximum sample position error `0.000812 yd`, final position error `0.000007 yd`, and velocity error `0.000482 yd/s`; grounded states and summary counters match exactly. This is verified as artifact equivalence only. The source revision and exact initialization/process that produced the preserved CSV are not recoverable from the available POC Git history.

## Recommendation and next evidence

Keep 014B.1B **PARTIAL**. Do not change mover mathematics, alter tolerances, or replace the preserved golden files.

The smallest next diagnostic is an integration-only first-frame support trace for each failed scenario that records: start feet/capsule center, `presentable` and collider-backlog values, downward capsule probe result (entity, point/normal/distance if available), `GroundDecalSurface` membership, and the first mover outcome. Add an optional grounded-start mode that uses the same validated ground-ray procedure as the playable initializer, without changing its algorithm. Compare that mode separately from the historical fixed-coordinate mode. This can determine whether equalizing support initialization makes current POC and extracted mover agree under the integration geometry. The 120 Hz fence counter discrepancy should be traced separately by first differing per-tick contact/grounded event.

If the first-frame query cannot explain why the POC and integration stacks see different support, the next required evidence is an isolated current-POC replay with bounded raw collision-query instrumentation, or a separately approved read-only source snapshot—neither should write to or modify the accepted POC checkout. Do not promote a new CSV to golden status until its source, feature selection, data path, geometry readiness and initialization are recorded alongside it.

## Reproduction and validation

No broad tests were rerun; this task is provenance diagnosis and the referenced artifacts already contain the replay/comparison outputs. Existing recorded commands/results remain in `docs/ELW-COLLISION-014B1A-EQUIVALENCE.md`.

Fresh POC execution was previously isolated from the POC checkout, using the current POC source and output/target directories under integration. Its output files and hash are listed above. The POC and integration comparisons can be reviewed with:

```powershell
py -3 scripts/compare_elw_poc_replay.py `
  --reference C:\Survivors\goldshire-movement-poc\goldshire-movement-poc-012a-fence-sequence-upstream-capsule.csv `
  --fresh artifacts\elw-collision-014b1a-poc-reference-dev-1\goldshire-movement-poc-012a-upstream-capsule.csv `
  --output artifacts\elw-collision-014b1a-poc-reference-dev-1\preserved-vs-fresh-recheck.tsv

py -3 scripts/compare_elw_collision_014b1a.py `
  --reference artifacts\elw-collision-014b1a-poc-reference-dev-1\goldshire-movement-poc-012a-upstream-capsule.csv `
  --extracted artifacts\elw-collision-014b1a-extracted-dev-2 `
  --output artifacts\elw-collision-014b1a-poc-reference-dev-1\current-poc-vs-extracted.tsv
```

No code, POC, game, original-engine or WotLK data files were changed. No commit or push was made. The integration worktree remains intentionally dirty from the earlier 014B extraction; this report is the sole addition from this task.
