use crate::{Bitmap, Error};
use qrcode::{Color, QrCode, bits::Bits, types::QrError};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Options {
    #[serde(default)]
    versions: Version,
    #[serde(default)]
    mode: Option<Mode>,
    #[serde(default)]
    ec_level: EcLevel,
}

include!(concat!(env!("OUT_DIR"), "/safe_clear.rs"));

fn find_safe_clear_size(version: u8) -> u8 {
    SAFE_SIZES[version as usize - 1]
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Version {
    #[default]
    Normal,
    Micro,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum EcLevel {
    L,
    #[default]
    M,
    Q,
    H,
}

impl From<EcLevel> for qrcode::EcLevel {
    fn from(value: EcLevel) -> Self {
        match value {
            EcLevel::L => Self::L,
            EcLevel::M => Self::M,
            EcLevel::Q => Self::Q,
            EcLevel::H => Self::H,
        }
    }
}

impl Options {
    pub fn encode(
        &self,
        data: &[u8],
        is_utf8: bool,
        graphic_size: Option<Option<f64>>,
    ) -> Result<(Bitmap, u16), Error> {
        let valid_characters = match self.mode {
            None | Some(Mode::Byte) => true,
            Some(Mode::Numeric) => data.iter().all(u8::is_ascii_digit),
            Some(Mode::Alphanumeric) => data
                .iter()
                .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase() || b" $%*+-./:".contains(c)),
        };
        if !valid_characters {
            return Err(QrError::InvalidCharacter.into());
        }

        let ec_level = self.ec_level.into();
        let try_encode = |version| -> Result<QrCode, QrError> {
            let mut bits = Bits::new(version);
            if is_utf8 {
                bits.push_eci_designator(ECI_UTF8)?;
            }
            match self.mode {
                None => bits.push_optimal_data(data),
                Some(Mode::Numeric) => bits.push_numeric_data(data),
                Some(Mode::Alphanumeric) => bits.push_alphanumeric_data(data),
                Some(Mode::Byte) => bits.push_byte_data(data),
            }?;
            bits.push_terminator(ec_level)?;
            QrCode::with_bits(bits, ec_level)
        };

        let max_version = match self.versions {
            Version::Normal => 40,
            Version::Micro => 4,
        };
        let mut qrcode: Result<QrCode, QrError> = Err(QrError::InvalidVersion);
        for version in 1..=max_version {
            let version = match self.versions {
                Version::Normal => qrcode::Version::Normal(version),
                Version::Micro => qrcode::Version::Micro(version),
            };
            qrcode = try_encode(version);
            match qrcode {
                Err(
                    QrError::DataTooLong
                    | QrError::UnsupportedCharacterSet
                    | QrError::InvalidVersion,
                ) => (),
                _ => break,
            }
        }
        let qrcode = qrcode?;

        let width = qrcode.width();
        let mut pixels = qrcode.to_colors();

        let clear_size: u16 = 'clear: {
            let Some(size_opt) = graphic_size else {
                break 'clear 0;
            };
            let qrcode::Version::Normal(v) = qrcode.version() else {
                break 'clear 0;
            };
            let s_len = match size_opt {
                Some(r) => {
                    let ratio = r.max(0.0).min(1.0);
                    ((ratio * width as f64).floor() as u8 - 1) | 1 // odd floor
                }
                None => find_safe_clear_size(v as u8),
            };

            if s_len > 0 {
                let side = s_len as usize;
                let offset = (width - side) / 2;
                // Clear centered square
                for y in offset..(offset + side) {
                    for x in offset..(offset + side) {
                        pixels[y * width + x] = Color::Light;
                    }
                }
            }

            s_len as u16
        };

        let bitmap = Bitmap::new(pixels.into_iter().map(|c| c != Color::Light), width);
        Ok((bitmap, clear_size))
    }
}

const ECI_UTF8: u32 = 26;

impl From<QrError> for Error {
    fn from(value: QrError) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_0() {
        assert!(Options::default().encode(b"0", false, None).is_ok());
    }

    #[test]
    fn test_graphic_auto() {
        let (_, clear_size) = Options::default()
            .encode(b"https://example.com", false, Some(None))
            .unwrap();
        assert!(clear_size > 0 && clear_size % 2 == 1);
    }
}
