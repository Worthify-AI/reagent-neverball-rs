# Recorded comparisons and remaining differences

Reference: Ubuntu Neverball `1.6.0+git20180603-3build2`, a stripped x86-64 ELF, SHA-256 `e762f51e8c3427786bcb36afe94f62d54e2903a8e04c5c8e06642dce24525410`. The executable was the only implementation-code input. Generated decompilation and observed execution supplied implementation evidence; licensed compiled levels/models and artwork/audio were separate runtime inputs. Original engine implementation and existing reimplementation source remained withheld.

## Nine courses, independent input comparisons

Each comparison constructs `FullGame` from a compiled SOL initial state. A saved raw SDL-key ledger is the input. The original recording supplies expected measurements; its positions, velocity, tilt, camera and mover state are never applied to the live Rust simulation. Input frames run at 60 Hz and span one or two fixed 90 Hz simulation updates.

The nine retained routes contain **6,990 recorded batches**, **6,989 compared position samples** and **6,981 simulation updates**. Maximum observed position separation is zero on each of these recordings. This states a finite measured result, not global bit parity on arbitrary states or platforms. Coin-count comparisons also match where measured. The mover comparison's camera-eye samples match.

| Course | Recorded batches | Observed behavior | Maximum measured position separation |
|---|---:|---|---:|
| Easy 01 | 2,423 | Ten coins, goal, success and upward result animation | 0 |
| Roundcoins | 764 | Twenty coins, three switches entered, four toggles, repeated elevator contacts | 0 |
| Easytele | 601 | Jump/teleport, one-second physics pause, fall and result motion | 0 |
| Learngrow | 646 | Three coins and fall; this route did not reach a size item | 0 |
| Locks | 392 | Roll and fall; this route did not enter a switch | 0 |
| Curved | 439 | Curved surface, roll and fall | 0 |
| Thwomp2 | 454 | Moving course, coin and fall; no separate thwomp-impact claim | 0 |
| Adventure | 382 | Shrink pickup with 46 recorded radius commands, fall and result motion | 0 |
| Mover | 889 | 888 compared updates, moving-platform course and independent Chase camera | 0 |

Only Easy 01 is a completed-course recording. Named features present in a level are not counted as exercised unless the route reaches them. All 183 packaged gameplay levels additionally parse and run 180 idle steps; content loading is broader than route coverage.

The Easy 01 route was planned in Rust and saved before the original executable ran. Both received the same 368 key-down/key-up events, including releases. Both reach success at batch 2,334 and collect ten coins. All 2,423 recorded position samples match through the result animation. The native comparison media preserves each application's independently rendered frames, with labels outside the game images and no per-side retiming.

## The arithmetic that mattered

The initial elevator run separated by 0.676638 world units even though coin counts and switch events agreed. Matching the executable required several precise rules:

- A moving-platform contact uses the sampled path chord during the current sweep, rather than reevaluating the smooth path at contact time.
- Plane, edge and vertex contacts use the original world-space arithmetic and restitution/dot-product order.
- Movers and timed switches consume shared integer-millisecond ticks with a fractional accumulator.
- SOL path durations are rounded to integer milliseconds, then **multiplied by `0.001f32`**. A nominal three seconds becomes `3.000000238418579`; division by 1,000 produces a different float.
- The collision iteration budget spans the entire update, including path-boundary segments. The final iteration consumes remaining motion without another contact search.

These are binary-derived rules, not fitted corrections using expected positions. After the changes, the same elevator route's recorded positions, radius, tilt and coins match throughout. Direct executable component probes also exercise timed-switch expiry and a path-boundary collision-budget case.

## Repeatable public checks

`cargo test --locked` runs **46 tests with zero ignored tests**, including 19 physics tests, the all-course load/idle test, original replay parsing, numeric component measurements and actual raw-key fixtures. A separate checkout with no private experiment directory passed the full suite and strict all-target Clippy.

`fixtures/physics/` retains sanitized NBRs, exact delivered ledgers, component measurements, provenance and hashes. Player headers are normalized to “Fixture”; original command streams remain unchanged and their hashes are retained. Missing fixtures fail rather than silently skip coverage. Original levels come from the licensed public `data/` directory.

The current course comparator is `src/bin/physics-course-holdout.rs`; completed-route/result comparisons use `src/bin/physics-result-holdout.rs`. `compare-easy-win.rs` retains an earlier comparison attempt and is not the source of the final result above. Ordinary NBR replay playback deliberately restores recorded state and is separate from these independent simulations.

## Component and numerical limits

Constructed component calls test selected calculations rather than whole-game playthroughs. A synthetic rotating-platform 90-update case retains maximum position separation `1.0374162e-5` and velocity separation `6.624146e-5`. Ball orientation uses a quaternion representation and has small basis differences from the original matrix integration: maximum observed outer/inner basis differences on the turning component trace are approximately `9.09e-6` and `6.06e-6`.

The independent native camera run, the component camera fixture and replay-camera playback are separate exercises. A component fixture supplied with measured positions does not stand in for a live independent camera comparison.

## Earlier attempts

Commit `450aee7c39f77d472beaf32a589f078b38100303` contained a one-course approximation. An early diagonal run separated by 0.54 world units. Subsequent contacts/camera corrections improved it, but still lacked ordinary engine systems. Earlier Easy 01 comparisons also had a one-tick success difference and larger result-animation separation; the final arithmetic changes resolve those in the retained route.

An early 60/90 Hz comparator misread input holding, and an importer dropped camera-only teleport batches. Those failed attempts remain in the private experiment record. The original NBR importer now retains all 601 batches of the teleport fixture, including 90 camera-only frames.

## Remaining scope

The engine implements the principal packaged game systems and flows. Finite measurements do not cover every contact, course, damaged input or device. GPU filtering, text layout and some presentation details can differ. Browser display controls have capability limits; native joystick support is unavailable and labeled accordingly. Physical gamepad hardware has not been exercised. Arbitrary ZIP/PK3 add-on search and original save-profile interchange are not implemented; a native alternative data directory/course can be selected with `--data` and `--level`.

No broad accuracy percentage or speed advantage is claimed. The result is a working reconstruction with a stated test boundary and recorded differences, not a universal software-recovery benchmark.
