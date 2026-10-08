# Browser tilt controls

On a phone or tablet, choose a course, tap **Enable tilt**, and allow motion access when prompted. Hold the device comfortably still: the first reading becomes neutral. **Center tilt** resets it. **Disable tilt** returns to mouse, keyboard, gamepad or touch controls. Motion permission is never requested during page load. If the embed receives no motion data, open the game in its own tab.

Steering uses relative device attitude, including a raised neutral position. It ignores compass heading, follows the screen orientation, and smooths the signal over 80 ms with a small neutral dead zone. Changing orientation, hiding the page or leaving focus clears steering and centers on the next reading. Touch arrows override tilt while held; keyboard steering keeps its existing precedence. Motion data stays on the device.

The WebAssembly build loads MP3 derivatives of the licensed reference sounds because some Safari Web Audio versions cannot decode the original Ogg music. Native builds retain the original recordings. Conversion hashes are in `data/web-audio/PROVENANCE.json`. A failed decoder emits a warning and a silent cue; it does not hang the game.

Run the permission, calibration, lifecycle and steering regression checks with Node.js:

```sh
node --test tests/web-tilt.test.cjs tests/web-audio.test.cjs
```

The browser checks use injected orientation readings and permission responses; this is distinct from testing a physical iPhone/iPad accelerometer or its permission sheet.

API references: [W3C Device Orientation and Motion](https://www.w3.org/TR/orientation-event/) and [WebKit iframe motion restriction](https://bugs.webkit.org/show_bug.cgi?id=221399).
