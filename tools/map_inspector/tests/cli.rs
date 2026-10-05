#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn run(arguments: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bo2-map-inspector"))
        .args(arguments)
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// Independently encoded, entirely arbitrary user-created triangle, never an
// original asset or derivative. Floating-point values are private to the fixture.
fn synthetic(variant: bool) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[..8].copy_from_slice(b"BO2DGS\0\0");
    bytes[8..12].copy_from_slice(&1_u32.to_le_bytes());
    bytes[16..24].copy_from_slice(&3_u64.to_le_bytes());
    bytes[24..32].copy_from_slice(&3_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&1_u64.to_le_bytes());
    bytes[40..48].copy_from_slice(&160_u64.to_le_bytes());
    let value = if variant { -9876.25_f32 } else { 1234.125_f32 };
    for position in [[value, 0.0, -0.0], [value, 1.0, 0.0], [value, 0.0, 1.0]] {
        for component in position {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    let indices = if variant {
        [2_u32, 0, 1]
    } else {
        [0_u32, 1, 2]
    };
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&3_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    for endpoint in [[value, 0.0, 0.0], [value, 1.0, 1.0]] {
        for component in endpoint {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    assert_eq!(bytes.len(), 160);
    bytes
}

#[test]
fn help_and_argument_failures_have_distinct_exit_codes_and_redacted_arguments() {
    for help in ["--help", "-h"] {
        let output = run(&[OsStr::new(help)]);
        assert_eq!(output.status.code(), Some(0));
        for expected in [
            "project-owned",
            "diagnostic geometry exchange",
            "not a BO2 disk format",
            "authenticity",
            "fidelity",
            "no output files",
        ] {
            // The final phrase appears with an initial "or" in the shared help.
            assert!(
                text(&output).contains(expected)
                    || (expected == "no output files" && text(&output).contains("or output files"))
            );
        }
        assert!(output.stderr.is_empty());
    }
    for arguments in [
        vec![],
        vec!["--PRIVATE_ARGUMENT"],
        vec!["PRIVATE_ARGUMENT", "extra"],
        vec!["--", "PRIVATE_ARGUMENT", "extra"],
        vec!["--"],
    ] {
        let arguments: Vec<_> = arguments.iter().map(OsStr::new).collect();
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(!text(&output).contains("PRIVATE"));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn successful_input_emits_bounded_counts_and_never_coordinates_references_bounds_or_paths() {
    let fixture = Fixture::new(OsStr::new("PRIVATE_SOURCE.bin"), &synthetic(false));
    let before = fs::metadata(&fixture.path).unwrap();
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let emitted = text(&output);
    for expected in [
        "diagnostic snapshot contract valid",
        "total_length_bytes: 160",
        "position_records: 3",
        "index_references: 3",
        "triangles: 1",
        "surfaces: 1",
        "opaque_source_category_7: surfaces=1 zero_source_count_surfaces=1 references=3",
        "source_authenticity: unverified",
        "rendering: unverified",
        "original_game_fidelity: UNKNOWN",
    ] {
        assert!(emitted.contains(expected), "missing {expected}");
    }
    assert!(!emitted.contains("PRIVATE"));
    assert!(!emitted.contains("1234.125"));
    assert!(output.stdout.len() < 2048);
    assert_eq!(fs::read(&fixture.path).unwrap(), synthetic(false));
    let after = fs::metadata(&fixture.path).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());

    // Different endpoint/coordinate bits and ordered indices must produce
    // exactly identical output; those arrays are not diagnostics.
    let other = Fixture::new(OsStr::new("PRIVATE_OTHER.bin"), &synthetic(true));
    let other_output = run(&[other.path.as_os_str()]);
    assert_eq!(other_output.status.code(), Some(0));
    assert_eq!(other_output.stdout, output.stdout);
    assert_eq!(other_output.stderr, output.stderr);
    assert_eq!(fs::read(&other.path).unwrap(), synthetic(true));
}

#[test]
fn missing_input_redacts_its_native_path() {
    let fixture = Fixture::new(OsStr::new("PRIVATE_MISSING.bin"), &[]);
    fs::remove_file(&fixture.path).unwrap();
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).contains("input not found"));
    assert!(!text(&output).contains("PRIVATE"));
    assert!(output.stdout.is_empty());
}

#[test]
fn directory_input_is_an_access_or_regular_file_failure_with_no_path() {
    let fixture = Fixture::new(OsStr::new("fixture.bin"), &synthetic(false));
    let output = run(&[fixture.directory.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text(&output).contains("input access failed")
            || text(&output).contains("not a regular file")
    );
    assert!(!text(&output).contains(&fixture.directory.to_string_lossy().to_string()));
    assert!(output.stdout.is_empty());
}

#[test]
fn explicit_separator_accepts_an_option_like_native_filename() {
    let fixture = Fixture::new(OsStr::new("--PRIVATE_SOURCE.bin"), &synthetic(false));
    let output = Command::new(env!("CARGO_BIN_EXE_bo2-map-inspector"))
        .current_dir(&fixture.directory)
        .args([OsStr::new("--"), OsStr::new("--PRIVATE_SOURCE.bin")])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(!text(&output).contains("PRIVATE"));
    assert_eq!(fs::read(&fixture.path).unwrap(), synthetic(false));
}

#[test]
fn non_unicode_paths_are_opened_natively_and_never_emitted() {
    #[cfg(windows)]
    let name = {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[
            b'P' as u16,
            b'R' as u16,
            b'I' as u16,
            b'V' as u16,
            0xd800,
            b'.' as u16,
            b'b' as u16,
            b'i' as u16,
            b'n' as u16,
        ])
    };
    #[cfg(unix)]
    let name = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(b"PRIV\xff.bin".to_vec())
    };
    #[cfg(not(any(windows, unix)))]
    let name = OsString::from("PRIV_unicode_\u{00e9}.bin");
    let fixture = Fixture::new(&name, &synthetic(false));
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(!text(&output).contains("PRIV"));
    assert_eq!(fs::read(&fixture.path).unwrap(), synthetic(false));
}

#[test]
fn actual_truncation_and_trailing_bytes_reject_with_no_summary_or_input_mutation() {
    for bytes in [
        synthetic(false)[..63].to_vec(),
        synthetic(false)[..159].to_vec(),
        {
            let mut bytes = synthetic(false);
            bytes.extend_from_slice(b"PRIVATE_TRAILER");
            bytes
        },
    ] {
        let fixture = Fixture::new(OsStr::new("PRIVATE_SOURCE.bin"), &bytes);
        let output = run(&[fixture.path.as_os_str()]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(
            text(&output).contains("truncated snapshot")
                || text(&output).contains("length mismatch")
        );
        assert!(!text(&output).contains("PRIVATE"));
        assert_eq!(fs::read(&fixture.path).unwrap(), bytes);
    }
}

#[test]
fn malformed_fields_have_redacted_typed_diagnostics_and_no_partial_summary() {
    for (offset, replacement, expected) in [
        (
            0,
            b"PRIVATE!".as_slice(),
            "unsupported diagnostic snapshot magic",
        ),
        (
            8,
            &[0xfe, 0xff, 0xff, 0xff],
            "unsupported diagnostic snapshot version",
        ),
        (12, &[1], "unsupported diagnostic snapshot flags"),
        (63, &[1], "unsupported diagnostic snapshot reserved bytes"),
        (64, &[0x34, 0x12, 0xc0, 0x7f], "nonfinite position"),
        (100, &[0xff; 4], "index is out of range"),
        (136, &[0x34, 0x12, 0xc0, 0x7f], "surface bounds are invalid"),
    ] {
        let mut bytes = synthetic(false);
        bytes[offset..offset + replacement.len()].copy_from_slice(replacement);
        let fixture = Fixture::new(OsStr::new("PRIVATE_SOURCE.bin"), &bytes);
        let output = run(&[fixture.path.as_os_str()]);
        assert_eq!(output.status.code(), Some(1));
        assert!(text(&output).contains(expected), "{}", text(&output));
        assert!(!text(&output).contains("PRIVATE"));
        assert!(!text(&output).contains("4294967294"));
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read(&fixture.path).unwrap(), bytes);
    }
}

#[test]
fn many_opaque_categories_have_constant_output_shape_and_no_category_value_dump() {
    let mut bytes = vec![0; 64];
    bytes[..8].copy_from_slice(b"BO2DGS\0\0");
    bytes[8..12].copy_from_slice(&1_u32.to_le_bytes());
    bytes[32..40].copy_from_slice(&1_024_u64.to_le_bytes());
    bytes[40..48].copy_from_slice(&(64_u64 + 48 * 1_024).to_le_bytes());
    for category in 10_000_u32..11_024 {
        bytes.extend_from_slice(&[0; 16]);
        bytes.extend_from_slice(&category.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 24]);
    }
    let fixture = Fixture::new(OsStr::new("PRIVATE_CATEGORIES.bin"), &bytes);
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout.clone())
            .unwrap()
            .lines()
            .count(),
        20
    );
    assert!(text(&output).contains(
        "opaque_source_category_other: surfaces=1024 zero_source_count_surfaces=1024 references=0"
    ));
    assert!(!text(&output).contains("10000"));
    assert!(output.stdout.len() < 2048);
    assert_eq!(fs::read(&fixture.path).unwrap(), bytes);
}

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
}

impl Fixture {
    fn new(name: &OsStr, bytes: &[u8]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "bo2-geometry-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join(name);
        fs::write(&path, bytes).unwrap();
        Self { directory, path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.directory);
    }
}
