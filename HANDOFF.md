# Handoff: QR graphic auto-size computation

## Background

The `qrcode` function in `src/lib.typ` accepts a `graphic` argument (added recently) that places content centered in a cleared square in the middle of the QR code. When `graphic-size: auto` is used the plugin picks the cleared square size automatically.

The current auto-size logic in `plugin/src/qrcode.rs` (`find_safe_clear_size`) is **broken**. It uses a table of total EC-H codewords per version and solves `√(t/2 × 8)` for the side length. This formula:

1. Ignores boundary codewords — the top and bottom edges of the cleared square each partially corrupt ~`s/2` extra RS codewords, which are full symbol errors not counted by the formula.
2. Uses 100% of the EC budget with no margin, so any noise causes failure.
3. Does not account for the per-RS-block distribution — errors must be spread evenly across blocks, but a spatial cluster in the center hits some blocks harder.

Empirically: `graphic: logo` on a V4 QR encoding `"https://typst.app"` fails to scan on a phone despite the formula claiming it is safe.

## Research summary

- **fancyqr (LaTeX)**: no safe-size formula at all; clears exactly the image box in modules. User responsibility.
- **qrcode.react**: defaults to `DEFAULT_IMG_SCALE = 0.1` (10% of rendered pixel size) as a rough fallback; otherwise uses user-supplied dimensions. No analysis.
- **segno / python-qrcode / nayuki**: no embedded-logo feature.
- **No pre-calculated table exists** in any open-source project.

Industry consensus is "keep logo under ~30% of QR area" → `s ≤ √0.30 × width ≈ 0.55 × width`. A conservative but unanalysed heuristic.

## Chosen fix: brute-force calibration tool

Rather than guess, we will empirically determine the correct safe sizes with a dedicated Rust tool that:

1. Encodes test payloads at every version (1–40) at EC level H using the **exact same `qrcode` crate version** as the plugin.
2. Forces all 8 mask patterns explicitly per payload (the encoder picks by penalty score; different masks XOR the center differently, changing the error distribution).
3. Clears a centered square of increasing odd side lengths (3, 5, 7, …) filled with **white**, then repeats filled with **black** (asymmetric error cost).
4. Attempts to decode each result with **both `rxing` and `rqrr`**; only counts as success if both pass (`rxing` is ZXing-derived and representative of Android cameras; `rqrr` catches different edge cases).
5. Records the largest side length where **all** masks, **both** fill colours, and **all** tested payloads succeed.
6. Writes `plugin/safe_clear.toml` with the final per-version safe sizes.

The plugin's `build.rs` reads `safe_clear.toml` at Rust compile time and emits a constant into `OUT_DIR`; no TOML parser ships in the WASM binary.

## Payload set

For each version, test at minimum:
- Max-capacity payload at EC-H (worst case for data density near center)
- Half-capacity payload
- Short sparse payload (e.g. a URL)
- All-zero bytes payload
- All-255 bytes payload

~30 payloads per version is sufficient. More is better for confidence but diminishing returns past ~50.

## Directory layout to create

```
tools/
  compute-clear/
    Cargo.toml          # standalone binary, NOT a workspace member of plugin/
    src/
      main.rs

plugin/
  safe_clear.toml       # COMMITTED; written by compute tool; read by build.rs
  build.rs              # NEW; reads safe_clear.toml, emits OUT_DIR/safe_clear.rs
  Cargo.toml            # add [build-dependencies] toml = "0.8"
  src/
    qrcode.rs           # replace find_safe_clear_size + EC_H_CODEWORDS with include!
```

## plugin/safe_clear.toml format

```toml
# Safe odd center side-lengths (modules) per QR version 1-40 at EC level H.
# A value of 0 means no safe size exists for that version.
# Re-generate: just compute
sizes = [3, 5, 7, 9, 9, 11, 13, 15, ...]
```

## plugin/build.rs

```rust
fn main() {
    let text = std::fs::read_to_string("safe_clear.toml").unwrap();
    let table: toml::Table = text.parse().unwrap();
    let sizes: Vec<u8> = table["sizes"].as_array().unwrap()
        .iter().map(|v| v.as_integer().unwrap() as u8).collect();
    assert_eq!(sizes.len(), 40, "safe_clear.toml must have exactly 40 entries");

    let out = format!(
        "pub(crate) const SAFE_SIZES: [u8; 40] = [{}];\n",
        sizes.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
    );
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{out_dir}/safe_clear.rs"), out).unwrap();
    println!("cargo:rerun-if-changed=safe_clear.toml");
}
```

## plugin/Cargo.toml additions

```toml
[build-dependencies]
toml = "0.8"
```

## plugin/src/qrcode.rs changes

**Remove** these lines (lines 16–34):

```rust
/// Total EC codewords at level H per version (versions 1–40, index = version−1).
/// Source: ISO/IEC 18004:2015 Table 9.
const EC_H_CODEWORDS: [u16; 40] = [
    17, 28, 44, 64, 88, 112, 130, 156, 192, 224, 264, 308, 352, 384, 432, 480, 532, 588, 650, 700,
    750, 816, 900, 960, 1050, 1110, 1200, 1260, 1350, 1440, 1530, 1620, 1710, 1800, 1890, 1980,
    2100, 2220, 2310, 2430,
];

/// Largest odd side-length ≥ 3 that fits within the EC-H budget (conservative
/// lower bound regardless of actual EC level), or 0.
fn find_safe_clear_size(version: u8) -> u8 {
    let ec_budget = (EC_H_CODEWORDS[version as usize - 1] as usize / 2) * 8;
    let s_len = (ec_budget as f64).sqrt() as u8;
    match s_len {
        0..=2 => 0,
        s if s % 2 == 0 => s - 1,
        s => s,
    }
}
```

**Replace with:**

```rust
include!(concat!(env!("OUT_DIR"), "/safe_clear.rs"));

fn find_safe_clear_size(version: u8) -> u8 {
    SAFE_SIZES[version as usize - 1]
}
```

Everything else in `qrcode.rs` is unchanged.

## tools/qr-calibrate/Cargo.toml

```toml
[package]
name = "qr-calibrate"
version = "0.1.0"
edition = "2024"

[dependencies]
qrcode    = { version = "0.14", default-features = false }  # MUST match plugin/Cargo.toml
rxing     = "0.7"
rqrr      = "0.5"
rayon     = "1"
image     = "0.25"   # needed to feed bitmaps to rxing/rqrr
```

The `qrcode` version **must** be kept in sync with `plugin/Cargo.toml`. This is the entire reason the tool is in Rust.

## Parallelism

Use `rayon::par_iter` over the outer loop of `(version, mask, fill, payload)` tuples. The decode calls are independent and embarrassingly parallel. No CUDA; the bottleneck is RS arithmetic which is small integer work that saturates CPU cores efficiently.

## Running order

1. `cargo run --manifest-path tools/qr-calibrate/Cargo.toml --release`
   — writes `plugin/safe_clear.toml`
2. `just build` as normal — `build.rs` picks up `safe_clear.toml` automatically
3. Commit `plugin/safe_clear.toml` and `plugin/build.rs`

Re-run step 1 any time `qrcode` crate version changes in `plugin/Cargo.toml`.
