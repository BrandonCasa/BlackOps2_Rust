#![forbid(unsafe_code)]

use std::io::{self, Read};

use bo2_assets::fastfile::{
    ENVELOPE_LENGTH, EnvelopeError, PC_VERSION, UnsupportedName, inspect_pc_signed_envelope,
};

// Entirely synthetic: protocol constants, a user-created name and arbitrary
// signature-region bytes. No original sample bytes or payload fixtures are used.
fn envelope(name: &str) -> [u8; ENVELOPE_LENGTH] {
    assert!(name.len() <= 31);
    let mut bytes = [0; ENVELOPE_LENGTH];
    bytes[..8].copy_from_slice(b"TAff0100");
    bytes[8..12].copy_from_slice(&PC_VERSION.to_le_bytes());
    bytes[12..20].copy_from_slice(b"PHEEBs71");
    bytes[24..24 + name.len()].copy_from_slice(name.as_bytes());
    for (index, byte) in bytes[56..].iter_mut().enumerate() {
        *byte = (index as u8).wrapping_mul(17).wrapping_add(3);
    }
    bytes
}

#[test]
fn parses_exact_metadata_with_empty_or_unverified_remainder() {
    for length in [312, 704, u64::MAX] {
        let bytes = envelope("synthetic_zone");
        let parsed = inspect_pc_signed_envelope(&mut bytes.as_slice(), length).unwrap();
        assert_eq!(parsed.version(), 147);
        assert_eq!(parsed.embedded_name(), "synthetic_zone");
        assert_eq!(parsed.total_length(), length);
        assert_eq!(parsed.signature_region(), 56..312);
        assert_eq!(parsed.opaque_remainder_region(), 312..length);
        assert_eq!(parsed.opaque_remainder_length(), length - 312);
        assert_eq!(
            parsed.opaque_remainder_length().checked_add(312),
            Some(length)
        );
    }
}

#[test]
fn every_truncation_is_typed_with_the_stage_and_actual_available_bytes() {
    let bytes = envelope("synthetic_zone");
    for length in 0..ENVELOPE_LENGTH {
        let mut reader = PrefixReader::new(&bytes[..length], usize::MAX);
        assert_eq!(
            inspect_pc_signed_envelope(&mut reader, length as u64),
            Err(EnvelopeError::Truncated {
                needed: if length < 12 { 12 } else { 312 },
                available: length as u64,
            }),
            "truncation at {length}"
        );
        assert_eq!(reader.position, length);
        assert!(
            reader
                .requests
                .iter()
                .all(|&(offset, count)| offset + count <= 312)
        );
    }
}

#[test]
fn alternate_markers_are_rejected_after_only_the_discriminator() {
    for marker in [
        b"TAffu100",
        b"TAffs100",
        b"TAffx100",
        b"TAsvu100",
        b"ABff0100",
        b"TAff0101",
        b"PRIVATE!",
    ] {
        let mut bytes = envelope("fixture");
        bytes[..8].copy_from_slice(marker);
        let mut reader = PrefixReader::new(&bytes, usize::MAX);
        assert_eq!(
            inspect_pc_signed_envelope(&mut reader, 312),
            Err(EnvelopeError::UnsupportedMarker)
        );
        assert_eq!(reader.requests, [(0, 12)]);
        assert_eq!(reader.position, 12);
    }
}

#[test]
fn versions_and_byte_swapped_147_are_rejected_without_layout_fallback() {
    for version in [146, 148, 147_u32.swap_bytes(), u32::MAX] {
        let mut bytes = envelope("fixture");
        bytes[8..12].copy_from_slice(&version.to_le_bytes());
        let mut reader = PrefixReader::new(&bytes, usize::MAX);
        assert_eq!(
            inspect_pc_signed_envelope(&mut reader, 312),
            Err(EnvelopeError::UnsupportedVersion { version })
        );
        assert_eq!(reader.position, 12);
        assert_eq!(reader.requests, [(0, 12)]);
    }
}

#[test]
fn auth_marker_mismatch_is_malformed_supported_structure() {
    let mut bytes = envelope("fixture");
    bytes[12..20].copy_from_slice(b"PRIVATE!");
    assert_eq!(
        inspect_pc_signed_envelope(&mut bytes.as_slice(), 312),
        Err(EnvelopeError::MalformedAuthenticationMarker)
    );
}

#[test]
fn any_nonzero_flag_byte_is_an_unsupported_variant() {
    for offset in 20..24 {
        let mut bytes = envelope("fixture");
        bytes[offset] = 1;
        assert_eq!(
            inspect_pc_signed_envelope(&mut bytes.as_slice(), 312),
            Err(EnvelopeError::UnsupportedFlags)
        );
    }
}

#[test]
fn names_are_limited_to_the_observed_subset() {
    let cases = [
        ([0; 32], UnsupportedName::Empty),
        ([b'x'; 32], UnsupportedName::MissingTerminator),
        (
            {
                let mut name = [0; 32];
                name[0] = b'\n';
                name
            },
            UnsupportedName::NonPrintableAscii,
        ),
        (
            {
                let mut name = [0; 32];
                name[0] = 0x7f;
                name
            },
            UnsupportedName::NonPrintableAscii,
        ),
        (
            {
                let mut name = [0; 32];
                name[0] = 0xff;
                name
            },
            UnsupportedName::NonPrintableAscii,
        ),
        (
            {
                let mut name = [0; 32];
                name[0] = b'x';
                name[2] = b'y';
                name
            },
            UnsupportedName::NonzeroPadding,
        ),
    ];
    for (name, reason) in cases {
        let mut bytes = envelope("fixture");
        bytes[24..56].copy_from_slice(&name);
        assert_eq!(
            inspect_pc_signed_envelope(&mut bytes.as_slice(), 312),
            Err(EnvelopeError::UnsupportedName(reason))
        );
    }
}

#[test]
fn longest_name_printable_boundaries_and_path_like_name_are_only_metadata() {
    for name in [
        "1234567890123456789012345678901",
        " ~",
        "../synthetic/other",
    ] {
        let bytes = envelope(name);
        let parsed = inspect_pc_signed_envelope(&mut bytes.as_slice(), 312).unwrap();
        assert_eq!(parsed.embedded_name(), name);
        assert!(parsed.embedded_name().len() <= 31);
    }
}

#[test]
fn signature_bytes_have_no_effect_on_metadata_success() {
    let baseline = envelope("fixture");
    let expected = inspect_pc_signed_envelope(&mut baseline.as_slice(), 704).unwrap();
    for fill in [0, 1, 0xff] {
        let mut changed = baseline;
        changed[56..312].fill(fill);
        assert_eq!(
            inspect_pc_signed_envelope(&mut changed.as_slice(), 704).unwrap(),
            expected
        );
    }
}

#[test]
fn reads_are_staged_capped_and_never_touch_the_payload() {
    let bytes = envelope("fixture");
    let mut reader = PrefixReader::new(&bytes, usize::MAX);
    inspect_pc_signed_envelope(&mut reader, u64::MAX).unwrap();
    assert_eq!(reader.requests, [(0, 12), (12, 300)]);
    assert_eq!(reader.position, 312);
}

#[test]
fn short_reads_and_interrupted_reads_complete_the_same_metadata() {
    let bytes = envelope("fixture");
    let mut reader = PrefixReader::new(&bytes, 3);
    reader.interrupt_once = true;
    let parsed = inspect_pc_signed_envelope(&mut reader, 312).unwrap();
    assert_eq!(parsed.embedded_name(), "fixture");
    assert_eq!(reader.position, 312);
    assert!(
        reader
            .requests
            .iter()
            .all(|&(offset, count)| offset + count <= 312)
    );
}

#[test]
fn injected_io_errors_are_typed_and_redact_the_original_message() {
    let bytes = envelope("fixture");
    for failure_at in [0, 12, 100, 311] {
        let mut reader = PrefixReader::new(&bytes, 1);
        reader.failure_at = Some(failure_at);
        let error = inspect_pc_signed_envelope(&mut reader, 312).unwrap_err();
        assert_eq!(
            error,
            EnvelopeError::Io {
                kind: io::ErrorKind::PermissionDenied
            }
        );
        assert!(!error.to_string().contains("PRIVATE_PATH"));
        assert!(!format!("{error:?}").contains("PRIVATE_PATH"));
    }
}

#[test]
fn inconsistent_declared_lengths_are_detected_within_the_envelope() {
    let bytes = envelope("fixture");
    for (actual, declared) in [
        (0, 312),
        (11, 12),
        (100, 312),
        (311, u64::MAX),
        (312, 0),
        (312, 311),
    ] {
        let mut reader = PrefixReader::new(&bytes[..actual], 1);
        assert!(matches!(
            inspect_pc_signed_envelope(&mut reader, declared),
            Err(EnvelopeError::InconsistentLength { declared: reported, .. }) if reported == declared
        ));
        assert!(reader.position <= 312);
    }
}

#[test]
fn invalid_read_count_is_reported_without_panicking() {
    struct InvalidReader;
    impl Read for InvalidReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Ok(usize::MAX)
        }
    }
    assert_eq!(
        inspect_pc_signed_envelope(&mut InvalidReader, 312),
        Err(EnvelopeError::Io {
            kind: io::ErrorKind::InvalidData
        })
    );
}

struct PrefixReader<'a> {
    bytes: &'a [u8],
    position: usize,
    step: usize,
    requests: Vec<(usize, usize)>,
    interrupt_once: bool,
    failure_at: Option<usize>,
}

impl<'a> PrefixReader<'a> {
    fn new(bytes: &'a [u8], step: usize) -> Self {
        Self {
            bytes,
            position: 0,
            step,
            requests: Vec::new(),
            interrupt_once: false,
            failure_at: None,
        }
    }
}

impl Read for PrefixReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        assert!(self.position < 312, "payload read attempted");
        assert!(
            self.position + buffer.len() <= 312,
            "request reaches payload"
        );
        self.requests.push((self.position, buffer.len()));
        if self.interrupt_once {
            self.interrupt_once = false;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        if self.failure_at == Some(self.position) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PRIVATE_PATH must remain redacted",
            ));
        }
        let count = self
            .step
            .min(buffer.len())
            .min(self.bytes.len() - self.position);
        buffer[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
