#![forbid(unsafe_code)]
use bo2_assets::{
    brush_collection::Identity,
    terrain_snapshot::{
        HEADER_BYTES, MAGIC, MAX_BYTES, TerrainError, TerrainIdentity, read_terrain_snapshot,
    },
};
use std::io::{self, Read};
fn word(bytes: &mut Vec<u8>, v: u32) {
    bytes.extend_from_slice(&v.to_le_bytes());
}
fn half(bytes: &mut Vec<u8>, v: u16) {
    bytes.extend_from_slice(&v.to_le_bytes());
}
fn triple(bytes: &mut Vec<u8>, values: [f32; 3]) {
    for v in values {
        word(bytes, v.to_bits());
    }
}
fn leaf(bytes: &mut Vec<u8>) {
    half(bytes, 0);
    half(bytes, 1);
    word(bytes, 0xffff_ffff);
    word(bytes, 0x8000_0000);
    triple(bytes, [-1.; 3]);
    triple(bytes, [1.; 3]);
    word(bytes, 0);
    half(bytes, 0xffff);
    half(bytes, 0);
}
fn fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC);
    word(&mut bytes, 1);
    word(&mut bytes, 0);
    bytes.extend_from_slice(&402_u64.to_le_bytes());
    bytes.extend_from_slice(&10_u64.to_le_bytes());
    bytes.extend_from_slice(&[1; 32]);
    bytes.extend_from_slice(&20_u64.to_le_bytes());
    bytes.extend_from_slice(&[2; 32]);
    for count in [1, 2, 1, 1, 3, 4, 1, 1] {
        word(&mut bytes, count);
    }
    bytes.extend_from_slice(&[0; 8]);
    assert_eq!(bytes.len(), HEADER_BYTES);
    word(&mut bytes, u32::MAX);
    word(&mut bytes, 0x8000_0000);
    for (children, reference) in [(1, 1), (0, 0)] {
        triple(&mut bytes, [-0., 2., 3.]);
        half(&mut bytes, 0);
        half(&mut bytes, children);
        triple(&mut bytes, [4., 5., 6.]);
        word(&mut bytes, reference);
    }
    bytes.extend_from_slice(&[1, 0, 0, 0]);
    word(&mut bytes, 0);
    word(&mut bytes, 0xffff_ffff);
    word(&mut bytes, 1234);
    for vertex in [2, 0, 1] {
        half(&mut bytes, vertex);
    }
    for vertex in [[-0., 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        triple(&mut bytes, vertex);
    }
    bytes.extend_from_slice(&[0b101, 0xaa, 0xbb, 0xcc]);
    leaf(&mut bytes);
    word(&mut bytes, 0);
    word(&mut bytes, 0);
    triple(&mut bytes, [-1.; 3]);
    triple(&mut bytes, [1.; 3]);
    word(&mut bytes, 1_f32.to_bits());
    leaf(&mut bytes);
    assert_eq!(bytes.len(), 402);
    bytes
}
fn read(bytes: &[u8]) -> Result<bo2_assets::terrain_snapshot::TerrainSnapshot, TerrainError> {
    read_terrain_snapshot(&mut &bytes[..], bytes.len() as u64)
}
#[test]
fn preserves_source_words_order_metadata_and_shared_leaf_geometry() {
    let snapshot = read(&fixture()).unwrap();
    assert_eq!(snapshot.materials()[0].surface_flags, u32::MAX);
    assert_eq!(snapshot.materials()[0].contents, 0x8000_0000);
    assert_eq!(
        snapshot.aabbs()[0].origin_bits,
        [-0_f32, 2., 3.].map(f32::to_bits)
    );
    assert_eq!(snapshot.aabbs()[0].reference, 1);
    assert_eq!(snapshot.partitions()[0].nuinds, -1);
    assert_eq!(snapshot.partitions()[0].fuind, 1234);
    assert_eq!(snapshot.triangles(), &[[2, 0, 1]]);
    assert_eq!(snapshot.vertices_bits()[0][0], (-0_f32).to_bits());
    assert_eq!(snapshot.packed_edges(), &[0b101, 0xaa, 0xbb, 0xcc]);
    assert_eq!(snapshot.map_leaves()[0], snapshot.models()[0].leaf);
    assert_eq!(snapshot.models()[0].ordinal, 0);
    assert!(!snapshot.models()[0].serialized_info_present);
    assert_eq!(snapshot.map_leaves()[0].cluster, -1);
    let expected = TerrainIdentity {
        source: Identity {
            bytes: 10,
            sha256: [1; 32],
        },
        membership: Identity {
            bytes: 20,
            sha256: [2; 32],
        },
    };
    snapshot.check_identity(expected).unwrap();
    let mut changed = expected;
    changed.membership.sha256[0] = 3;
    assert_eq!(
        snapshot.check_identity(changed),
        Err(TerrainError::IdentityMismatch)
    );
}
#[test]
fn rejects_every_truncated_prefix_and_mismatched_supplied_length() {
    let bytes = fixture();
    for end in 0..bytes.len() {
        assert_eq!(
            read_terrain_snapshot(&mut &bytes[..end], bytes.len() as u64).unwrap_err(),
            TerrainError::Truncated
        );
    }
    assert_eq!(
        read_terrain_snapshot(&mut bytes.as_slice(), 403).unwrap_err(),
        TerrainError::LengthMismatch
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(read(&trailing).unwrap_err(), TerrainError::LengthMismatch);
}
#[test]
fn rejects_format_reserves_nonfinite_bounds_presence_and_all_references() {
    let bytes = fixture();
    let change_word = |at: usize, value: u32| {
        let mut b = bytes.clone();
        b[at..at + 4].copy_from_slice(&value.to_le_bytes());
        b
    };
    let change_half = |at: usize, value: u16| {
        let mut b = bytes.clone();
        b[at..at + 2].copy_from_slice(&value.to_le_bytes());
        b
    };
    for (b, error) in [
        (change_word(8, 2), TerrainError::UnsupportedVersion),
        (change_word(12, 1), TerrainError::NonzeroReserved),
        (change_word(136, 1), TerrainError::NonzeroReserved),
        (change_word(124, 0), TerrainError::PackedEdgeLength),
        (change_word(108, 1_000_001), TerrainError::LimitExceeded),
        (
            change_word(152, f32::NAN.to_bits()),
            TerrainError::NonFinite,
        ),
        (change_half(164, 1), TerrainError::InvalidReference),
        (
            change_word(168, (-1_f32).to_bits()),
            TerrainError::InvalidBounds,
        ),
        (change_word(180, 0), TerrainError::Cycle),
        (change_word(212, 1), TerrainError::InvalidReference),
        (change_word(220, u32::MAX), TerrainError::InvalidReference),
        (change_half(232, 3), TerrainError::InvalidReference),
        (
            change_word(238, f32::INFINITY.to_bits()),
            TerrainError::NonFinite,
        ),
        (change_half(278, 2), TerrainError::InvalidReference),
        (
            change_word(290, 2_f32.to_bits()),
            TerrainError::InvalidBounds,
        ),
        (change_word(322, 1), TerrainError::InvalidModelOrdinal),
        (change_word(326, 2), TerrainError::InvalidPresence),
        (
            change_word(354, (-1_f32).to_bits()),
            TerrainError::InvalidBounds,
        ),
    ] {
        assert_eq!(read(&b).unwrap_err(), error);
    }
    let mut wrong = bytes.clone();
    wrong[0] = 0;
    assert_eq!(read(&wrong).unwrap_err(), TerrainError::UnsupportedMagic);
    let mut wrong = bytes;
    wrong[217] = 1;
    assert_eq!(read(&wrong).unwrap_err(), TerrainError::NonzeroReserved);
}
#[test]
fn checks_cycles_even_in_unreferenced_records_and_accepts_shared_dags() {
    let mut bytes = fixture();
    // Root terminal, otherwise unused second node points to itself.
    bytes[166..168].copy_from_slice(&0_u16.to_le_bytes());
    bytes[180..184].copy_from_slice(&0_u32.to_le_bytes());
    bytes[198..200].copy_from_slice(&1_u16.to_le_bytes());
    bytes[212..216].copy_from_slice(&1_u32.to_le_bytes());
    assert_eq!(read(&bytes).unwrap_err(), TerrainError::Cycle);
    // Repeated map/model owners share the admitted tree; no source identity sorting.
    assert!(read(&fixture()).is_ok());
    bytes[212..216].copy_from_slice(&0_u32.to_le_bytes());
    assert!(read(&bytes).is_ok());
}

#[test]
fn permits_empty_terrain_without_claiming_scene_completeness() {
    let mut bytes = fixture()[..HEADER_BYTES].to_vec();
    bytes[16..24].copy_from_slice(&(HEADER_BYTES as u64).to_le_bytes());
    bytes[104..136].fill(0);
    let empty = read(&bytes).unwrap();
    assert!(empty.aabbs().is_empty());
    assert!(empty.models().is_empty());
    assert!(empty.materials().is_empty());
    assert!(empty.packed_edges().is_empty());
}
struct Slow<'a>(&'a [u8]);
impl Read for Slow<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out.len().min(self.0.len()).min(3);
        out[..n].copy_from_slice(&self.0[..n]);
        self.0 = &self.0[n..];
        Ok(n)
    }
}
struct Failing(bool);
impl Read for Failing {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.0 {
            Ok(out.len() + 1)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "private message",
            ))
        }
    }
}
#[test]
fn bounds_read_attempts_and_rejects_oversize_before_reading() {
    let bytes = fixture();
    assert!(read_terrain_snapshot(&mut Slow(&bytes), bytes.len() as u64).is_ok());
    assert_eq!(
        read_terrain_snapshot(&mut Failing(false), MAX_BYTES + 1).unwrap_err(),
        TerrainError::LimitExceeded
    );
    assert_eq!(
        read_terrain_snapshot(&mut Failing(false), 144).unwrap_err(),
        TerrainError::InterruptedReadLimit
    );
    assert_eq!(
        read_terrain_snapshot(&mut Failing(true), 144).unwrap_err(),
        TerrainError::InvalidReadCount
    );
}
#[test]
#[ignore = "requires explicitly supplied external licensed terrain exchange"]
fn retained_full_payload_admission() {
    use sha2::{Digest, Sha256};
    use std::io::{BufReader, Seek, SeekFrom};
    let path =
        std::env::var_os("BO2_TERRAIN_SNAPSHOT").expect("explicit external terrain snapshot");
    let mut file = BufReader::new(std::fs::File::open(path).unwrap());
    let length = file.get_ref().metadata().unwrap().len();
    let snapshot = read_terrain_snapshot(&mut file, length).unwrap();
    let hash = |hex: &str| -> [u8; 32] {
        std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
    };
    snapshot
        .check_identity(TerrainIdentity {
            source: Identity {
                bytes: 78_753_344,
                sha256: hash("663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5"),
            },
            membership: Identity {
                bytes: 2_634_532,
                sha256: hash("6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3"),
            },
        })
        .unwrap();
    assert_eq!(
        (
            snapshot.materials().len(),
            snapshot.aabbs().len(),
            snapshot.partitions().len(),
            snapshot.triangles().len(),
            snapshot.vertices_bits().len(),
            snapshot.packed_edges().len()
        ),
        (509, 51_129, 18_968, 67_196, 40_025, 25_200)
    );
    assert_eq!(
        (snapshot.map_leaves().len(), snapshot.models().len()),
        (4791, 265)
    );
    let root_zero = |leaf: &&bo2_assets::terrain_snapshot::TerrainLeaf| {
        leaf.brush_root == 0 && leaf.terrain_contents != 0 && leaf.aabb_count > 0
    };
    let leaves = snapshot
        .map_leaves()
        .iter()
        .filter(root_zero)
        .collect::<Vec<_>>();
    assert_eq!(
        (
            leaves.len(),
            leaves
                .iter()
                .map(|l| usize::from(l.aabb_count))
                .sum::<usize>()
        ),
        (214, 253)
    );
    let models = snapshot
        .models()
        .iter()
        .map(|m| &m.leaf)
        .filter(root_zero)
        .collect::<Vec<_>>();
    assert_eq!(
        (
            models.len(),
            models
                .iter()
                .map(|l| usize::from(l.aabb_count))
                .sum::<usize>()
        ),
        (27, 28)
    );
    assert!(
        snapshot
            .models()
            .iter()
            .all(|model| !model.serialized_info_present)
    );
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 4096];
    let mut total = 0;
    loop {
        let size = file.read(&mut buffer).unwrap();
        if size == 0 {
            break;
        }
        total += size;
        digest.update(&buffer[..size]);
    }
    assert_eq!(total, 3_084_512);
    assert_eq!(
        <[u8; 32]>::from(digest.finalize()),
        hash("e5ecb51a076898aeae66cfb6fd38ef79218768c29d3089279303e39801315a67")
    );
}
