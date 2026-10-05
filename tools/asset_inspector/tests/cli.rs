#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn run(arguments: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bo2-asset-inspector"))
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

fn synthetic() -> Vec<u8> {
    let mut bytes = vec![0; 704];
    bytes[..8].copy_from_slice(b"TAff0100");
    bytes[8..12].copy_from_slice(&147_u32.to_le_bytes());
    bytes[12..20].copy_from_slice(b"PHEEBs71");
    bytes[24..31].copy_from_slice(b"fixture");
    bytes[56..312].fill(0xa5);
    bytes[312..].fill(0xfe);
    bytes
}

#[test]
fn help_and_usage_have_distinct_exit_codes_and_redact_arguments() {
    let output = run(&[OsStr::new("--help")]);
    assert_eq!(output.status.code(), Some(0));
    assert!(text(&output).contains("312 bytes"));
    for arguments in [
        vec![],
        vec!["--PRIVATE_ARGUMENT"],
        vec!["PRIVATE_ARGUMENT", "extra"],
    ] {
        let arguments: Vec<_> = arguments.iter().map(OsStr::new).collect();
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(!text(&output).contains("PRIVATE_ARGUMENT"));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn valid_input_emits_only_metadata_and_leaves_source_unchanged() {
    let bytes = synthetic();
    let fixture = Fixture::new("fixture.bin", &bytes);
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    let emitted = text(&output);
    for expected in [
        "envelope metadata only",
        "embedded_name: \"fixture\"",
        "total_length_bytes: 704",
        "signature_region_bytes: 56..312 (opaque)",
        "opaque_remainder_length_bytes: 392",
        "disk_stem: match",
        "authentication: unverified",
        "payload: unread",
    ] {
        assert!(emitted.contains(expected), "missing {expected}");
    }
    assert!(output.stderr.is_empty());
    assert!(!emitted.contains(&fixture.path.to_string_lossy().to_string()));
    assert!(!emitted.contains("a5a5"));
    assert!(!emitted.contains("fefe"));
    assert_eq!(fs::read(&fixture.path).unwrap(), bytes);

    // Equality of all emitted bytes excludes alternate signature/payload
    // encodings as well as the particular hexadecimal patterns checked above.
    let mut changed = bytes.clone();
    changed[56..312].fill(0x0c);
    changed[312..].fill(0x3e);
    let second = Fixture::new("fixture.bin", &changed);
    let second_output = run(&[second.path.as_os_str()]);
    assert_eq!(second_output.status.code(), Some(0));
    assert_eq!(second_output.stdout, output.stdout);
    assert_eq!(second_output.stderr, output.stderr);
    assert_eq!(fs::read(&second.path).unwrap(), changed);
}

#[test]
fn disk_stem_mismatch_is_diagnostic_and_does_not_reject_or_echo_the_stem() {
    let fixture = Fixture::new("PRIVATE_FILENAME.bin", &synthetic());
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(text(&output).contains("disk_stem: mismatch (diagnostic only)"));
    assert!(!text(&output).contains("PRIVATE_FILENAME"));
}

#[test]
fn missing_input_is_distinct_and_redacts_its_path() {
    let fixture = Fixture::new("PRIVATE_MISSING.bin", &[]);
    fs::remove_file(&fixture.path).unwrap();
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).contains("input not found"));
    assert!(!text(&output).contains("PRIVATE_MISSING"));
    assert!(output.stdout.is_empty());
}

#[test]
fn truncated_input_has_a_typed_redacted_diagnostic() {
    let fixture = Fixture::new("PRIVATE_TRUNCATED.bin", &synthetic()[..100]);
    let output = run(&[fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).contains("truncated envelope: need 312 bytes, have 100"));
    assert!(!text(&output).contains("PRIVATE_TRUNCATED"));
    assert!(output.stdout.is_empty());
}

#[test]
fn malformed_marker_auth_and_name_bytes_are_never_echoed() {
    for (start, end, expected) in [
        (0, 8, "unsupported fastfile marker"),
        (12, 20, "malformed supported-envelope authentication marker"),
        (24, 56, "unsupported embedded-name subset"),
    ] {
        let mut bytes = synthetic();
        for (index, byte) in bytes[start..end].iter_mut().enumerate() {
            *byte = b"PRIVATE!"[index % 8];
        }
        let fixture = Fixture::new("PRIVATE_SOURCE.bin", &bytes);
        let output = run(&[fixture.path.as_os_str()]);
        assert_eq!(output.status.code(), Some(1));
        assert!(text(&output).contains(expected));
        assert!(!text(&output).contains("PRIVATE"));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn directory_input_is_an_access_or_regular_file_failure_without_a_path() {
    let fixture = Fixture::new("fixture.bin", &synthetic());
    let output = run(&[fixture.directory.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text(&output).contains("input access failed")
            || text(&output).contains("not a regular file")
    );
    assert!(!text(&output).contains(&fixture.directory.to_string_lossy().to_string()));
}

#[test]
fn explicit_separator_accepts_one_input() {
    let fixture = Fixture::new("fixture.bin", &synthetic());
    let output = run(&[OsStr::new("--"), fixture.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
}

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
}

impl Fixture {
    fn new(name: &str, bytes: &[u8]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "bo2-envelope-cli-{}-{}",
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
