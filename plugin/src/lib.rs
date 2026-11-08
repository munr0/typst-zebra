mod datamatrix;
mod qrcode;

use ::datamatrix::placement::PathSegment;
use serde::{Deserialize, de::DeserializeOwned};
use wasm_minimal_protocol::{initiate_protocol, wasm_func};

type Bitmap = ::datamatrix::placement::Bitmap<bool>;

initiate_protocol!();

#[wasm_func]
pub fn encode(
    data: &[u8],
    data_type: &[u8],
    barcode: &[u8],
    options: &[u8],
    clear: &[u8],
) -> Result<Vec<u8>, String> {
    let is_utf8 = match data_type {
        b"bytes" => false,
        b"str" => !data.is_ascii(),
        _ => return Err("invalid data_type".to_string()),
    };
    let bitmap = match barcode {
        // aztec
        b"datamatrix" => cbor_read::<datamatrix::Options>(options)?.encode(data, is_utf8),
        // pdf417
        b"qrcode" => cbor_read::<qrcode::Options>(options)?.encode(data, is_utf8),
        _ => return Err("invalid barcode".to_string()),
    }?;
    let bitmap = clear_rects(&bitmap, &cbor_read::<Vec<Rect>>(clear)?)?;
    Ok(bitmap_to_path(&bitmap))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rect {
    width: i64,
    height: i64,
    x: Option<i64>,
    y: Option<i64>,
}

/// Makes the modules inside any of the rectangles light
fn clear_rects(bitmap: &Bitmap, rects: &[Rect]) -> Result<Bitmap, String> {
    let (width, height) = (bitmap.width() as i64, bitmap.height() as i64);
    // a missing `x` or `y` centres the rectangle, rounding towards the top left
    let rects: Vec<_> = rects
        .iter()
        .map(|rect| {
            let x = rect.x.unwrap_or((width - rect.width).div_euclid(2));
            let y = rect.y.unwrap_or((height - rect.height).div_euclid(2));
            (x..x + rect.width, y..y + rect.height)
        })
        .collect();
    let cleared = |x, y| {
        rects
            .iter()
            .any(|(xs, ys)| xs.contains(&x) && ys.contains(&y))
    };
    // `Bitmap::path` assumes the outline starts at a dark top left module
    if cleared(0, 0) {
        return Err("clear must not include the top left module".to_string());
    }
    let bits = bitmap.bits().iter().enumerate().map(|(i, &bit)| {
        let (x, y) = (i as i64 % width, i as i64 / width);
        bit && !cleared(x, y)
    });
    Ok(Bitmap::new(bits, bitmap.width()))
}

fn cbor_read<T: DeserializeOwned>(options: &[u8]) -> Result<T, String> {
    ciborium::de::from_reader(options).map_err(|e| match e {
        ciborium::de::Error::Semantic(_, message) => message,
        _ => "error parsing options cbor".to_string(),
    })
}

fn bitmap_to_path(bitmap: &Bitmap) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.push_i16(bitmap.width().try_into().unwrap());
    bytes.push_i16(bitmap.height().try_into().unwrap());
    for part in bitmap.path() {
        match part {
            PathSegment::Move(dx, dy) => {
                bytes.push(0);
                bytes.push_i16(dx);
                bytes.push_i16(dy);
            }
            PathSegment::Horizontal(dx) => {
                bytes.push(1);
                bytes.push_i16(dx);
            }
            PathSegment::Vertical(dy) => {
                bytes.push(2);
                bytes.push_i16(dy);
            }
            PathSegment::Close => bytes.push(3),
        }
    }
    bytes
}

trait Bytes {
    fn push_i16(&mut self, n: i16);
}

impl Bytes for Vec<u8> {
    fn push_i16(&mut self, n: i16) {
        self.extend(n.to_le_bytes());
    }
}

#[derive(Debug)]
struct Error(String);

impl From<Error> for String {
    fn from(value: Error) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert!(encode(b"0", b"str", b"datamatrix", &[0xA0], &[0x80]).is_ok());
    }

    #[test]
    fn test_errors() {
        assert_eq!(
            encode(b"0", b"unknown", b"", b"", b"").unwrap_err(),
            "invalid data_type"
        );
        assert_eq!(
            encode(b"0", b"str", b"unknown", b"", b"").unwrap_err(),
            "invalid barcode"
        );
    }

    #[test]
    fn test_clear() {
        let bitmap = qrcode::Options::default().encode(b"0", false).unwrap();
        let clear = |x, y| {
            let rect = Rect {
                width: 5,
                height: 5,
                x,
                y,
            };
            clear_rects(&bitmap, &[rect]).map(|b| b.bits().to_vec())
        };
        assert_ne!(clear(Some(8), Some(8)).unwrap(), bitmap.bits());
        assert_eq!(clear(None, None), clear(Some(8), Some(8)));
        assert!(clear(Some(-1), Some(-1)).is_err());
    }

    #[test]
    fn readme_pixel_segments() {
        let bitmap = datamatrix::Options::default()
            .encode(b"20912", false)
            .unwrap();
        assert_eq!(
            bitmap
                .bits()
                .iter()
                .enumerate()
                .filter_map(|(index, bit)| bit.then_some(index))
                .collect::<Vec<_>>(),
            &[
                0, 2, 4, 6, 8, 10, 11, 12, 14, 15, 16, 17, 19, 20, 22, 23, 24, 25, 26, 27, 30, 31,
                33, 35, 37, 39, 40, 44, 45, 46, 48, 50, 52, 54, 56, 57, 58, 59, 60, 64, 66, 67, 68,
                70, 71, 75, 78, 79, 80, 82, 83, 84, 88, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99
            ]
        );
    }
}
