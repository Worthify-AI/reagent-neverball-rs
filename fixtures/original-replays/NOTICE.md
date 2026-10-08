# Original runtime replay fixtures

These recordings were produced by running Neverball 1.6.0 (Ubuntu package 1.6.0+git20180603-3build2) with its packaged compiled levels. They preserve the original NBR version-9 command streams; no engine source was consulted to implement the Rust decoder. The camera JSON files are independently decoded expected observations from those streams, not Rust simulation output.

- camera-modes.nbr / camera-modes.camera.json: 1,124 complete batches, Easy 01, changes among manual, chase and lazy camera modes.
- camera-transitions.nbr / camera-transitions.camera.json: 449 complete batches, Easy 01 after restart. The earlier pause/restart interaction is not claimed to remain in this restarted stream.
- holdout-diagonal.nbr: Easy 01; original item 15 is removed and coins become 1 near 7.75556 seconds; item 4 is removed and coins become 2 near 12.12222 seconds.
- physics-mover-complete-batches.nbr: 889 complete batches from map-easy/mover.sol. The capture supervisor timed out; the retained replay nevertheless ends at a complete batch boundary. It is byte-identical to the original partial capture, with no tail bytes removed. It is not a completed-level run. Mover 0 changes to path 1 near 5.001 seconds and path 2 near 6.001 seconds, with explicit clock resets.

The moving replay test uses the repository asset data/map-easy/mover.sol. Its preferred-form authoring asset is retained at assets/neverball/source-full/map-easy/mover.map. SHA256.json identifies every fixture and that compiled SOL input. Tests resolve paths relative to CARGO_MANIFEST_DIR and need no external capture directory, credentials, service, or network.

Neverball and its level/model assets: copyright Robert Kooima, Jean Privat and contributors; GPL-2.0-or-later. Complete package attribution is retained in data/COPYRIGHT.debian and the repository dependency notices. The original GPL license and corresponding authoring assets are distributed with this project. These runtime observations and our independently authored test/decoder code are distributed under GPL-3.0-or-later, consistently with the repository LICENSE; the original assets retain their own notices.

The NBR headers retain their original player label and timestamp. No authentication data, task receipts, host paths, or internal reports are included.

Teleport regression: easytele.nbr contains 601 original batches. Batches 155 through 244 omit physics/mover command 30 while retaining camera/timer updates. Playback uses update-rate command 18 and batch boundaries, so the full stream lasts 6.667 seconds rather than dropping the one-second jump interval. SHA256.json includes the unchanged recording.
