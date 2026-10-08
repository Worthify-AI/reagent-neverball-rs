# Reading the compiled visuals

The executable was the only implementation-code input. Generated decompilation and observations of that executable guided the independently authored Rust engine. Compiled levels, models, licensed textures and sound are separately disclosed runtime data. Original engine implementation and existing engine reimplementation source remained withheld. Corresponding asset-authoring files were acquired after reconstruction solely for redistribution obligations.

## Follow the reader, then follow its indices

The executable-derived level reader checks little-endian signature `0x4c4f53af` and version 8. The 92-byte header supplies table counts. For Easy 01, 332 text bytes and twelve 8-byte dictionary records place the material table at byte 520. Eleven material records precede the vertices, edges, planes, texture coordinates, corner offsets and triangles. This file ends at byte 162716, exactly where the decoded records end.

A material normally occupies 136 bytes: four RGBA float vectors, shininess, flags and a 64-byte texture name. Flag `0x200` adds an integer and a float, eight bytes in all. A fixed-size reader that misses the extension can produce plausible values and then decode every subsequent table at the wrong address. The general parser follows the optional fields and checks the remaining data before allocating tables.

A 16-byte triangle contains a material index and three corner indices. Each 12-byte corner links a UV, a normal/plane and a vertex. Following those links recovers authored texture alignment and normals; no screenshot fitting is needed. The first course has 2320 triangles. Its arrow triangle at byte 133840 is `[9,1962,1963,1964]`. Material 9 names the arrow texture. The renderer's `V → 1−V` conversion adapts image coordinates; it does not change the recovered UV values.

The same parser loads 233 retained SOL files, including all 183 gameplay courses, backgrounds and models. Paths can carry quaternion records; moving bodies link to those paths. The physics and renderer sample the same reconstructed body poses. Runtime texture lookup uses a manifest rather than trial requests for guessed filenames.

## Alpha is not an RGB coefficient

The checkerboard ball's diffuse value is approximately `(0.8,0.8,0.8,1.0)`. Its material alpha is 1.0. The LA texture stores alpha 127 at every pixel: `127 / 255 = 49.8039%`. That is one texture-alpha input, not a measurement of the entire ball's final opacity.

The executable-derived rendering calls draw distinct inner, solid and outer model layers, including rear and front passes and depth-mask behavior. Overlapping surfaces, lighting and blending affect the final image. The Rust implementation follows those recovered layer rules rather than applying “80% opacity” to the whole sphere.

Material records also supply ambient, diffuse, specular and emission values and shininess. Binary light initialization supplies a global ambient term of `(0.2,0.2,0.2,1)` and two directional lights at `(-8,32,-8,0)` and `(8,32,8,0)`, with different diffuse/specular colors. The renderer implements material-dependent per-vertex lighting and separate specular color. Remaining rendering differences are listed in [COMPARISONS.md](../COMPARISONS.md).

## Flags choose a rendering operation, not just a color

Material flag `0x100` identifies reflective floor surfaces. The executable draws a stencil-constrained reflected scene with reversed winding and a clipping plane. The Rust renderer reproduces that path. Flag `0x40` receives the ball shadow; binary calls supply its projection matrix and the second height mask. The original texture-unit order is shadow, height mask, then the surface texture. Drawing the same textures in a different order loses the surface color.

Flag `0x400` marks point-sprite materials. A first renderer treated their indices as triangles and connected a jump beam's stars into enormous bright facets. The executable instead draws points with distance attenuation. Recovering that primitive, its size calculation and camera-facing billboards restores the small stars and animated avatar effects. The jump record stores position in fields 0–2, destination in 3–5 and radius in field 6. Reading destination X as radius doubled the first Easytele beam. The corrected scales use the recovered radius, without a fitted half multiplier. No replacement artwork was painted over the result.

These fixes came from comparing actual rendered frames, inspecting the executable's graphics calls and correcting the Rust operation. Material values alone do not recover the drawing rules.

## Level setup is data; behavior still needs an engine

Easy 01 stores the ball at byte 144556: `(1,0.2505,-1)`, radius `0.25`. The goal at byte 144540 is `(1,0,-17)`, radius `0.75`. Eighteen item records begin at byte 144180. Dictionary text supplies goal `10` and time `9000`; runtime establishes the interpretation as ten coins and ninety seconds.

Those inputs do not explain collisions, coin predicates, switch timers or the camera. The implementation recovers those separately from executable calculations and reference exercises. The camera has Chase, Lazy and Manual modes, stateful following and rotation. During a jump the vertical FOV follows `configured_fov × 2 × abs(client_elapsed − 0.5)`. The formula matches 660 retained original projection calls, including 60 jump frames. Rust currently samples simulation elapsed; the reference interpolates a separate client clock, so individual rendered frames can differ. The same-key camera comparison tests an independent live Rust camera, separately from component fixtures supplied with measured reference positions.

## Pictures and comparisons

The completed-course comparison sends the same saved 368 key-down/key-up events to the original Linux executable and the native Rust application. Both calculate movement independently and render their own frames. The editorial composite adds labels outside the 800×600 game images. It does not replace either game's pixels or retime a side to improve agreement.

The route collects ten coins and reaches the goal in both engines. All 2,423 recorded position samples match, including the upward result animation, and both reach success at batch 2,334. Earlier one-tick and result-animation differences remain in the historical experiment record; they are resolved in this retained route. Across nine courses, 6,989 position samples have zero observed separation. These are finite measurements, not a claim about every level or contact.

Original NBR replay playback intentionally restores recorded state. It is a separate feature and is not used as the Rust simulation in the same-key comparison. Replay import exercises the reader; independently reproducing its outcomes exercises engine behavior.

All 183 levels have been parsed and idle-stepped. Cross-level executable exercises and component checks provide additional coverage; neither establishes that every route has been completed. Licensed art explains the common visual identity. The newly authored engine explains the behavior being compared.
