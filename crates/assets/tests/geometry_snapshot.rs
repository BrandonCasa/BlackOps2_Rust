#![forbid(unsafe_code)]

use std::io::{self, Read};
use std::mem::size_of_val;

use bo2_assets::geometry_snapshot::{
    GeometrySnapshot, HEADER_BYTES, MAGIC, MAX_INDICES, MAX_INTERRUPTED_READS, MAX_POSITIONS,
    MAX_REFERENCE_VISITS, MAX_RETAINED_BYTES, MAX_SURFACES, MAX_TOTAL_BYTES, SnapshotError,
    SnapshotLimit, SnapshotSection, SnapshotSurface, VERSION, read_geometry_snapshot,
};

// All positions, references, endpoints and metadata are arbitrary user-created
// values. This builder is a test-only encoder of our exchange contract, never a
// producer for original files or a fixture derived from licensed game geometry.
struct Surface {
    first: u64,
    count: u64,
    category: u32,
    source_count: u32,
    bounds: [[f32; 3]; 2],
}

fn encode(positions: &[[f32; 3]], indices: &[u32], surfaces: &[Surface]) -> Vec<u8> {
    let length = 64 + 12 * positions.len() + 4 * indices.len() + 48 * surfaces.len();
    let mut bytes = header(
        positions.len() as u64,
        indices.len() as u64,
        surfaces.len() as u64,
        length as u64,
    )
    .to_vec();
    for position in positions {
        for component in position {
            bytes.extend_from_slice(&component.to_bits().to_le_bytes());
        }
    }
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    for surface in surfaces {
        bytes.extend_from_slice(&surface.first.to_le_bytes());
        bytes.extend_from_slice(&surface.count.to_le_bytes());
        bytes.extend_from_slice(&surface.category.to_le_bytes());
        bytes.extend_from_slice(&surface.source_count.to_le_bytes());
        for endpoint in surface.bounds {
            for component in endpoint {
                bytes.extend_from_slice(&component.to_bits().to_le_bytes());
            }
        }
    }
    bytes
}

fn header(positions: u64, indices: u64, surfaces: u64, total: u64) -> [u8; HEADER_BYTES] {
    let mut bytes = [0; HEADER_BYTES];
    bytes[..8].copy_from_slice(&MAGIC);
    bytes[8..12].copy_from_slice(&VERSION.to_le_bytes());
    bytes[16..24].copy_from_slice(&positions.to_le_bytes());
    bytes[24..32].copy_from_slice(&indices.to_le_bytes());
    bytes[32..40].copy_from_slice(&surfaces.to_le_bytes());
    bytes[40..48].copy_from_slice(&total.to_le_bytes());
    bytes
}

fn fixture() -> Vec<u8> {
    encode(
        &[
            [0.0, -0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [10.0, 0.0, 0.0],
        ],
        &[2, 0, 1, 1, 3, 0],
        &[
            Surface {
                first: 0,
                count: 3,
                category: 7,
                source_count: 0,
                bounds: [[0.0; 3], [1.0, 1.0, 0.0]],
            },
            Surface {
                first: 3,
                count: 3,
                category: 0,
                source_count: 99,
                bounds: [[0.0; 3], [10.0, 0.0, 0.0]],
            },
        ],
    )
}

fn parse(bytes: &[u8]) -> Result<GeometrySnapshot, SnapshotError> {
    read_geometry_snapshot(&mut &*bytes, bytes.len() as u64)
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

const INDICES_OFFSET: usize = 64 + 4 * 12;
const SURFACES_OFFSET: usize = INDICES_OFFSET + 6 * 4;

#[test]
fn shared_positions_are_checked_only_against_each_surfaces_own_references() {
    let snapshot = parse(&fixture()).unwrap();
    assert_eq!(snapshot.positions().len(), 4);
    assert_eq!(snapshot.indices(), [2, 0, 1, 1, 3, 0]);
    let surfaces = snapshot.surfaces();
    assert_eq!(surfaces.len(), 2);
    assert_eq!(surfaces[0].index_range(), 0..3);
    assert_eq!(surfaces[1].index_range(), 3..6);
    assert_eq!(surfaces[0].triangle_count(), 1);
    assert_eq!(surfaces[0].source_format_category(), 7);
    assert_eq!(surfaces[0].source_declared_vertex_count(), 0);
    assert_eq!(surfaces[1].source_declared_vertex_count(), 99);
    assert_eq!(surfaces[0].bounds(), &[[0.0; 3], [1.0, 1.0, 0.0]]);
    // Position 3 is intentionally outside surface 0; it belongs only to surface 1.
    assert!(snapshot.positions()[3][0] > surfaces[0].bounds()[1][0]);
    let expected_capacity = size_of_val(snapshot.positions())
        + size_of_val(snapshot.indices())
        + size_of_val(snapshot.surfaces());
    assert!(snapshot.retained_capacity_bytes() >= expected_capacity as u64);
    assert!(snapshot.retained_capacity_bytes() <= MAX_RETAINED_BYTES);
    assert_eq!(snapshot.total_bytes(), fixture().len() as u64);
}

#[test]
fn every_finite_float_bit_and_ordered_reference_survives_without_repairs() {
    let positions = [
        [
            f32::from_bits(0x8000_0000),
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
        ],
        [f32::MAX, -f32::MAX, 3.125],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0], // Equal positions and an unreferenced record must survive.
    ];
    let bounds = [[-f32::MAX; 3], [f32::MAX; 3]];
    let snapshot = parse(&encode(
        &positions,
        &[1, 0, 0, 2, 2, 2],
        &[Surface {
            first: 0,
            count: 6,
            category: u32::MAX,
            source_count: 0,
            bounds,
        }],
    ))
    .unwrap();
    for (actual, expected) in snapshot.positions().iter().zip(positions) {
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
    }
    assert_eq!(snapshot.indices(), [1, 0, 0, 2, 2, 2]);
    assert_eq!(snapshot.surfaces()[0].source_format_category(), u32::MAX);
    let (owned_positions, owned_indices, owned_surfaces) = snapshot.into_parts();
    assert_eq!(owned_positions.len(), 4);
    assert_eq!(owned_indices, [1, 0, 0, 2, 2, 2]);
    assert_eq!(owned_surfaces[0].index_range(), 0..6);
}

#[test]
fn empty_snapshots_unreferenced_positions_and_empty_surfaces_are_valid() {
    let empty = parse(&encode(&[], &[], &[])).unwrap();
    assert_eq!(empty.total_bytes(), 64);
    assert_eq!(empty.retained_capacity_bytes(), 0);
    let positions = [[5.0; 3]];
    let empty_surface = Surface {
        first: 0,
        count: 0,
        category: 8,
        source_count: u32::MAX,
        bounds: [[0.0; 3], [0.0; 3]],
    };
    let snapshot = parse(&encode(&positions, &[], &[empty_surface])).unwrap();
    assert_eq!(snapshot.positions(), positions);
    assert_eq!(snapshot.surfaces()[0].index_range(), 0..0);
    for category in [6, 8, 9, u32::MAX] {
        let mut bytes = fixture();
        put_u32(&mut bytes, SURFACES_OFFSET + 16, category);
        assert_eq!(
            parse(&bytes).unwrap().surfaces()[0].source_format_category(),
            category
        );
    }
}

#[test]
fn zero_length_surfaces_can_appear_between_triangles_without_filtering() {
    let bounds = [[0.0; 3], [0.0; 3]];
    let snapshot = parse(&encode(
        &[[0.0; 3]],
        &[0; 6],
        &[
            Surface {
                first: 0,
                count: 0,
                category: 1,
                source_count: 0,
                bounds,
            },
            Surface {
                first: 0,
                count: 3,
                category: 2,
                source_count: 0,
                bounds,
            },
            Surface {
                first: 3,
                count: 0,
                category: 3,
                source_count: 0,
                bounds,
            },
            Surface {
                first: 3,
                count: 3,
                category: 4,
                source_count: 0,
                bounds,
            },
            Surface {
                first: 6,
                count: 0,
                category: 5,
                source_count: 0,
                bounds,
            },
        ],
    ))
    .unwrap();
    assert_eq!(
        snapshot
            .surfaces()
            .iter()
            .map(SnapshotSurface::index_range)
            .collect::<Vec<_>>(),
        [0..0, 0..3, 3..3, 3..6, 6..6]
    );
}

#[test]
fn every_actual_file_truncation_rejects_before_body_allocation_or_reads() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        let mut reader = RecordingReader::new(&bytes[..length], usize::MAX);
        let error = read_geometry_snapshot(&mut reader, length as u64).unwrap_err();
        if length < 64 {
            assert_eq!(error, SnapshotError::Truncated(SnapshotSection::Header));
            assert_eq!(reader.position, 0);
        } else {
            assert_eq!(error, SnapshotError::LengthMismatch);
            assert_eq!(reader.position, 64);
        }
        assert!(
            reader
                .requests
                .iter()
                .all(|&(start, length)| start + length <= 64)
        );
    }
}

#[test]
fn unexpected_eof_at_every_record_byte_reports_its_section() {
    let bytes = fixture();
    for end in 0..bytes.len() {
        let mut reader = RecordingReader::new(&bytes[..end], 7);
        let section = if end < 64 {
            SnapshotSection::Header
        } else if end < INDICES_OFFSET {
            SnapshotSection::Positions
        } else if end < SURFACES_OFFSET {
            SnapshotSection::Indices
        } else {
            SnapshotSection::Surfaces
        };
        assert_eq!(
            read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap_err(),
            SnapshotError::Truncated(section),
            "unexpected EOF at {end}"
        );
        assert_eq!(reader.position, end);
    }
}

#[test]
fn unsupported_magic_version_flags_and_reserved_bytes_never_read_the_body() {
    let baseline = fixture();
    for (offset, value, expected) in [
        (0, 0xff, SnapshotError::UnsupportedMagic),
        (8, 2, SnapshotError::UnsupportedVersion),
        (11, 1, SnapshotError::UnsupportedVersion),
        (12, 1, SnapshotError::UnsupportedFlags),
        (15, 1, SnapshotError::UnsupportedFlags),
        (48, 1, SnapshotError::UnsupportedReserved),
        (63, 1, SnapshotError::UnsupportedReserved),
    ] {
        let mut bytes = baseline.clone();
        bytes[offset] = value;
        let mut reader = RecordingReader::new(&bytes, usize::MAX);
        assert_eq!(
            read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap_err(),
            expected
        );
        assert_eq!(reader.requests, [(0, 64)]);
    }
}

#[test]
fn multiplication_and_total_addition_overflow_reject_from_the_header() {
    // Each count separately overflows its section bytes; the last case has
    // representable sections whose sum overflows. The reader owns no body.
    for (p, i, s) in [
        (u64::MAX, 0, 0),
        (0, u64::MAX, 0),
        (0, 0, u64::MAX),
        (u64::MAX / 12, 1, 0),
    ] {
        let bytes = header(p, i, s, 64);
        let mut reader = RecordingReader::new(&bytes, usize::MAX);
        assert_eq!(
            read_geometry_snapshot(&mut reader, 64).unwrap_err(),
            SnapshotError::ArithmeticOverflow
        );
        assert_eq!(reader.requests, [(0, 64)]);
    }
}

#[test]
fn oversized_counts_total_bytes_and_retained_bytes_reject_before_body_reads() {
    assert_eq!(MAX_INDICES, MAX_REFERENCE_VISITS);
    for (p, i, s, expected) in [
        (MAX_POSITIONS + 1, 0, 0, SnapshotLimit::Positions),
        (0, MAX_INDICES + 1, 0, SnapshotLimit::Indices),
        (0, 0, MAX_SURFACES + 1, SnapshotLimit::Surfaces),
        (2_000_000, 10_000_000, 100_000, SnapshotLimit::TotalBytes),
        (2_000_000, 9_000_000, 100_000, SnapshotLimit::RetainedBytes),
    ] {
        let bytes = header(p, i, s, 64);
        let mut reader = RecordingReader::new(&bytes, usize::MAX);
        assert_eq!(
            read_geometry_snapshot(&mut reader, 64).unwrap_err(),
            SnapshotError::LimitExceeded(expected)
        );
        assert_eq!(reader.position, 64);
    }
    let bytes = header(0, 0, 0, 64);
    let mut reader = RecordingReader::new(&bytes, usize::MAX);
    assert_eq!(
        read_geometry_snapshot(&mut reader, MAX_TOTAL_BYTES + 1).unwrap_err(),
        SnapshotError::LimitExceeded(SnapshotLimit::TotalBytes)
    );
    assert!(reader.requests.is_empty());
}

#[test]
fn both_header_and_caller_lengths_must_equal_the_computed_length() {
    let baseline = fixture();
    for total in [
        0,
        63,
        baseline.len() as u64 - 1,
        baseline.len() as u64 + 1,
        u64::MAX,
    ] {
        let mut bytes = baseline.clone();
        put_u64(&mut bytes, 40, total);
        let mut reader = RecordingReader::new(&bytes, usize::MAX);
        assert_eq!(
            read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap_err(),
            SnapshotError::LengthMismatch
        );
        assert_eq!(reader.position, 64);
    }
    for length in [baseline.len() as u64 - 1, baseline.len() as u64 + 1] {
        let mut reader = RecordingReader::new(&baseline, usize::MAX);
        assert_eq!(
            read_geometry_snapshot(&mut reader, length).unwrap_err(),
            SnapshotError::LengthMismatch
        );
        assert_eq!(reader.position, 64);
    }
}

#[test]
fn trailing_bytes_are_not_ignored_and_probe_consumes_at_most_one() {
    let mut bytes = fixture();
    let length = bytes.len();
    bytes.extend_from_slice(b"PRIVATE_TRAILER");
    let mut reader = RecordingReader::new(&bytes, usize::MAX);
    assert_eq!(
        read_geometry_snapshot(&mut reader, length as u64).unwrap_err(),
        SnapshotError::TrailingData
    );
    assert_eq!(reader.position, length + 1);
    assert_eq!(reader.requests.last(), Some(&(length, 1)));
    assert_eq!(parse(&bytes).unwrap_err(), SnapshotError::LengthMismatch);
}

#[test]
fn successful_reads_and_short_reads_never_request_outside_admitted_ranges() {
    let bytes = fixture();
    for step in [1, 7, usize::MAX] {
        let mut reader = RecordingReader::new(&bytes, step);
        reader.interrupts = 3;
        let snapshot = read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap();
        assert_eq!(snapshot.indices(), [2, 0, 1, 1, 3, 0]);
        assert_eq!(reader.position, bytes.len());
        assert_eq!(reader.requests.last(), Some(&(bytes.len(), 1)));
        assert!(
            reader
                .requests
                .iter()
                .all(|&(start, count)| start + count <= bytes.len()
                    || (start == bytes.len() && count == 1))
        );
    }
}

#[test]
fn chunks_handle_more_than_one_scratch_buffer_without_reordering() {
    let positions = vec![[0.0, -0.0, 1.0]; 800];
    let indices: Vec<_> = (0..2_100).map(|index| (index % 800) as u32).collect();
    let surfaces: Vec<_> = (0..700)
        .map(|surface| Surface {
            first: surface * 3,
            count: 3,
            category: (surface % 10) as u32,
            source_count: 0,
            bounds: [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
        })
        .collect();
    let bytes = encode(&positions, &indices, &surfaces);
    let mut reader = RecordingReader::new(&bytes, 59);
    let snapshot = read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap();
    assert_eq!(snapshot.indices(), indices);
    assert_eq!(snapshot.positions().len(), 800);
    assert_eq!(snapshot.surfaces().len(), 700);
    assert_eq!(snapshot.surfaces()[699].index_range(), 2097..2100);
    assert!(reader.requests.iter().all(|&(_, count)| count <= 4096));
}

#[test]
fn nan_and_either_infinity_in_any_position_component_are_rejected() {
    for component in 0..12 {
        for bits in [0x7fc0_1234, 0xffc0_1234, 0x7f80_0000, 0xff80_0000] {
            let mut bytes = fixture();
            put_u32(&mut bytes, 64 + component * 4, bits);
            assert_eq!(parse(&bytes).unwrap_err(), SnapshotError::NonFinitePosition);
        }
    }
}

#[test]
fn indices_equal_to_capacity_and_maximum_indices_are_invalid() {
    for offset in 0..6 {
        for index in [4, u32::MAX] {
            let mut bytes = fixture();
            put_u32(&mut bytes, INDICES_OFFSET + offset * 4, index);
            assert_eq!(parse(&bytes).unwrap_err(), SnapshotError::InvalidIndex);
        }
    }
    assert_eq!(
        parse(&encode(&[], &[0; 3], &[])).unwrap_err(),
        SnapshotError::InvalidIndex
    );
}

#[test]
fn total_indices_must_be_triangular_before_the_body_is_read() {
    for indices in [1, 2, 4, 5] {
        let length = 64 + 4 * indices;
        let bytes = header(0, indices, 0, length);
        let mut reader = RecordingReader::new(&bytes, usize::MAX);
        assert_eq!(
            read_geometry_snapshot(&mut reader, length).unwrap_err(),
            SnapshotError::NonTriangularIndices
        );
        assert_eq!(reader.position, 64);
    }
}

#[test]
fn overflowing_misaligned_and_out_of_range_surfaces_are_invalid() {
    for (first, count) in [(u64::MAX, 3), (3, u64::MAX), (1, 3), (0, 2), (0, 9), (9, 0)] {
        let mut bytes = fixture();
        put_u64(&mut bytes, SURFACES_OFFSET, first);
        put_u64(&mut bytes, SURFACES_OFFSET + 8, count);
        assert_eq!(
            parse(&bytes).unwrap_err(),
            SnapshotError::InvalidSurfaceRange
        );
    }
}

#[test]
fn gapped_overlapping_or_incomplete_surface_partitions_are_rejected() {
    for (surface, first, count) in [(0, 3, 3), (1, 0, 3), (1, 6, 0)] {
        let mut bytes = fixture();
        put_u64(&mut bytes, SURFACES_OFFSET + surface * 48, first);
        put_u64(&mut bytes, SURFACES_OFFSET + surface * 48 + 8, count);
        assert_eq!(
            parse(&bytes).unwrap_err(),
            SnapshotError::NonPartitioningSurfaces
        );
    }
    let mut bytes = fixture();
    put_u64(&mut bytes, SURFACES_OFFSET + 48 + 8, 0);
    assert_eq!(
        parse(&bytes).unwrap_err(),
        SnapshotError::NonPartitioningSurfaces
    );
    assert_eq!(
        parse(&encode(&[[0.0; 3]], &[0; 3], &[])).unwrap_err(),
        SnapshotError::NonPartitioningSurfaces
    );
}

#[test]
fn nonfinite_or_reversed_endpoint_components_are_invalid_even_on_empty_surfaces() {
    for component in 0..6 {
        for bits in [0x7fc0_1234, 0x7f80_0000, 0xff80_0000] {
            let mut bytes = fixture();
            put_u32(&mut bytes, SURFACES_OFFSET + 24 + component * 4, bits);
            assert_eq!(
                parse(&bytes).unwrap_err(),
                SnapshotError::InvalidSurfaceBounds
            );
        }
    }
    for axis in 0..3 {
        let mut bytes = fixture();
        put_u32(
            &mut bytes,
            SURFACES_OFFSET + 24 + axis * 4,
            20.0_f32.to_bits(),
        );
        assert_eq!(
            parse(&bytes).unwrap_err(),
            SnapshotError::InvalidSurfaceBounds
        );
    }
    assert_eq!(
        parse(&encode(
            &[],
            &[],
            &[Surface {
                first: 0,
                count: 0,
                category: 0,
                source_count: 0,
                bounds: [[1.0; 3], [0.0; 3]]
            }]
        ))
        .unwrap_err(),
        SnapshotError::InvalidSurfaceBounds
    );
}

#[test]
fn exact_own_bounds_reject_one_float_step_outside_without_tolerance() {
    for (surface, endpoint, axis, bits) in [
        (0, 1, 0, 1.0_f32.to_bits() - 1),
        (0, 0, 1, 1_u32),
        (1, 1, 0, 10.0_f32.to_bits() - 1),
    ] {
        let mut bytes = fixture();
        put_u32(
            &mut bytes,
            SURFACES_OFFSET + surface * 48 + 24 + endpoint * 12 + axis * 4,
            bits,
        );
        assert_eq!(
            parse(&bytes).unwrap_err(),
            SnapshotError::PositionOutsideSurfaceBounds
        );
    }
}

#[test]
fn interrupted_read_budget_is_cumulative_and_finite() {
    let bytes = fixture();
    let mut reader = RecordingReader::new(&bytes, 1);
    reader.interrupts = MAX_INTERRUPTED_READS;
    read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap();
    let mut reader = RecordingReader::new(&bytes, 1);
    reader.interrupts = MAX_INTERRUPTED_READS + 1;
    assert_eq!(
        read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap_err(),
        SnapshotError::LimitExceeded(SnapshotLimit::InterruptedReads)
    );
    assert_eq!(reader.position, 0);
    assert_eq!(reader.requests.len(), MAX_INTERRUPTED_READS as usize + 1);
}

#[test]
fn injected_io_and_invalid_read_counts_are_typed_and_raw_messages_are_redacted() {
    let bytes = fixture();
    for failure_at in [0, 63, 64, INDICES_OFFSET, SURFACES_OFFSET, bytes.len()] {
        let mut reader = RecordingReader::new(&bytes, 1);
        reader.failure_at = Some(failure_at);
        let error = read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap_err();
        assert_eq!(
            error,
            SnapshotError::Io {
                kind: io::ErrorKind::PermissionDenied
            }
        );
        assert!(!format!("{error:?} {error}").contains("PRIVATE"));
        assert!(std::error::Error::source(&error).is_none());
    }
    struct InvalidReader;
    impl Read for InvalidReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Ok(usize::MAX)
        }
    }
    assert_eq!(
        read_geometry_snapshot(&mut InvalidReader, 64).unwrap_err(),
        SnapshotError::Io {
            kind: io::ErrorKind::InvalidData
        }
    );
}

#[test]
fn endpoint_signed_zero_and_subnormal_bits_survive_exact_containment() {
    let position = [-0.0, f32::from_bits(1), f32::from_bits(0x8000_0001)];
    let bounds = [
        position,
        [0.0, f32::from_bits(1), f32::from_bits(0x8000_0001)],
    ];
    let snapshot = parse(&encode(
        &[position],
        &[0; 3],
        &[Surface {
            first: 0,
            count: 3,
            category: 6,
            source_count: 0,
            bounds,
        }],
    ))
    .unwrap();
    assert_eq!(
        snapshot.positions()[0].map(f32::to_bits),
        position.map(f32::to_bits)
    );
    for (actual, expected) in snapshot.surfaces()[0].bounds().iter().zip(bounds) {
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
    }
}

#[test]
fn interruption_admission_spans_the_header_body_and_eof_in_one_budget() {
    struct StagedReader<'a> {
        reader: RecordingReader<'a>,
        before: u32,
        after: u32,
    }
    impl Read for StagedReader<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            let remaining = if self.reader.position < 64 {
                &mut self.before
            } else {
                &mut self.after
            };
            if *remaining > 0 {
                *remaining -= 1;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            self.reader.read(output)
        }
    }
    let bytes = fixture();
    for after in [1, 2] {
        let mut reader = StagedReader {
            reader: RecordingReader::new(&bytes, usize::MAX),
            before: MAX_INTERRUPTED_READS - 1,
            after,
        };
        let result = read_geometry_snapshot(&mut reader, bytes.len() as u64);
        if after == 1 {
            assert!(result.is_ok());
        } else {
            assert_eq!(
                result.unwrap_err(),
                SnapshotError::LimitExceeded(SnapshotLimit::InterruptedReads)
            );
            assert_eq!(reader.reader.position, 64);
        }
    }
}

#[test]
fn invalid_reader_counts_in_body_or_eof_do_not_panic() {
    struct InvalidAt<'a> {
        reader: RecordingReader<'a>,
        at: usize,
    }
    impl Read for InvalidAt<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.reader.position == self.at {
                return Ok(output.len() + 1);
            }
            self.reader.read(output)
        }
    }
    let bytes = fixture();
    for at in [64, INDICES_OFFSET, SURFACES_OFFSET, bytes.len()] {
        let mut reader = InvalidAt {
            reader: RecordingReader::new(&bytes, usize::MAX),
            at,
        };
        assert_eq!(
            read_geometry_snapshot(&mut reader, bytes.len() as u64).unwrap_err(),
            SnapshotError::Io {
                kind: io::ErrorKind::InvalidData
            }
        );
    }
}

struct RecordingReader<'a> {
    bytes: &'a [u8],
    position: usize,
    step: usize,
    requests: Vec<(usize, usize)>,
    interrupts: u32,
    failure_at: Option<usize>,
}

impl<'a> RecordingReader<'a> {
    fn new(bytes: &'a [u8], step: usize) -> Self {
        Self {
            bytes,
            position: 0,
            step,
            requests: Vec::new(),
            interrupts: 0,
            failure_at: None,
        }
    }
}

impl Read for RecordingReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.requests.push((self.position, output.len()));
        if self.interrupts > 0 {
            self.interrupts -= 1;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        if self.failure_at == Some(self.position) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PRIVATE_PATH_AND_RAW_MESSAGE",
            ));
        }
        let count = output
            .len()
            .min(self.step)
            .min(self.bytes.len() - self.position);
        output[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
