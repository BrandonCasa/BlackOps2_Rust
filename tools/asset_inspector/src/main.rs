#![forbid(unsafe_code)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use bo2_assets::fastfile::{PcSignedFastfileEnvelope, inspect_pc_signed_envelope};

const HELP: &str = "Usage: bo2-asset-inspector [--] <input-file>
       bo2-asset-inspector --help

Read-only inspection of one signed PC fastfile envelope (TAff0100, version 147).
Reads at most 312 bytes; emits envelope metadata only. Authentication remains
unverified and the payload stays unread. Source paths and raw malformed fields
are omitted from diagnostics. No recursive discovery or configuration loading.

Exit codes: 0 success/help, 1 input/inspection failure, 2 invalid arguments.";

fn main() -> ExitCode {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    let input = match arguments.as_slice() {
        [help] if help == "--help" || help == "-h" => {
            return match writeln!(io::stdout().lock(), "{HELP}") {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => failure(&output_error(error)),
            };
        }
        [path] if !path.to_string_lossy().starts_with('-') => path,
        [separator, path] if separator == "--" => path,
        _ => {
            let _ = writeln!(
                io::stderr().lock(),
                "invalid arguments; use --help for usage"
            );
            return ExitCode::from(2);
        }
    };
    match inspect(Path::new(input)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => failure(&message),
    }
}

fn inspect(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(input_error)?;
    let metadata = file.metadata().map_err(input_error)?;
    if !metadata.is_file() {
        return Err("input is not a regular file".to_owned());
    }
    let envelope =
        inspect_pc_signed_envelope(&mut file, metadata.len()).map_err(|error| error.to_string())?;
    let stem_diagnostic = match path.file_stem() {
        Some(stem) if stem == OsStr::new(envelope.embedded_name()) => "match",
        Some(_) => "mismatch (diagnostic only)",
        None => "unavailable (diagnostic only)",
    };
    write_metadata(&mut io::stdout().lock(), &envelope, stem_diagnostic).map_err(output_error)
}

fn write_metadata<W: Write>(
    output: &mut W,
    envelope: &PcSignedFastfileEnvelope,
    stem_diagnostic: &str,
) -> io::Result<()> {
    let signature = envelope.signature_region();
    let remainder = envelope.opaque_remainder_region();
    writeln!(
        output,
        "inspection: envelope metadata only\n\
format: signed PC fastfile (TAff0100)\n\
version: {}\n\
flags_variant: observed zero bytes\n\
embedded_name: {:?}\n\
total_length_bytes: {}\n\
signature_region_bytes: {}..{} (opaque)\n\
opaque_remainder_offset_bytes: {}\n\
opaque_remainder_length_bytes: {}\n\
disk_stem: {stem_diagnostic}\n\
authentication: unverified\n\
payload: unread (length/completeness unverified)",
        envelope.version(),
        envelope.embedded_name(),
        envelope.total_length(),
        signature.start,
        signature.end,
        remainder.start,
        envelope.opaque_remainder_length()
    )
}

fn failure(message: &str) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "inspection failed: {message}");
    ExitCode::FAILURE
}

fn output_error(error: io::Error) -> String {
    format!("output write failed ({:?})", error.kind())
}

fn input_error(error: io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        "input not found".to_owned()
    } else {
        format!("input access failed ({:?})", error.kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_failure_is_returned_and_its_raw_message_is_redacted() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE_ERROR"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut bytes = [0; 312];
        bytes[..8].copy_from_slice(b"TAff0100");
        bytes[8..12].copy_from_slice(&147_u32.to_le_bytes());
        bytes[12..20].copy_from_slice(b"PHEEBs71");
        bytes[24] = b'x';
        let metadata = inspect_pc_signed_envelope(&mut bytes.as_slice(), 312).unwrap();
        let error = write_metadata(&mut FailingWriter, &metadata, "match").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(output_error(error), "output write failed (BrokenPipe)");
    }
}
