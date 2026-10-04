//! Versioned notebook payloads; ordinary JS and native IPC do not use this codec.

use base64::Engine;
use flate2::{Compression, bufread::GzDecoder, write::GzEncoder};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write};

pub const PAYLOAD_VERSION: &str = "CMV2";
pub const MIN_COMPRESSION_BYTES: usize = 1024;
/// Maximum serialized bytes accepted by the default decoder (not object heap size).
pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;

#[derive(thiserror::Error, Debug)]
pub enum PayloadError {
    #[error("payload decode failed: expected CMV2:R:<base64> or CMV2:G:<base64>")]
    InvalidHeader,
    #[error("unsupported payload version: {0}")]
    UnsupportedVersion(String),
    #[error("unsupported payload encoding: {0}")]
    UnsupportedEncoding(String),
    #[error("base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("gzip processing failed: {0}")]
    Gzip(#[from] std::io::Error),
    #[error("postcard processing failed: {0}")]
    Postcard(#[from] postcard::Error),
    #[error("payload exceeds the decoded size limit of {limit} bytes")]
    TooLarge { limit: usize },
    #[error("payload contains trailing data")]
    TrailingData,
}

/// Encode any notebook scene, animation, or command using postcard and Base64.
/// Below 1 KiB gzip is skipped; at or above 1 KiB gzip level 1 is used.
/// R is raw postcard, G is gzip. No legacy payload formats are emitted.
pub fn encode_payload<T: Serialize>(value: &T) -> Result<String, PayloadError> {
    let bytes = postcard::to_allocvec(value)?;
    let (encoding, data) = if bytes.len() >= MIN_COMPRESSION_BYTES {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&bytes)?;
        ("G", encoder.finish()?)
    } else {
        ("R", bytes)
    };
    Ok(format!(
        "{PAYLOAD_VERSION}:{encoding}:{}",
        base64::engine::general_purpose::STANDARD.encode(data)
    ))
}

/// Decode a CMV2 notebook payload, accepting at most 64 MiB of postcard bytes.
/// Old unprefixed and CMV1 payloads are rejected; sender/receiver versions must match.
pub fn decode_payload<T: DeserializeOwned>(payload: &str) -> Result<T, PayloadError> {
    decode_payload_with_limit(payload, DEFAULT_MAX_PAYLOAD_BYTES)
}

/// Decode with an explicit serialized-byte limit, checked before deserialization.
/// A separate wire-size preflight bounds Base64 allocation. Gzip is allowed
/// twice the decoded limit plus 1 KiB of framing, so incompressible data is not
/// assumed to shrink. Output is read only up to limit + one detection byte.
pub fn decode_payload_with_limit<T: DeserializeOwned>(
    payload: &str,
    max_decoded_bytes: usize,
) -> Result<T, PayloadError> {
    let (version, body) = payload
        .trim()
        .split_once(':')
        .ok_or(PayloadError::InvalidHeader)?;
    if version != PAYLOAD_VERSION {
        return Err(PayloadError::UnsupportedVersion(version.into()));
    }
    let (encoding, encoded) = body.split_once(':').ok_or(PayloadError::InvalidHeader)?;
    if !matches!(encoding, "R" | "G") {
        return Err(PayloadError::UnsupportedEncoding(encoding.into()));
    }
    let max_wire_bytes = if encoding == "G" {
        max_decoded_bytes
            .saturating_mul(2)
            .saturating_add(MIN_COMPRESSION_BYTES)
    } else {
        max_decoded_bytes
    };
    if encoded.len() > max_wire_bytes.div_ceil(3).saturating_mul(4) {
        return Err(PayloadError::TooLarge {
            limit: max_decoded_bytes,
        });
    }
    let data = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    let bytes = if encoding == "G" {
        let mut decoder = GzDecoder::new(data.as_slice());
        let mut bytes = Vec::with_capacity(data.len().min(max_decoded_bytes));
        (&mut decoder)
            .take((max_decoded_bytes as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() > max_decoded_bytes {
            return Err(PayloadError::TooLarge {
                limit: max_decoded_bytes,
            });
        }
        if !decoder.get_ref().is_empty() {
            return Err(PayloadError::TrailingData);
        }
        bytes
    } else {
        if data.len() > max_decoded_bytes {
            return Err(PayloadError::TooLarge {
                limit: max_decoded_bytes,
            });
        }
        data
    };
    let (value, remaining) = postcard::take_from_bytes(&bytes)?;
    if !remaining.is_empty() {
        return Err(PayloadError::TrailingData);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ViewerCommand;
    use cosmol_viewer_core::scene::{Animation, Scene};

    #[test]
    fn small_commands_use_raw_postcard() {
        let command = ViewerCommand::CameraParameterLogging { enabled: true };
        let payload = encode_payload(&command).unwrap();
        assert!(payload.starts_with("CMV2:R:"));
        assert_eq!(payload.len(), 11);
        assert!(matches!(
            decode_payload(&payload).unwrap(),
            ViewerCommand::CameraParameterLogging { enabled: true }
        ));
    }

    #[test]
    fn scenes_and_animations_share_the_same_format() {
        let scene = Scene::new();
        let payload = encode_payload(&scene).unwrap();
        let decoded: Scene = decode_payload(&payload).unwrap();
        assert_eq!(
            postcard::to_allocvec(&scene).unwrap(),
            postcard::to_allocvec(&decoded).unwrap()
        );
        let mut animation = Animation::new(0.1, 1, false);
        for _ in 0..40 {
            animation.add_frame(scene.clone());
        }
        let payload = encode_payload(&animation).unwrap();
        assert!(payload.starts_with("CMV2:G:"));
        let decoded: Animation = decode_payload(&payload).unwrap();
        assert_eq!(
            postcard::to_allocvec(&animation).unwrap(),
            postcard::to_allocvec(&decoded).unwrap()
        );
    }

    #[test]
    fn compression_threshold_is_inclusive() {
        let below = vec![0_u8; 1021]; // postcard length prefix is two bytes.
        let at = vec![0_u8; 1022];
        assert_eq!(postcard::to_allocvec(&below).unwrap().len(), 1023);
        assert_eq!(postcard::to_allocvec(&at).unwrap().len(), 1024);
        assert!(encode_payload(&below).unwrap().starts_with("CMV2:R:"));
        assert!(encode_payload(&at).unwrap().starts_with("CMV2:G:"));
    }

    #[test]
    fn large_payload_uses_gzip_without_testing_compression_ratio() {
        let mut seed = 123456789_u32;
        let bytes: Vec<u8> = (0..8192)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        let payload = encode_payload(&bytes).unwrap();
        assert!(payload.starts_with("CMV2:G:"));
        assert_eq!(decode_payload::<Vec<u8>>(&payload).unwrap(), bytes);
    }

    #[test]
    fn old_versions_and_unknown_encodings_are_rejected() {
        for payload in ["CMV1:AAAA", "CMV99:R:AAAA"] {
            assert!(matches!(
                decode_payload::<Vec<u8>>(payload),
                Err(PayloadError::UnsupportedVersion(_))
            ));
        }
        assert!(matches!(
            decode_payload::<Vec<u8>>("AAAA"),
            Err(PayloadError::InvalidHeader)
        ));
        assert!(matches!(
            decode_payload::<Vec<u8>>("CMV2:X:AAAA"),
            Err(PayloadError::UnsupportedEncoding(_))
        ));
    }

    #[test]
    fn invalid_base64_gzip_and_postcard_are_errors() {
        assert!(matches!(
            decode_payload::<u8>("CMV2:R:!"),
            Err(PayloadError::Base64(_))
        ));
        assert!(matches!(
            decode_payload::<u8>("CMV2:G:AAAA"),
            Err(PayloadError::Gzip(_))
        ));
        assert!(matches!(
            decode_payload::<u8>("CMV2:R:"),
            Err(PayloadError::Postcard(_))
        ));
        assert_eq!(decode_payload::<()>("CMV2:R:").unwrap(), ());
    }

    #[test]
    fn raw_and_gzip_limits_allow_exact_size_but_reject_expansion() {
        for bytes in [vec![1_u8; 16], vec![0_u8; 8192]] {
            let length = postcard::to_allocvec(&bytes).unwrap().len();
            let payload = encode_payload(&bytes).unwrap();
            assert_eq!(
                decode_payload_with_limit::<Vec<u8>>(&payload, length).unwrap(),
                bytes
            );
            assert!(matches!(
                decode_payload_with_limit::<Vec<u8>>(&payload, length - 1),
                Err(PayloadError::TooLarge { .. })
            ));
        }
        let bomb = encode_payload(&vec![0_u8; 8192]).unwrap();
        assert!(matches!(
            decode_payload_with_limit::<Vec<u8>>(&bomb, 32),
            Err(PayloadError::TooLarge { limit: 32 })
        ));
        let huge_wire = format!("CMV2:R:{}", "A".repeat(100));
        assert!(matches!(
            decode_payload_with_limit::<Vec<u8>>(&huge_wire, 8),
            Err(PayloadError::TooLarge { limit: 8 })
        ));
    }

    #[test]
    fn corrupted_gzip_and_trailing_data_are_rejected() {
        let payload = encode_payload(&vec![0_u8; 2048]).unwrap();
        let mut compressed = base64::engine::general_purpose::STANDARD
            .decode(payload.strip_prefix("CMV2:G:").unwrap())
            .unwrap();
        *compressed.last_mut().unwrap() ^= 1;
        let broken = format!(
            "CMV2:G:{}",
            base64::engine::general_purpose::STANDARD.encode(compressed)
        );
        assert!(matches!(
            decode_payload::<Vec<u8>>(&broken),
            Err(PayloadError::Gzip(_))
        ));
        let raw = format!(
            "CMV2:R:{}",
            base64::engine::general_purpose::STANDARD.encode([1, 2])
        );
        assert!(matches!(
            decode_payload::<u8>(&raw),
            Err(PayloadError::TrailingData)
        ));
        let mut compressed = base64::engine::general_purpose::STANDARD
            .decode(payload.strip_prefix("CMV2:G:").unwrap())
            .unwrap();
        compressed.push(0);
        let extra = format!(
            "CMV2:G:{}",
            base64::engine::general_purpose::STANDARD.encode(compressed)
        );
        assert!(matches!(
            decode_payload::<Vec<u8>>(&extra),
            Err(PayloadError::TrailingData)
        ));
    }
}
