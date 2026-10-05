//! Conservative signed PC fastfile envelope inspection, supported by findings 0002.
//!
//! Success establishes metadata only, never authentication, payload completeness,
//! loadability, or contents. Only the first 312 bytes are read; the signature and
//! remainder remain opaque. No embedded name is used to access another file.

use std::fmt;
use std::io::{self, Read};
use std::ops::Range;

pub const DISCRIMINATOR_LENGTH: usize = 12;
pub const ENVELOPE_LENGTH: usize = 312;
pub const SIGNATURE_OFFSET: u64 = 56;
pub const REMAINDER_OFFSET: u64 = 312;
pub const PC_VERSION: u32 = 147;

const MARKER: &[u8; 8] = b"TAff0100";
const AUTH_MARKER: &[u8; 8] = b"PHEEBs71";

/// The observed name subset, not a claim about every name accepted by the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedName {
    Empty,
    MissingTerminator,
    NonPrintableAscii,
    NonzeroPadding,
}

/// Errors deliberately retain no source paths or raw marker/name/signature bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeError {
    Io { kind: io::ErrorKind },
    Truncated { needed: u64, available: u64 },
    UnsupportedMarker,
    UnsupportedVersion { version: u32 },
    MalformedAuthenticationMarker,
    UnsupportedFlags,
    UnsupportedName(UnsupportedName),
    InconsistentLength { declared: u64, observed_prefix: u64 },
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { kind } => write!(f, "envelope read failed ({kind:?})"),
            Self::Truncated { needed, available } => write!(
                f,
                "truncated envelope: need {needed} bytes, have {available}"
            ),
            Self::UnsupportedMarker => f.write_str("unsupported fastfile marker"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported fastfile version ({version})")
            }
            Self::MalformedAuthenticationMarker => {
                f.write_str("malformed supported-envelope authentication marker")
            }
            Self::UnsupportedFlags => f.write_str("unsupported nonzero flags variant"),
            Self::UnsupportedName(reason) => {
                write!(f, "unsupported embedded-name subset ({reason:?})")
            }
            Self::InconsistentLength {
                declared,
                observed_prefix,
            } => write!(
                f,
                "reader/length inconsistency: declared {declared}, observed prefix {observed_prefix}"
            ),
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Permitted metadata for the exact marker/version and zero-flags/name subset.
///
/// The name allocation is at most 31 bytes. No signature or payload is retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcSignedFastfileEnvelope {
    embedded_name: String,
    total_length: u64,
    remainder_length: u64,
}

impl PcSignedFastfileEnvelope {
    pub fn version(&self) -> u32 {
        PC_VERSION
    }

    pub fn embedded_name(&self) -> &str {
        &self.embedded_name
    }

    pub fn total_length(&self) -> u64 {
        self.total_length
    }

    pub fn signature_region(&self) -> Range<u64> {
        SIGNATURE_OFFSET..REMAINDER_OFFSET
    }

    pub fn opaque_remainder_region(&self) -> Range<u64> {
        REMAINDER_OFFSET..self.total_length
    }

    pub fn opaque_remainder_length(&self) -> u64 {
        self.remainder_length
    }
}

/// Inspect from offset zero using the length of the same opened source.
///
/// A 12-byte discriminator is read first; unsupported marker/version returns
/// immediately. Supported inputs complete the 312-byte envelope using one fixed
/// buffer. Short reads and interruptions are handled. Nothing past byte 312 is
/// requested, even for `u64::MAX` lengths. The only allocation is a valid name of
/// at most 31 bytes. The signature is neither interpreted nor emitted.
///
/// A prefix EOF matching `total_length` is truncation; EOF disagreeing with it,
/// or prefix bytes extending past it, is a reader/length inconsistency. A length
/// of 312 can succeed with an empty remainder. Larger declared lengths cannot be
/// verified without reading the payload; source mutation and payload EOF beyond
/// the envelope are outside this metadata API's guarantees.
pub fn inspect_pc_signed_envelope<R: Read + ?Sized>(
    reader: &mut R,
    total_length: u64,
) -> Result<PcSignedFastfileEnvelope, EnvelopeError> {
    let mut envelope = [0_u8; ENVELOPE_LENGTH];
    let mut consumed = 0;
    read_stage(
        reader,
        &mut envelope,
        &mut consumed,
        DISCRIMINATOR_LENGTH,
        total_length,
    )?;
    if &envelope[..8] != MARKER {
        return Err(EnvelopeError::UnsupportedMarker);
    }
    let version = u32::from_le_bytes([envelope[8], envelope[9], envelope[10], envelope[11]]);
    if version != PC_VERSION {
        return Err(EnvelopeError::UnsupportedVersion { version });
    }
    read_stage(
        reader,
        &mut envelope,
        &mut consumed,
        ENVELOPE_LENGTH,
        total_length,
    )?;
    if &envelope[12..20] != AUTH_MARKER {
        return Err(EnvelopeError::MalformedAuthenticationMarker);
    }
    if envelope[20..24].iter().any(|&byte| byte != 0) {
        return Err(EnvelopeError::UnsupportedFlags);
    }
    let name_field = &envelope[24..56];
    let terminator =
        name_field
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(EnvelopeError::UnsupportedName(
                UnsupportedName::MissingTerminator,
            ))?;
    if terminator == 0 {
        return Err(EnvelopeError::UnsupportedName(UnsupportedName::Empty));
    }
    if name_field[..terminator]
        .iter()
        .any(|byte| !(b' '..=b'~').contains(byte))
    {
        return Err(EnvelopeError::UnsupportedName(
            UnsupportedName::NonPrintableAscii,
        ));
    }
    if name_field[terminator..].iter().any(|&byte| byte != 0) {
        return Err(EnvelopeError::UnsupportedName(
            UnsupportedName::NonzeroPadding,
        ));
    }
    let embedded_name = String::from_utf8(name_field[..terminator].to_vec())
        .map_err(|_| EnvelopeError::UnsupportedName(UnsupportedName::NonPrintableAscii))?;
    let remainder_length =
        total_length
            .checked_sub(REMAINDER_OFFSET)
            .ok_or(EnvelopeError::InconsistentLength {
                declared: total_length,
                observed_prefix: REMAINDER_OFFSET,
            })?;
    // Explicit range arithmetic remains checked even for maximum declared length.
    if REMAINDER_OFFSET.checked_add(remainder_length) != Some(total_length) {
        return Err(EnvelopeError::InconsistentLength {
            declared: total_length,
            observed_prefix: REMAINDER_OFFSET,
        });
    }
    Ok(PcSignedFastfileEnvelope {
        embedded_name,
        total_length,
        remainder_length,
    })
}

fn read_stage<R: Read + ?Sized>(
    reader: &mut R,
    buffer: &mut [u8; ENVELOPE_LENGTH],
    consumed: &mut usize,
    target: usize,
    declared: u64,
) -> Result<(), EnvelopeError> {
    while *consumed < target {
        match reader.read(&mut buffer[*consumed..target]) {
            Ok(0) => {
                let available = *consumed as u64;
                return if available == declared {
                    Err(EnvelopeError::Truncated {
                        needed: target as u64,
                        available,
                    })
                } else {
                    Err(EnvelopeError::InconsistentLength {
                        declared,
                        observed_prefix: available,
                    })
                };
            }
            Ok(count) => {
                let next = consumed.checked_add(count).filter(|&next| next <= target);
                *consumed = next.ok_or(EnvelopeError::Io {
                    kind: io::ErrorKind::InvalidData,
                })?;
                if *consumed as u64 > declared {
                    return Err(EnvelopeError::InconsistentLength {
                        declared,
                        observed_prefix: *consumed as u64,
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(EnvelopeError::Io { kind: error.kind() }),
        }
    }
    Ok(())
}
