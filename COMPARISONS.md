# Recorded comparisons

These measurements describe seven fixed 15-second input schedules on one licensed Neverball Easy01 course. World coordinates are used; the ball radius is 0.25. Expected positions are read solely for comparison. They are never supplied as movement inputs.

| Recording | Role | Samples | Max position difference | RMS difference |
|---|---|---:|---:|---:|
| construction | construction | 1123 | 0.036747 | 0.018401 |
| holdout-right | initial holdout, now regression | 1123 | 0.023509 | 0.011743 |
| holdout-left | initial holdout, now regression | 1123 | 0.044977 | 0.022172 |
| holdout-diagonal | initial holdout, now regression | 1123 | 0.064311 | 0.028992 |
| holdout-short-pulses | initial holdout, now regression | 1123 | 0.075941 | 0.031289 |
| final-holdout-left-forward | fresh holdout after final correction | 1123 | 0.072446 | 0.031561 |
| final-holdout-long-forward | fresh holdout after final correction | 1123 | 0.014401 | 0.005765 |

The boundary for these recordings is a maximum position difference below 0.10 world units. This is a declared toy comparison limit, not a claim about complete-engine accuracy. The final two recordings remained outside calibration until the compound-tilt and rounded-contact correction was complete.

The first diagonal holdout diverged by 0.541259 world units. Correcting tilt composition without fixing rounded contacts worsened one later collision to 4.502148 world units. Replacing expanded-plane corner approximations with closest-point convex contacts reduced the final diagonal result to the value in the table. Earlier recordings now serve as regression fixtures.

The reference replay reports 90 Hz simulation updates. A construction-calibrated setup offset of 2.5222222222 seconds relates the global 60 Hz input schedule to simulation time; it is fixed for all recordings. The Rust renderer consumes the same frame-indexed key schedule. The captured video is fixed logical time, not a wall-clock speed or whole-level speedrun. Camera behavior and presentation intentionally differ.

Coin, win, loss, pause and reset rules have scoped tests; these position recordings do not demonstrate all possible game states. The collision implementation remains approximate relative to the reference swept solver, especially at high speed and edge cases. Other levels, moving geometry, original camera modes and full replay compatibility are excluded.

Run the public comparison CLI:

```sh
cargo run --locked --bin compare -- fixtures/traces/final-holdout-left-forward.json fixtures/traces/final-holdout-left-forward.txt
```
