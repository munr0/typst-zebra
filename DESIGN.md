# Design: QR Code Center Graphic

## Overview

Add support for placing a centered graphic (image or arbitrary content) inside a QR code by clearing a rectangular region of modules. This is a QR-code-only feature; Data Matrix is not supported.

## Approach

Modules are cleared **in Rust, at the pixel grid level**, before the vector path is computed. This produces a clean gap in the rendered curve rather than an overlay on top of dark modules.

The interception point is **after `qrcode.to_colors()` and before `Bitmap::new()`** in `qrcode.rs`. At this stage the data exists as a flat row-major `Vec<Color>` which can be mutated directly.

```rust
// clear region: set modules to Color::Light
for y in y0..y0 + s {
    for x in x0..x0 + s {
        colors[y * width + x] = Color::Light;
    }
}
let bitmap = Bitmap::new(colors.into_iter().map(|c| c != Color::Light), width);
```

## Safe Zone Computation

### Structural modules (never clear)

The following are recovered by QR decoders geometrically, not via error correction. Clearing any of them is fatal:

- **Finder patterns**: 9×9 area (including separator) in each of the three corners
- **Timing patterns**: row 6 and column 6
- **Format information strips**: adjacent to each finder pattern
- **Alignment patterns**: version-specific positions from QR spec Annex E (versions 2–40)
- **Dark module**: position `(8, 4*N+9)` where N is the QR version number

### Error correction capacity

QR level H can recover up to 30% of damaged codewords. The number of EC codewords at level H per version is embedded as a `const [u16; 40]` table. The bit budget is:

```
ec_bit_budget = floor(total_ec_codewords_H[version] / 2) * 8
```

A worst-case assumption is used: every module in the clear zone is assumed to be Dark (i.e. maximum possible errors). This is conservative — in practice roughly half will be Light — so there is a comfortable safety margin.

### Search algorithm

```
for s in (3..=width).step_by(2).rev():   // largest odd first
    x0 = (width - s) / 2
    y0 = (width - s) / 2
    if any structural module lies within rect(x0, y0, s, s): continue
    if s * s <= ec_bit_budget: return s
return 0   // no safe clear zone exists
```

The result `s` is the side length in modules of the cleared square.

### When `graphic-size` is a ratio

```
requested = floor(qr_width * ratio)
s = largest odd integer ≤ requested that passes the structural and EC checks
```

The result is always clamped to the structurally safe maximum regardless of the user-specified ratio.

## Protocol Extension

The return byte buffer is extended from 4 bytes to 6 bytes:

```
[width: i16][height: i16][clear-size: i16][...path segments...]
```

`clear-size` is the value of `s` (in modules), or 0 if no graphic was requested. Typst reads this value and uses it to size the graphic overlay.

## Typst API

### `qrcode` signature additions

```typst
/// - graphic (none, content): Content placed centered over the cleared region.
///   EC level defaults to `"h"` when a graphic is provided.
/// - graphic-size (auto, ratio): Fraction of QR width to target for the clear zone.
///   `auto` uses the largest structurally safe odd×odd square at the given EC level.
///   Always clamped to the structurally safe maximum.
#let qrcode(
  data,
  options: (:),
  quiet-zone: 0,
  width: auto,
  height: auto,
  module-size: auto,
  fill: black,
  background-fill: none,
  graphic: none,          // new
  graphic-size: auto,     // new
) = ...
```

`graphic: none` is the default; when `none`, the Rust path is entirely unchanged and no protocol extension bytes are read. `graphic-size` is ignored when `graphic` is `none`.

### EC level defaulting

When `graphic != none` and the user has not explicitly set `ec-level` in `options`, Typst injects `ec-level: "h"` into the options dict before forwarding it to Rust. This means a user never needs to remember to set it:

```typst
// these are equivalent:
#qrcode("hello", graphic: image("logo.png"))
#qrcode("hello", graphic: image("logo.png"), options: (ec-level: "h"))
```

If the user explicitly sets a lower EC level alongside a graphic, that choice is respected without warning — they are assumed to know the trade-off.

### Graphic sizing in Typst

After `encode()` returns, `s` is read from the protocol buffer. The graphic is placed at exactly `s * module-size` in both dimensions:

```typst
let s = clear_size_from_protocol   // integer modules
// after module-size is resolved:
if graphic != none {
  place(center + horizon,
    box(width: s * module-size, height: s * module-size, graphic))
}
```

This ensures the rendered graphic aligns perfectly to the module grid with no approximation.

### Usage examples

```typst
// Auto-sized — Rust computes the largest safe zone
#qrcode("https://example.com", options: (ec-level: "h"),
  graphic: image("logo.png"))

// Explicit size cap — Rust finds largest safe odd s ≤ 25% of QR width
#qrcode("https://example.com", options: (ec-level: "h"),
  graphic: image("logo.png"),
  graphic-size: 25%)

// Any content — not just images
#qrcode("https://example.com", options: (ec-level: "h"),
  graphic: box(fill: white, inset: 2pt, text(size: 6pt)[AB]))
```

## Backwards Compatibility

### API
Both new parameters have safe defaults (`graphic: none`, `graphic-size: auto`). All existing call sites are unaffected.

### Protocol
The byte layout changes from `[width][height][segments…]` to `[width][height][clear-size][segments…]`. The WASM binary and `.typ` files are always distributed together as a single package version so users can never mix old and new. A semver minor version bump is required.

### `deny_unknown_fields` and option separation

`Options` in `qrcode.rs` is decorated with `#[serde(deny_unknown_fields)]`. If `graphic-size` were added to `Options` as a field, a user could accidentally pass it inside the `options: (:)` dict:

```typst
// wrong — bypasses Typst-side ratio→modules conversion
#qrcode("hello", options: (graphic-size: 0.3))
```

This would reach Rust as a raw float without the structural safety check being applied from the Typst side, producing a subtly wrong result with no error.

**Resolution**: `graphic_size` must never be added to the user-facing `Options` struct. It is passed as a separate WASM argument (a new `graphic_size` parameter to the `encode` function, encoded as its own byte argument) so that `deny_unknown_fields` continues to reject it if placed inside `options: (...)`.

## Implementation Checklist

- [ ] `qrcode.rs`: Add `graphic_size: Option<f64>` to `Options`
- [ ] `qrcode.rs`: Embed `const EC_H_CODEWORDS: [u16; 40]` from QR spec
- [ ] `qrcode.rs`: Implement `compute_structural_mask(version, width) -> Vec<bool>`
  - Finder patterns (×3)
  - Timing patterns
  - Format info strips
  - Alignment patterns (lookup table from QR spec Annex E)
  - Dark module
- [ ] `qrcode.rs`: Implement `find_safe_clear_size(width, version, max_s) -> u16`
- [ ] `qrcode.rs`: Mutate `Vec<Color>` to clear the zone before `Bitmap::new()`
- [ ] `lib.rs`: Extend `bitmap_to_path` to prepend `clear_size: i16` as bytes 4–5
- [ ] `generic.typ`: Read `clear_size` from protocol (byte index 4–5)
- [ ] `generic.typ`: Accept `graphic` and `graphic-size` parameters, forward `graphic-size` as float in CBOR options, place graphic after curve rendering
- [ ] `lib.typ`: Add `graphic` and `graphic-size` to `qrcode()` signature and doc comments
