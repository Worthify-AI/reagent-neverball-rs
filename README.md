# Worthify Rolling Lab

A playable Rust reconstruction of **one Neverball training course**, using licensed Neverball visual assets and an independently authored renderer. Native Linux and macOS, plus browser WebAssembly. Neverball is a free GPL game with Linux, Windows and macOS versions; this is not a Microsoft bundled game.

## Scope

Tilt, roll, collect ten coins, unlock the goal, beat the timer, fall, pause and restart. The numeric course geometry is converted from Neverball Easy 01 by Robert Kooima, under its GPL permission. We wrote a new renderer, input layer and approximate physics implementation from executable evidence and runtime observations, while withholding game implementation source.

The result does not reproduce all Neverball levels, its full swept collision solver, replay/save compatibility, audio, moving platforms, multiplayer or every camera mode. Position comparisons are bounded recordings, not a whole-engine accuracy claim. See `COMPARISONS.md` for construction, failed tests, corrections and remaining differences.

## Play and build

```sh
cargo run --release --bin reagent-neverball-rs
```

Arrows or W/A/S/D tilt; Enter or tap starts; Q/E rotate the view; V changes view distance; Space pauses; R restarts. Small viewports also show a touch direction pad. The course has 18 coins; ten unlock the exit.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
./build-web.sh
```

Serve `web/` with a local HTTP server to run the browser build. The loader and font are self-hosted.

## Recorded inputs

`harness/probe-inputs.txt` supplies frame-indexed SDL key codes. `--tas FILE --capture DIRECTORY` exports real native rendered PNG frames and simulation states. The same inputs can be supplied to the licensed reference with the included SDL API interposition harness. The harness synthesizes key events and a 60 Hz logical clock, captures real OpenGL frames and logs delivered events. It never injects ball coordinates or engine source. The resulting 15-second clip is a bounded forward-and-brake demonstration, not a completed speedrun or wall-clock benchmark.

See NOTICE.md and LICENSE for attribution, rights and dependency notices.

## Visual asset reconstruction

The render-only fixtures restore original compiled SOL materials, per-vertex normals and UV coordinates. `harness/convert_visual_sol.py .` regenerates these from retained visual assets and checks that all 2,320 course triangles match the unchanged physics fixture. Licensed PNG/JPEG textures, the basic-ball and coin artwork, the cloud/hill backdrop, and DejaVu Sans Bold appear in the renderer. Preferred-form map/model/material files and asset notices are under `assets/neverball/`.

Presentation remains approximate: directional lighting and specular response, transparency, texture filtering, contact shadows, camera behavior, ball orientation, HUD gradients, goal effects and menus differ from the reference. Physics is unchanged by this presentation update.
