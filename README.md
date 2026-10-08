# Neverball in Rust, with engine source withheld

A Worthify-assisted reconstruction of Neverball’s packaged game behavior in Rust, for native Linux/macOS and browser WebAssembly. The executable was the only implementation-code input. Its generated decompilation, observed behavior, and separately disclosed licensed runtime data guided independently authored Rust.

Neverball is a GPL-licensed game by Robert Kooima and contributors. We chose it so the experiment, artwork and level data could be shared under their terms. This is a reconstruction experiment using an open-source game as a source-withheld reference.

## Play

```sh
cargo run --locked --release --bin reagent-neverball-rs
```

The default runtime data is `data/`. Arrow keys or mouse movement tilt the floor. `1`/`2`/`3` choose Chase/Lazy/Manual cameras; `S`/`D` rotate, Shift rotates faster, and `E` toggles Chase/Manual. Enter selects, Escape pauses, and `R` restarts. Browser touch controls are outside the game canvas. On a phone or tablet, tap **Enable tilt**, allow motion access, then hold comfortably still to center. **Center tilt** resets the neutral angle; portrait and landscape steering are supported. See [browser controls](docs/browser-controls.md).

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
./build-web.sh
python3 -m http.server --directory web 8000
```

Linux native builds need the system ALSA development library. Dependencies are pinned in Cargo.lock; the browser graphics and audio loaders are self-hosted. Native profiles use a local JSON save; browser profiles use localStorage.

## Content and behavior

The general SOL v8 reader loads all 183 packaged gameplay levels: 165 in seven ordinary level sets and 18 additional test levels. All have been parsed and idle-stepped; that is not a claim that every route has been completed. The 233 retained SOL files also include backgrounds, balls, interface and object models.

The engine implements swept plane/edge/vertex contacts, moving and rotating bodies, path interpolation, switches, coins, grow/shrink items, jump transitions, goals, fall-out and time limits. Game flows include level selection, challenges, progress and records, pause/retry/next, and replay storage/playback. Original NBR v9 replay reading is separate from live simulation: replay playback restores recorded states, while the same-key experiments begin from the level’s initial state and calculate movement independently.

The executable-derived component tests and specific playthroughs constrain the result. Numerical and rendering differences remain; see [COMPARISONS.md](COMPARISONS.md). The original implementation source and existing engine reimplementations remain withheld.

## Same keys, two implementations

The public input harness records frame-indexed SDL keys and supplies a fixed 60 Hz logical input clock to the reference. Both applications render their own images. Physics can take multiple 90 Hz steps within one input frame. Our comparisons preserve that relationship and do not retime either side.

`--tas FILE --capture DIRECTORY --frames N` exports native rendered frames and state measurements. `--data DIRECTORY --level map-easy/easy.sol` selects runtime content. The route planner and comparator are under `src/bin/`; they use reconstructed state for planning and original recordings as expected results for comparison. They do not inject recorded positions into live gameplay.

## Attribution

Our code is GPL-3.0-or-later. Licensed Neverball levels, models, textures and audio retain their upstream terms. Runtime originals are in `data/`; corresponding authoring assets are retained separately under `assets/neverball/source-full/` for redistribution. Those preferred-form files were acquired after reconstruction and were not implementation inputs. No upstream engine source or reference executable is distributed here.

See [NOTICE.md](NOTICE.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), and the asset provenance manifests. The Rust logo and article research are not part of this game repository.
