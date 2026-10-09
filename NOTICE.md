# Attribution and experiment scope

This Rust game is a Worthify-assisted reconstruction using Neverball as a source-withheld binary reference. Our independently authored code is GPL-3.0-or-later.

Neverball is copyright Robert Kooima and contributors, licensed GPL-2.0-or-later. Reference origin: Ubuntu package `1.6.0+git20180603-3build2`; x86-64 Linux executable SHA-256 `e762f51e8c3427786bcb36afe94f62d54e2903a8e04c5c8e06642dce24525410`. The executable is stripped, meaning debugging symbols were removed. It was not deliberately obfuscated for this experiment.

## Inputs

The executable was the only implementation-code input. Original engine source and existing engine reimplementation source were withheld throughout. We used binary-derived decompilation, public help, license information, runtime observations, recorded inputs, compiled SOL levels/models, and licensed artwork/audio. Familiar mechanics and possible model prior knowledge were not eliminated; this was not a sealed clean-room audit.

The original compiled course geometry, texture coordinates, model geometry and artwork were reused under their licenses. They were not recreated from screenshots. ReAgent helped inspect the executable, answer specific file-reader questions and exercise the running reference. Independently authored Rust implements the simulation, loading, rendering and game flows.

## Redistributed assets

`data/` retains runtime assets from the matching Ubuntu `neverball-data` and `neverball-common` packages. Per-file hashes and origins are in `data/ASSET-MANIFEST.json`; package origins and hashes are in `data/PACKAGE-ORIGINS.json`; complete permissions are in `data/COPYRIGHT.debian`.

Corresponding preferred-form level/model/material assets are in `assets/neverball/source-full/`, with origins and hashes in its `SOURCE-ASSETS.json`. These files were acquired separately after engine construction to meet redistribution obligations. No upstream engine C/H or other implementation files were extracted into the repository. The Octocat ball is excluded because its permission is specific to the upstream game.

Fonts retain their own notices. The game HUD uses unmodified DejaVu Sans Bold; Inter remains in the legacy presentation assets under SIL OFL 1.1. Rust dependencies and the self-hosted browser loaders retain notices under `web/licenses/` and `THIRD_PARTY_NOTICES.md`.

## Comparison boundaries

Live same-key comparisons start with the compiled level’s initial state. Reference positions, tilt and camera states are expected results only, not movement commands. Component tests may supply measured inputs or constructed states and are labeled separately. Ordinary replay playback intentionally restores recorded state; it is not an independent reconstruction accuracy test.

Recorded trajectories, component tests and level loading do not imply exact parity for every course, device or rendering setting. Current differences and failed earlier measurements are retained in `COMPARISONS.md`.

Upstream: https://neverball.org/ and https://github.com/Neverball/neverball/blob/master/LICENSE.md

## Runtime texture capture extension

A later bounded runtime-artwork experiment captured texture pixels through OpenGL without reading packaged artwork during construction. A separately authored helper was compiled and launched by ReAgent chat. After capture ended, licensed-asset comparison identified the basic checker ball and its row orientation; normal material loading now uses that recovered derivative. Original assets and manifests remain retained. See `experiments/runtime-artwork/README.md` and `PROVENANCE.json` for authorship, hashes, limits and the separate post-construction audit. This extension does not change the earlier engine-input disclosures above.
