//! Computes permissible center clear sizes for QR codes (versions 1-40, EC level H)
//!
//! Run: just compute
//! Writes result to plugin/safe_clear.toml

use image::{GrayImage, Luma};
use indicatif::ParallelProgressIterator;
use qrcode::{
    bits::Bits,
    canvas::{Canvas, MaskPattern},
    ec, Color, EcLevel, Version,
};
use rayon::prelude::*;

// ISO 18004 byte-mode capacity at EC level H, versions 1-40
const MAX_CAP_H: [usize; 40] = [
    7, 14, 24, 34, 44, 58, 64, 84, 98, 119, 137, 155, 177, 194, 220, 250, 280, 310, 338, 382, 403,
    439, 461, 511, 535, 593, 625, 658, 698, 742, 790, 842, 898, 958, 983, 1051, 1093, 1139, 1219,
    1273,
];

const ALL_MASKS: [MaskPattern; 8] = [
    MaskPattern::Checkerboard,
    MaskPattern::HorizontalLines,
    MaskPattern::VerticalLines,
    MaskPattern::DiagonalLines,
    MaskPattern::LargeCheckerboard,
    MaskPattern::Fields,
    MaskPattern::Diamonds,
    MaskPattern::Meadow,
];

const SCALE: u32 = 4;
const QUIET: u32 = 4;

fn main() {
    let sizes: Vec<u8> = (1u8..=40)
        .collect::<Vec<_>>()
        .into_par_iter()
        .progress_count(40)
        .map(find_safe_size)
        .collect();

    let toml = format!(
        "# Numerically-determined permissible center clear sizes (in modules), QR versions 1-40 at EC level H\n\
         # Produced by compute-clear, regenerate with `just compute`\n\
         sizes = [{}]\n",
        sizes
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    std::fs::write("plugin/safe_clear.toml", toml).unwrap();
    eprintln!("Wrote plugin/safe_clear.toml");
}

fn find_safe_size(version: u8) -> u8 {
    let qr_width = 4 * version as usize + 17;

    let variants: Vec<(Vec<Color>, usize)> = generate_payloads(version)
        .into_iter()
        .flat_map(|p| encode_all_masks(&p, version))
        .filter(|(pixels, width)| {
            let img = to_gray_image(pixels, *width);
            decode_rxing(&img) && decode_rqrr(to_gray_image(pixels, *width))
        })
        .collect();

    if variants.is_empty() {
        return 0;
    }

    let mut safe = 0u8;
    let mut side = 3usize;
    while side < qr_width {
        if variants.iter().all(|(px, w)| {
            clear_and_decode(px, *w, side as u8, Color::Light)
                && clear_and_decode(px, *w, side as u8, Color::Dark)
        }) {
            safe = side as u8;
            side += 2;
        } else {
            break;
        }
    }
    safe
}

fn clear_and_decode(pixels: &[Color], width: usize, side: u8, fill: Color) -> bool {
    let s = side as usize;
    let offset = (width - s) / 2;
    let mut px = pixels.to_vec();
    for row in px[offset * width..(offset + s) * width].chunks_mut(width) {
        row[offset..offset + s].fill(fill);
    }
    let img = to_gray_image(&px, width);
    decode_rxing(&img) && decode_rqrr(img)
}

fn encode_all_masks(data: &[u8], version: u8) -> Vec<(Vec<Color>, usize)> {
    let v = Version::Normal(version as i16);
    let mut bits = Bits::new(v);
    if bits.push_byte_data(data).is_err() || bits.push_terminator(EcLevel::H).is_err() {
        return vec![];
    }
    let Ok((enc, ec_bytes)) = ec::construct_codewords(&bits.into_bytes(), v, EcLevel::H) else {
        return vec![];
    };
    let width = 4 * version as usize + 17;

    let mut base = Canvas::new(v, EcLevel::H);
    base.draw_all_functional_patterns();
    base.draw_data(&enc, &ec_bytes);

    ALL_MASKS
        .iter()
        .map(|&pattern| {
            let mut c = base.clone();
            c.apply_mask(pattern);
            (c.into_colors(), width)
        })
        .collect()
}

fn to_gray_image(pixels: &[Color], width: usize) -> GrayImage {
    let padded = width as u32 + 2 * QUIET;
    let img_size = padded * SCALE;
    let mut img = GrayImage::from_pixel(img_size, img_size, Luma([255u8]));
    for (i, &c) in pixels.iter().enumerate() {
        if c == Color::Light {
            continue;
        }
        let x = (i % width) as u32;
        let y = (i / width) as u32;
        let px = (x + QUIET) * SCALE;
        let py = (y + QUIET) * SCALE;
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                img.put_pixel(px + dx, py + dy, Luma([0u8]));
            }
        }
    }
    img
}

fn decode_rxing(img: &GrayImage) -> bool {
    rxing::helpers::detect_in_luma(img.as_raw().clone(), img.width(), img.height(), None).is_ok()
}

fn decode_rqrr(img: GrayImage) -> bool {
    let mut prepared = rqrr::PreparedImage::prepare(img);
    prepared.detect_grids().iter().any(|g| g.decode().is_ok())
}

fn generate_payloads(version: u8) -> Vec<Vec<u8>> {
    let max = MAX_CAP_H[version as usize - 1];
    let half = max / 2 + 1;

    let mut ps: Vec<Vec<u8>> = vec![
        vec![0x00; max],
        vec![0xff; max],
        vec![0x00; half],
        vec![0xff; half],
    ];

    let url: &[u8] = b"https://typst.app";
    if url.len() <= max {
        ps.push(url.to_vec());
    }

    let mut rng = Xorshift64::new(version as u64);
    while ps.len() < 64 {
        let len = (rng.next() as usize % max) + 1;
        ps.push((0..len).map(|_| rng.next() as u8).collect());
    }

    ps
}

struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        let s = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        Self(if s == 0 { 1 } else { s })
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
