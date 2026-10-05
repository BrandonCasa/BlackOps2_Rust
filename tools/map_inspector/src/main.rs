#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use bo2_assets::geometry_snapshot::{GeometrySnapshot, SnapshotError, read_geometry_snapshot};

const HELP: &str = "Usage: bo2-map-inspector [--] <diagnostic-snapshot-file>
       bo2-map-inspector --help

Read-only inspection of one project-owned diagnostic geometry exchange snapshot v1.
This is not a BO2 disk format. Emits bounded counts/categories/status only; no
positions, index values, bounds, paths or names. Source authenticity, rendering
and original-game fidelity remain unverified. Fixed resource policies apply.
No recursive discovery, configuration loading, automatic originals or output files.

Exit codes: 0 success/help, 1 input/inspection/output failure, 2 invalid arguments.";

fn main() -> ExitCode {
    let action = match arguments(env::args_os().skip(1)) {
        Ok(action) => action,
        Err(()) => {
            let _ = writeln!(
                io::stderr().lock(),
                "invalid arguments; use --help for usage"
            );
            return ExitCode::from(2);
        }
    };
    let result = match action {
        Action::Help => writeln!(io::stdout().lock(), "{HELP}").map_err(output_error),
        Action::Inspect(path) => inspect(Path::new(&path)),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "inspection failed: {error}");
            ExitCode::FAILURE
        }
    }
}

enum Action {
    Help,
    Inspect(OsString),
}

fn arguments(mut arguments: impl Iterator<Item = OsString>) -> Result<Action, ()> {
    match (arguments.next(), arguments.next(), arguments.next()) {
        (Some(help), None, None) if help == "--help" || help == "-h" => Ok(Action::Help),
        (Some(path), None, None) if !path.to_string_lossy().starts_with('-') => {
            Ok(Action::Inspect(path))
        }
        (Some(separator), Some(path), None) if separator == "--" => Ok(Action::Inspect(path)),
        _ => Err(()),
    }
}

#[derive(Debug)]
enum InspectorError {
    InputIo { kind: io::ErrorKind },
    NotRegularFile,
    Snapshot(SnapshotError),
    OutputIo { kind: io::ErrorKind },
}

impl fmt::Display for InspectorError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputIo {
                kind: io::ErrorKind::NotFound,
            } => output.write_str("input not found"),
            Self::InputIo { kind } => write!(output, "input access failed ({kind:?})"),
            Self::NotRegularFile => output.write_str("input is not a regular file"),
            Self::Snapshot(error) => error.fmt(output),
            Self::OutputIo { kind } => write!(output, "output write failed ({kind:?})"),
        }
    }
}

fn input_error(error: io::Error) -> InspectorError {
    InspectorError::InputIo { kind: error.kind() }
}
fn output_error(error: io::Error) -> InspectorError {
    InspectorError::OutputIo { kind: error.kind() }
}

fn inspect(path: &Path) -> Result<(), InspectorError> {
    let mut file = File::open(path).map_err(input_error)?;
    let metadata = file.metadata().map_err(input_error)?;
    if !metadata.is_file() {
        return Err(InspectorError::NotRegularFile);
    }
    let snapshot =
        read_geometry_snapshot(&mut file, metadata.len()).map_err(InspectorError::Snapshot)?;
    write_summary(&mut io::stdout().lock(), &snapshot).map_err(output_error)
}

fn write_summary(output: &mut impl Write, snapshot: &GeometrySnapshot) -> io::Result<()> {
    // Fixed bins keep output/allocation bounded even for many opaque categories.
    let mut categories = [[0_usize; 3]; 10];
    for surface in snapshot.surfaces() {
        let category = usize::try_from(surface.source_format_category())
            .unwrap_or(9)
            .min(9);
        categories[category][0] += 1;
        categories[category][1] += usize::from(surface.source_declared_vertex_count() == 0);
        categories[category][2] += surface.index_range().len();
    }
    writeln!(
        output,
        "inspection: diagnostic snapshot contract valid\n\
format: project-owned diagnostic geometry exchange v1\n\
total_length_bytes: {}\n\
position_records: {}\n\
index_references: {}\n\
triangles: {}\n\
surfaces: {}\n\
source_authenticity: unverified\n\
rendering: unverified\n\
original_game_fidelity: UNKNOWN",
        snapshot.total_bytes(),
        snapshot.positions().len(),
        snapshot.indices().len(),
        snapshot.indices().len() / 3,
        snapshot.surfaces().len()
    )?;
    for (category, counts) in categories.iter().enumerate() {
        let label = if category < 9 {
            category.to_string()
        } else {
            "other".to_owned()
        };
        writeln!(
            output,
            "opaque_source_category_{label}: surfaces={} zero_source_count_surfaces={} references={}",
            counts[0], counts[1], counts[2]
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bo2_assets::geometry_snapshot::{HEADER_BYTES, MAGIC, VERSION};

    #[test]
    fn output_errors_are_returned_with_only_the_kind() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "PRIVATE_PATH_AND_RAW_MESSAGE",
                ))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut bytes = [0; HEADER_BYTES];
        bytes[..8].copy_from_slice(&MAGIC);
        bytes[8..12].copy_from_slice(&VERSION.to_le_bytes());
        bytes[40..48].copy_from_slice(&(HEADER_BYTES as u64).to_le_bytes());
        let snapshot = read_geometry_snapshot(&mut bytes.as_slice(), HEADER_BYTES as u64).unwrap();
        let error = output_error(write_summary(&mut FailingWriter, &snapshot).unwrap_err());
        assert_eq!(error.to_string(), "output write failed (BrokenPipe)");
        assert!(!format!("{error:?}").contains("PRIVATE"));
    }

    #[test]
    fn input_error_retains_neither_the_message_nor_a_source() {
        let error = input_error(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "PRIVATE_PATH_AND_MESSAGE",
        ));
        assert_eq!(error.to_string(), "input access failed (PermissionDenied)");
        assert!(!format!("{error:?}").contains("PRIVATE"));
    }
}
