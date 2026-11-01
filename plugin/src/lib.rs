mod datamatrix;
mod qrcode;

use ::datamatrix::placement::PathSegment;
use serde::de::DeserializeOwned;
use wasm_minimal_protocol::{initiate_protocol, wasm_func};

type Bitmap = ::datamatrix::placement::Bitmap<bool>;

initiate_protocol!();

#[wasm_func]
pub fn encode(
    data: &[u8],
    data_type: &[u8],
    barcode: &[u8],
    options: &[u8],
    graphic_size: &[u8],
) -> Result<Vec<u8>, String> {
    let is_utf8 = match data_type {
        b"bytes" => false,
        b"str" => !data.is_ascii(),
        _ => return Err("invalid data_type".to_string()),
    };
    let (bitmap, clear_size) = match barcode {
        // aztec
        b"datamatrix" => {
            let b = cbor_read::<datamatrix::Options>(options)?.encode(data, is_utf8)?;
            (b, 0u16)
        }
        // pdf417
        b"qrcode" => {
            let gs: Option<Option<f64>> = if graphic_size.is_empty() {
                None
            } else {
                Some(cbor_read::<Option<f64>>(graphic_size)?)
            };
            cbor_read::<qrcode::Options>(options)?.encode(data, is_utf8, gs)?
        }
        _ => return Err("invalid barcode".to_string()),
    };
    Ok(bitmap_to_path(&bitmap, clear_size))
}

fn cbor_read<T: DeserializeOwned>(options: &[u8]) -> Result<T, String> {
    ciborium::de::from_reader(options).map_err(|e| match e {
        ciborium::de::Error::Semantic(_, message) => message,
        _ => "error parsing options cbor".to_string(),
    })
}

fn bitmap_to_path(bitmap: &Bitmap, clear_size: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.push_i16(bitmap.width().try_into().unwrap());
    bytes.push_i16(bitmap.height().try_into().unwrap());
    bytes.push_i16(clear_size as i16);
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

// Stub out the WASM host imports so `cargo test` links on native targets.
#[cfg(test)]
mod wasm_stubs {
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn wasm_minimal_protocol_write_args_to_buffer(_ptr: *mut u8) {}
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn wasm_minimal_protocol_send_result_to_host(
        _ptr: *const u8,
        _len: usize,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert!(encode(b"0", b"str", b"datamatrix", &[0xA0], &[]).is_ok());
    }

    #[test]
    fn test_errors() {
        assert_eq!(
            encode(b"0", b"unknown", b"", b"", &[]).unwrap_err(),
            "invalid data_type"
        );
        assert_eq!(
            encode(b"0", b"str", b"unknown", b"", &[]).unwrap_err(),
            "invalid barcode"
        );
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
