# ELW-COLLISION-014B.1A — Golden Reference Equivalence and Performance

**Assessment: PARTIAL — the extracted mover matches the preserved reference artifact, but the current POC source does not reproduce that artifact.**

## Scope and source revisions

This validation adds an isolated real-geometry replay harness; it makes no changes to movement mathematics.

| Checkout | Revision | State / use |
| --- | --- | --- |
| Integration checkout | `723f5b4a38b258fbaaf8789a20bebaf4c8a2f627` | `codex/elw-collision-014b1`; existing 014B extraction work preserved, plus this harness |
| Goldshire POC | `b96a983ebe151015c188cc285b998183587cf69c` | Read-only dirty worktree; source was rebuilt into the integration checkout's separate target directory |
| Production game | `0467ee90f7c42a0c1ffb0fcfddc4f00e4d001242` | Read-only |
| Original engine | `b96a983ebe151015c188cc285b998183587cf69c` | Read-only |

The POC's pre-existing golden files were not changed. SHA-256:

| Golden file | SHA-256 |
| --- | --- |
| `goldshire-movement-poc-012a-fence-sequence-upstream-capsule.csv` | `CBC3E0C640941D41E85A15C43153E698BDA8C285EA05220CB2AC87D1728CF9A0` |
| `goldshire-movement-poc-012a-goldshire-upstream-capsule.csv` | `426A7CCA90F6F7E6DFEAB390E838EDA3815688542D4B244142575388F428E963` |

All new executables, replay traces and comparison outputs are in this integration checkout, under `target/` or `artifacts/`. The extracted WotLK data was read only.

## Method

The new `benilla-worldview` binary boots the real world/collision stack from local WotLK data and drives the extracted `benilla-movement` API. It replays the 20 fixed scenarios encoded by the preserved POC sequence plus the POC's pinned `terrain_ledge`, at 30, 60 and 120 Hz, and adds one supplementary jump-and-land case. Each uses the upstream capsule (radius 1/3 yd, height 2.0277777 yd), 3.5 yd/s horizontal speed where applicable, matching input direction and duration, and fixed simulation steps. The comparison excludes the supplementary jump case because it has no golden counterpart.

Reference samples are preserved at 0.25-second intervals and rounded to 0.001. Before comparison, tolerances were fixed at 0.01 yd per position axis, 0.01 yd/s per velocity axis, 0.011 seconds for sample time (the reference rounds to 0.01), and 0.011 for input components. Grounded state and summary counters are exact. These tolerances accommodate output precision and floating-point noise without hiding visible drift. The extracted harness also records every tick.

### Preserved artifact versus extracted replay

| Scenario | Rates | Samples | Max sampled position delta (yd) | Max final delta (yd) | Result |
| --- | --- | ---: | ---: | ---: | --- |
| Tree A — hold | 30/60/120 | 39 | 0.000255 | 0.000006 | Pass |
| Tree A — +X | 30/60/120 | 42 | 0.000766 | 0.000005 | Pass |
| Tree A — −X | 30/60/120 | 42 | 0.000255 | 0.000006 | Pass |
| Tree A — +Y | 30/60/120 | 42 | 0.000690 | 0.000005 | Pass |
| Tree A — −Y | 30/60/120 | 42 | 0.000576 | 0.000006 | Pass |
| Tree B — hold | 30/60/120 | 39 | 0.000262 | 0.000004 | Pass |
| Tree B — +X | 30/60/120 | 42 | 0.000643 | 0.000006 | Pass |
| Tree B — −X | 30/60/120 | 42 | 0.000779 | 0.000004 | Pass |
| Tree B — +Y | 30/60/120 | 42 | 0.000581 | 0.000005 | Pass |
| Tree B — −Y | 30/60/120 | 42 | 0.000459 | 0.000006 | Pass |
| Fence — head-on approach/block | 30/60/120 | 60 | 0.000750 | 0.000006 | Pass |
| Fence — parallel and recover | 30/60/120 | 48 | 0.000688 | 0.000004 | Pass |
| Fence — retreat after contact | 30/60/120 | 24 | 0.000746 | 0.000006 | Pass |
| Blacksmith — elevated floor hold | 30/60/120 | 39 | 0.000396 | 0.000006 | Pass |
| Blacksmith — stairs down | 30/60/120 | 24 | 0.000759 | 0.000005 | Pass |
| Blacksmith — stairs up | 30/60/120 | 24 | 0.000812 | 0.000005 | Pass |
| Blacksmith wall — +X | 30/60/120 | 36 | 0.000543 | 0.000004 | Pass |
| Blacksmith wall — −X | 30/60/120 | 36 | 0.000622 | 0.000007 | Pass |
| Blacksmith wall — +Y | 30/60/120 | 36 | 0.000694 | 0.000007 | Pass |
| Blacksmith wall — −Y | 30/60/120 | 36 | 0.000619 | 0.000005 | Pass |
| Terrain ledge | 30/60/120 | 30 | 0.000636 | 0.000004 | Pass |
| **Total** | **63 scenario/rate pairs** | **807** | **0.000812 overall maximum** | **0.000007 overall maximum** | **63/63 pass** |

All matched summary counters (tick count, grounded/airborne ticks and episodes, contacts, obstacle contacts, final grounded state) also match exactly. This establishes that the extraction reproduces the **preserved CSV artifact** at the measured samples and counters.

## Reference provenance discrepancy — unresolved

To verify the golden snapshot itself, the POC was freshly rebuilt from its current read-only worktree using an integration-local target directory and run with output directed to a unique integration artifact directory. The replay used the same 21 scenarios and rates. Compared with the preserved `fence-sequence` golden CSV, **31 of 63 pairs fail**; maximum sampled position difference is **9.992640 yd**, and maximum velocity difference is **3.858 yd/s**. The largest concentration is in the Tree A/B cases. For example, preserved Tree A hold remains grounded throughout, while the fresh POC run has 3 airborne ticks at 30 Hz and an initial drop of about 0.084 yd. Several other fresh Tree trajectories differ materially in contact and airborne counts. Some fence and blacksmith cases remain close.

The current POC checkout is dirty and its source timestamps are newer than the preserved outputs. Therefore those outputs are not demonstrated to have been generated by the source now present in the POC worktree. This investigation did not isolate which source or setup change explains the discrepancy. It would be unsound to call current-source equivalence proven from the preserved-artifact comparison alone, or to change movement behavior to make either comparison pass.

Comparing the extracted replay directly against the freshly rebuilt current POC CSV likewise fails 31/63 scenario/rate pairs, with a maximum sampled position discrepancy of 9.992791 yd. This corroborates the provenance gap; the 63/63 preserved-artifact pass is not a substitute for a source-matched comparison.

The integration and POC checkouts also differ in surrounding application/world-streaming code from the extraction work. In particular, the integration has movement extraction/adaptation and terrain-residency additions absent from the current POC tree. No production movement math was changed here. The current-source mismatch needs a maintainer-approved, source-pinned golden regeneration or a narrow diagnosis before equivalence can be accepted.

## Performance

Both sides report per-mover-step timings in optimized dev builds; world initialization and initial streaming are outside the recorded step samples. The POC records a mean timing per scenario; the extracted harness records each step and reports per-scenario mean/median/p95/max. For comparable distribution summaries, the table shows step-count-weighted mean and the median/p95 across scenario means (21 matched scenarios at each rate):

| Rate | Steps each | POC weighted mean (µs) | Extracted weighted mean (µs) | Ratio | Scenario mean median POC/extracted (µs) | Scenario mean p95 POC/extracted (µs) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 30 Hz | 1,872 | 72.335 | 73.019 | 1.009× | 42.970 / 45.255 | 192.833 / 200.797 |
| 60 Hz | 3,740 | 92.670 | 88.918 | 0.960× | 42.545 / 38.084 | 357.830 / 356.153 |
| 120 Hz | 7,443 | 103.984 | 102.631 | 0.987× | 45.296 / 43.889 | 351.912 / 367.031 |

No material systematic overhead is apparent in these measurements; weighted means differ by −4.0% to +0.9%. However, the POC measurements come from its preserved output and could not be confirmed against a fresh replay that matches that output. Treat this as indicative rather than a definitive same-source benchmark. The 120 Hz p95 scenario mean is higher for extraction by about 15.1 µs; the weighted mean is lower, so this appears to be scenario/runtime variation rather than broad overhead, but the provenance mismatch limits the conclusion.

## Supplementary jump/landing

The extra real-geometry jump scenario completed one jump and was grounded at the end at every rate: 68 ticks / 12 airborne at 30 Hz, 136 / 24 at 60 Hz, and 270 / 49 at 120 Hz. This is a smoke result only; it is not a reference-equivalence claim.

## Validation

Commands and outcomes completed for this report are listed below. The first full test invocation had one transient failure; an isolated rerun passed, and a subsequent full retry passed:

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --locked --all-targets --message-format short -- -D warnings` — pass after reducing five scenario-coordinate literals to equivalent `f32` precision.
- `cargo check --locked` — pass.
- `cargo test --locked -p benilla-movement` — 53 passed, 0 failed.
- `cargo test --locked -p benilla-app --lib player::` — 240 passed, 0 failed.
- `cargo test --locked` — `benilla-adt`: 9 passed; `benilla-app`: 4,131 passed, 1 failed, 344 ignored. The failure was `net::io::cycle_tests::a_failed_world_dial_keeps_the_logon_for_the_next_realm`; isolated rerun passed (1 passed). This is outside movement and appears intermittent, but the complete suite has not yet passed on a clean rerun.
- Debug `cargo build --locked` — pass.
- Release `cargo build --release --locked` — pass.
- Final `cargo test --locked --quiet` retry — pass. `benilla-app` reports 4,132 passed / 0 failed / 344 ignored; all other workspace test targets also passed (including 9 `benilla-adt` tests). The first full run's one failure was the realm retry cycle test; its isolated rerun passed 1/1.
- `git diff --check` — pass.

Data-dependent collision replay used `WOW_DATA=C:\Wrath of the Lich King\Data`, read-only.

## Reproduction and artifacts

From this integration checkout, with the local data environment set:

```powershell
cargo run -p benilla-worldview --bin elw-movement-equivalence --locked
py -3 scripts/compare_elw_collision_014b1a.py --reference C:\Survivors\goldshire-movement-poc\goldshire-movement-poc-012a-fence-sequence-upstream-capsule.csv --extracted artifacts\elw-collision-014b1a-extracted-dev-2 --output artifacts\elw-collision-014b1a-extracted-dev-2\comparison.tsv
py -3 scripts/summarize_elw_performance.py --poc C:\Survivors\goldshire-movement-poc\goldshire-movement-poc-012a-fence-sequence-upstream-capsule.csv --extracted artifacts\elw-collision-014b1a-extracted-dev-2\extracted-summary.tsv --output artifacts\elw-collision-014b1a-extracted-dev-2\performance.tsv
```

Primary replay evidence is in `artifacts/elw-collision-014b1a-extracted-dev-2/` (`extracted-trace.tsv`, `extracted-samples.tsv`, `extracted-summary.tsv`, `comparison-recheck.tsv`, `performance.tsv`, and `run-info.txt`). Fresh POC replay and provenance comparisons are in `artifacts/elw-collision-014b1a-poc-reference-dev-1/`, including `goldshire-movement-poc-012a-upstream-capsule.csv`, `preserved-vs-fresh-recheck.tsv`, and `current-poc-vs-extracted.tsv`. The earlier 0.05-yard skin experiment is retained in `elw-collision-014b1a-extracted-dev-1/` and is excluded; the accepted comparison used the POC's 0.02-yard skin.

## Files and status

New harness and comparison infrastructure is isolated to `crates/benilla-worldview/src/bin/elw_movement_equivalence.rs`, the `benilla-worldview` manifest, a test-only `benilla-world::worldview::run_with` extension seam, and the three scripts under `scripts/`. Existing 014B.1 extraction files and all user changes remain in place. This checkout is intentionally uncommitted, per task instruction. POC, game, original engine, and WotLK data were not written by the task; POC-generated replay outputs were directed to this checkout.

## Recommendation

Keep the outcome **PARTIAL**. Before approving behavioral equivalence, identify/pin the source/setup revision that generated the accepted golden CSV, then rerun both implementations against the same scenario definitions, geometry residency, and initialization. Do not change mover math or promote the extraction based solely on the passing historical CSV comparison. No MMap, Detour, or engine collision change is implicated by this validation.
