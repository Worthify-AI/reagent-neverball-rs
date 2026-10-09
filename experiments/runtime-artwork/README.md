# Runtime artwork capture

The original small C helper recovers decoded texture pixels at the running reference application's OpenGL interface. It does not open the game's artwork files. A separate coding agent authored it; ReAgent chat compiled and launched it against the licensed reference. This is a bounded artwork experiment, not a claim that the existing Rust engine or its earlier assets were developed with no runtime data.

The final run produced 97 captured uploads with 90 distinct RGBA payloads, with no skipped or failed events. After construction ended, a separate audit compared those pixels with the already-licensed public assets. Event 23 exactly matched the basic checker ball after vertical row reversal, including alpha 127. No original image bytes were copied to produce the recovered PNG. See PROVENANCE.json for hashes and phase boundaries.

Normal Rust material loading now routes `ball/basic-ball/basic-ball` to `data/runtime-artwork/basic-ball-captured.png`. The original asset and texture index remain retained unchanged. The embedded fallback remains its previously disclosed licensed texture. This change therefore exercises the recovered asset in the ordinary native and browser renderer, not just a fallback.

## Reproduce the helper's synthetic checks

Requires installed C compiler, desktop GL/GLX and SDL2 development headers, Python 3, and an X display:

```sh
cc -std=c11 -O2 -Wall -Wextra -Werror -fPIC -shared texcap.c -o libtexcap.so -ldl -lGL -pthread
cc -std=c11 -O2 -Wall -Wextra -Werror fixture.c -o fixture $(pkg-config --cflags --libs sdl2 gl)
mkdir synthetic
TEXCAP_DIR="$PWD/synthetic" LD_PRELOAD="$PWD/libtexcap.so" ./fixture
python3 convert.py synthetic/session-<printed-id> --fixture
```

The hidden 2×2 fixture checks exact RGB/RGBA, base/sized luminance and luminance-alpha bytes, pack state, pack/unpack PBO bindings, offset-zero uploads, NULL allocation, SDL proc-address interception and GL-error preservation. These synthetic results are not game-art recovery. Run the converter only after capture stops; `--flip-y` explicitly reverses row order for normal image orientation.

## Limits and output semantics

Single GLX context/rendering thread, level-zero border-zero 2D unsigned-byte color uploads only. No subimage/copy updates, compressed/depth/integer/alpha-only storage, meshes, fonts or audio. Luminance R is replicated into RGB; luminance-only alpha is 255, and luminance-alpha retains readback alpha. Metadata records that expansion. 64 MiB per image, 256 MiB cumulative raw-write budget, 4096 records. Raw pixels keep GL row-zero-first order unless explicitly flipped by conversion.

Pack state and buffers are restored. Application errors are retained by an interposed glGetError queue; observer errors are recorded separately. Other contexts/threads and bypassed entrypoints are outside the preservation guarantee. Readback and filesystem synchronization alter timing. Atomic, fsynced raw output precedes committed JSON metadata; orphan or temporary files are not successful records.

The complete capture remains private; this directory publishes the helper, synthetic fixture, converter and one identified capture with sanitized metadata. No compiled helper, reference executable, private task trace, credentials or internal service addresses are included.

## Licensing

Independent helper, fixture and converter: GPL-3.0-or-later under the repository root LICENSE. Recovered Neverball pixels retain GPL-2.0-or-later, copyright Robert Kooima, Jean Privat and Neverball contributors. See ../../NOTICE.md, ../../THIRD_PARTY_NOTICES.md and ../../data/COPYRIGHT.debian. The original corresponding authoring assets and permissions remain in ../../assets/neverball/source-full/. Capture does not replace upstream attribution or alter those terms.
