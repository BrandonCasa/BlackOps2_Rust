#![forbid(unsafe_code)]
use bo2_assets::{
    brush_collection::Identity,
    static_model_snapshot::{
        HEADER_BYTES, MAGIC, MAX_BYTES, StaticModelError, StaticModelIdentity, StaticModelSnapshot,
        read_static_model_snapshot,
    },
};
use std::io::{self, Read};
const I0: usize = 144;
const I1: usize = 232;
const P0: usize = 408;
const P1: usize = 464;
const S0: usize = 520;
const T0: usize = 616;
const C0: usize = 712;
fn word(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn half(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn bits(b: &mut Vec<u8>, values: &[f32]) {
    for v in values {
        word(b, v.to_bits());
    }
}
fn bounds(b: &mut Vec<u8>) {
    bits(b, &[-1.; 3]);
    bits(b, &[1.; 3]);
}
fn fixture() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&MAGIC);
    word(&mut b, 1);
    word(&mut b, 0);
    b.extend_from_slice(&728_u64.to_le_bytes());
    b.extend_from_slice(&10_u64.to_le_bytes());
    b.extend_from_slice(&[1; 32]);
    b.extend_from_slice(&20_u64.to_le_bytes());
    b.extend_from_slice(&[2; 32]);
    for count in [3, 2, 2, 2, 1] {
        word(&mut b, count);
    }
    b.extend_from_slice(&[0; 20]);
    assert_eq!(b.len(), HEADER_BYTES);
    for (i, provider) in [0, 1, 0].into_iter().enumerate() {
        word(&mut b, i as u32);
        word(&mut b, provider);
        half(&mut b, if i == 0 { 2 } else { 0 });
        half(&mut b, 0);
        word(&mut b, if i == 2 { 0 } else { u32::MAX });
        bits(&mut b, &[-0., i as f32, 3.]);
        bits(&mut b, &[1., 0., 0., 0., 1., 0., 0., 0., 1.]);
        bounds(&mut b);
    }
    for i in 0..2 {
        word(&mut b, i);
        word(&mut b, 0x8000_0000);
        half(&mut b, if i == 0 { 1 } else { 0xffff });
        half(&mut b, 0);
        word(&mut b, i);
        word(&mut b, 1);
        word(&mut b, if i == 0 { 0 } else { 1 });
        word(&mut b, if i == 0 { 1 } else { 0 });
        bounds(&mut b);
        word(&mut b, 1_f32.to_bits());
    }
    for i in 0..2 {
        word(&mut b, 0);
        word(&mut b, i);
        word(&mut b, 1);
        bounds(&mut b);
        word(&mut b, (-2_i32) as u32);
        word(&mut b, u32::MAX);
        word(&mut b, 0x8000_0000);
    }
    for _ in 0..2 {
        bits(&mut b, &[-0., 0., 1., 0., 1., 0., 0., 0., 0., 1., 0., 0.]);
    }
    for v in [0, 1, 2, 0x8000_0000] {
        word(&mut b, v);
    }
    assert_eq!(b.len(), 728);
    b
}
fn patch(b: &mut [u8], offset: usize, value: u32) {
    b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn read(b: &[u8]) -> Result<StaticModelSnapshot, StaticModelError> {
    read_static_model_snapshot(&mut &b[..], b.len() as u64)
}
#[test]
fn preserves_order_sharing_signed_lod_exact_words_and_unresolved_metadata() {
    let snapshot = read(&fixture()).unwrap();
    assert_eq!(
        snapshot
            .instances()
            .iter()
            .map(|s| s.provider)
            .collect::<Vec<_>>(),
        [0, 1, 0]
    );
    assert_eq!(snapshot.instances()[0].serialized_next, 2);
    assert_eq!(snapshot.instances()[2].contents, 0);
    assert_eq!(snapshot.instances()[0].origin_bits[0], (-0_f32).to_bits());
    assert_eq!(snapshot.providers()[1].collision_lod, -1);
    assert_eq!(snapshot.providers()[0].first_surface, 0);
    assert_eq!(snapshot.providers()[1].first_surface, 1);
    assert_eq!(snapshot.surfaces()[1].ordinal, 0);
    assert_eq!(snapshot.surfaces()[0].bone_index, -2);
    assert_eq!(snapshot.triangles()[0], snapshot.triangles()[1]);
    assert_eq!(snapshot.triangles()[0].plane_bits[0], (-0_f32).to_bits());
    assert!(snapshot.collmaps()[0].geometry_list_present);
    assert_eq!(snapshot.collmaps()[0].unresolved_geometry_count, 2);
    assert_eq!(snapshot.collmaps()[0].geometry_list_contents, 0x8000_0000);
    let expected = StaticModelIdentity {
        source: Identity {
            bytes: 10,
            sha256: [1; 32],
        },
        associations: Identity {
            bytes: 20,
            sha256: [2; 32],
        },
    };
    snapshot.check_identity(expected).unwrap();
    let mut changed = expected;
    changed.associations.sha256[0] = 3;
    assert_eq!(
        snapshot.check_identity(changed),
        Err(StaticModelError::IdentityMismatch)
    );
    assert!(!format!("{snapshot:?}").contains("plane_bits"));
    let mut absent = fixture();
    for offset in [C0 + 4, C0 + 8, C0 + 12] {
        patch(&mut absent, offset, 0);
    }
    let admitted = read(&absent).unwrap();
    assert!(!admitted.collmaps()[0].geometry_list_present);
}
#[test]
fn rejects_reference_order_count_padding_and_presence_failures() {
    for (offset, value, error) in [
        (I0 + 4, 2, StaticModelError::InvalidReference),
        (I0 + 4, 1, StaticModelError::InvalidOrder),
        (I0 + 8, 4, StaticModelError::InvalidReference),
        (I1, 0, StaticModelError::InvalidOrdinal),
        (P0, 1, StaticModelError::InvalidOrdinal),
        (P0 + 12, 1, StaticModelError::InvalidOrder),
        (P0 + 16, 3, StaticModelError::InvalidReference),
        (P1 + 16, 0, StaticModelError::InvalidOrder),
        (S0, 1, StaticModelError::InvalidOrdinal),
        (S0 + 4, 1, StaticModelError::InvalidOrder),
        (S0 + 8, 3, StaticModelError::InvalidReference),
        (C0, 1, StaticModelError::InvalidOrdinal),
        (C0 + 4, 2, StaticModelError::InvalidPresence),
        (C0 + 4, 0, StaticModelError::InvalidPresence),
        (124, 1, StaticModelError::NonzeroReserved),
        (104, u32::MAX, StaticModelError::LimitExceeded),
    ] {
        let mut b = fixture();
        patch(&mut b, offset, value);
        assert_eq!(read(&b).unwrap_err(), error, "offset {offset}");
    }
    let mut b = fixture();
    patch(&mut b, I1 + 4, 0);
    assert_eq!(read(&b).unwrap_err(), StaticModelError::InvalidOrder);
    for offset in [I0 + 10, P0 + 10] {
        let mut b = fixture();
        b[offset] = 1;
        assert_eq!(read(&b).unwrap_err(), StaticModelError::NonzeroReserved);
    }
}
#[test]
fn rejects_nonfinite_bounds_and_radius_without_changing_admitted_state() {
    let snapshot = read(&fixture()).unwrap();
    let saved = snapshot.triangles()[0];
    for offset in [
        I0 + 16,
        I0 + 28,
        I0 + 64,
        P0 + 28,
        P0 + 52,
        S0 + 12,
        T0,
        T0 + 16,
        T0 + 32,
    ] {
        for value in [
            f32::NAN.to_bits(),
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits(),
        ] {
            let mut b = fixture();
            patch(&mut b, offset, value);
            assert_eq!(read(&b).unwrap_err(), StaticModelError::NonFinite);
            assert_eq!(snapshot.triangles()[0], saved);
        }
    }
    for offset in [I0 + 64, P0 + 28, S0 + 12] {
        let mut b = fixture();
        patch(&mut b, offset, 2_f32.to_bits());
        assert_eq!(read(&b).unwrap_err(), StaticModelError::InvalidBounds);
    }
    let mut b = fixture();
    patch(&mut b, P0 + 52, (-1_f32).to_bits());
    assert_eq!(read(&b).unwrap_err(), StaticModelError::InvalidRadius);
}
#[test]
fn checks_exact_lengths_truncation_and_leaves_trailing_bytes_unread() {
    let b = fixture();
    for end in 0..b.len() {
        assert_eq!(
            read_static_model_snapshot(&mut &b[..end], b.len() as u64).unwrap_err(),
            StaticModelError::Truncated
        );
    }
    let mut changed = b.clone();
    patch(&mut changed, 16, 727);
    assert_eq!(
        read(&changed).unwrap_err(),
        StaticModelError::LengthMismatch
    );
    let mut changed = b.clone();
    changed[0] = 0;
    assert_eq!(
        read(&changed).unwrap_err(),
        StaticModelError::UnsupportedMagic
    );
    let mut changed = b.clone();
    patch(&mut changed, 8, 2);
    assert_eq!(
        read(&changed).unwrap_err(),
        StaticModelError::UnsupportedVersion
    );
    let mut extended = b.clone();
    extended.extend_from_slice(&[7, 8, 9]);
    let mut input = &extended[..];
    read_static_model_snapshot(&mut input, b.len() as u64).unwrap();
    assert_eq!(input, [7, 8, 9]);
}
struct Slow<'a>(&'a [u8]);
impl Read for Slow<'_> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.0.read(&mut b[..1])?;
        Ok(n)
    }
}
struct Failing(u8);
impl Read for Failing {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        match self.0 {
            0 => Err(io::ErrorKind::Interrupted.into()),
            1 => Ok(b.len() + 1),
            _ => Err(io::ErrorKind::PermissionDenied.into()),
        }
    }
}
#[test]
fn caps_reads_interruptions_and_invalid_read_implementations() {
    let b = fixture();
    assert!(read_static_model_snapshot(&mut Slow(&b), b.len() as u64).is_ok());
    assert_eq!(
        read_static_model_snapshot(&mut Failing(0), MAX_BYTES + 1).unwrap_err(),
        StaticModelError::LimitExceeded
    );
    assert_eq!(
        read_static_model_snapshot(&mut Failing(2), 0).unwrap_err(),
        StaticModelError::Truncated
    );
    assert_eq!(
        read_static_model_snapshot(&mut Failing(0), 144).unwrap_err(),
        StaticModelError::InterruptedReadLimit
    );
    assert_eq!(
        read_static_model_snapshot(&mut Failing(1), 144).unwrap_err(),
        StaticModelError::InvalidReadCount
    );
    assert_eq!(
        read_static_model_snapshot(&mut Failing(2), 144).unwrap_err(),
        StaticModelError::Io(io::ErrorKind::PermissionDenied)
    );
}
#[test]
fn caps_each_declared_family_and_accepts_empty_exchange_without_fake_providers() {
    for (i, cap) in [65_535, 65_535, 65_536, 1_000_000, 1_000_000]
        .into_iter()
        .enumerate()
    {
        let mut b = fixture();
        patch(&mut b, 104 + i * 4, cap + 1);
        assert_eq!(read(&b).unwrap_err(), StaticModelError::LimitExceeded);
    }
    let mut b = fixture();
    patch(&mut b, C0 + 8, 1_000_001);
    assert_eq!(read(&b).unwrap_err(), StaticModelError::LimitExceeded);
    let mut b = fixture();
    let second = b[C0..C0 + 16].to_vec();
    b.extend_from_slice(&second);
    patch(&mut b, 16, 744);
    patch(&mut b, 120, 2);
    patch(&mut b, P1 + 24, 1);
    patch(&mut b, C0 + 8, 600_000);
    patch(&mut b, C0 + 24, 600_001);
    assert_eq!(read(&b).unwrap_err(), StaticModelError::LimitExceeded);
    let mut b = fixture();
    b.truncate(HEADER_BYTES);
    patch(&mut b, 16, HEADER_BYTES as u32);
    for i in 0..5 {
        patch(&mut b, 104 + i * 4, 0);
    }
    let snapshot = read(&b).unwrap();
    assert!(snapshot.instances().is_empty());
    assert!(snapshot.providers().is_empty());
}
#[test]
#[ignore = "requires explicitly supplied external licensed classic static exchange"]
fn retained_full_payload_admission() {
    use sha2::{Digest, Sha256};
    use std::io::{BufReader, Seek, SeekFrom};
    let path = std::env::var_os("BO2_STATIC_MODEL_SNAPSHOT").expect("explicit external snapshot");
    let mut file = BufReader::new(std::fs::File::open(path).unwrap());
    let length = file.get_ref().metadata().unwrap().len();
    let snapshot = read_static_model_snapshot(&mut file, length).unwrap();
    let hash = |hex: &str| -> [u8; 32] {
        std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
    };
    snapshot
        .check_identity(StaticModelIdentity {
            source: Identity {
                bytes: 78_753_344,
                sha256: hash("663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5"),
            },
            associations: Identity {
                bytes: 3_209_708,
                sha256: hash("d322619cd3b97284006f0673e1b3fc817cfaf6a41528b34a1e5752213ee8271b"),
            },
        })
        .unwrap();
    assert_eq!(
        (
            snapshot.instances().len(),
            snapshot.providers().len(),
            snapshot.surfaces().len(),
            snapshot.triangles().len(),
            snapshot.collmaps().len()
        ),
        (10_837, 420, 510, 78_296, 2)
    );
    assert!(snapshot.instances().iter().all(|s| s.serialized_next == 0));
    assert_eq!(
        snapshot
            .instances()
            .iter()
            .filter(|s| s.contents == 0)
            .count(),
        210
    );
    for (contents, count) in [(1, 10_140), (16, 150), (17, 285), (3, 52)] {
        assert_eq!(
            snapshot
                .instances()
                .iter()
                .filter(|s| s.contents == contents)
                .count(),
            count
        );
    }
    for (lod, count) in [(0, 53), (1, 189), (2, 136), (3, 42)] {
        assert_eq!(
            snapshot
                .providers()
                .iter()
                .filter(|p| p.collision_lod == lod)
                .count(),
            count
        );
    }
    assert!(snapshot.providers().iter().all(|p| p.surface_count > 0));
    assert!(snapshot.surfaces().iter().all(|s| s.triangle_count > 0));
    assert!(
        snapshot
            .collmaps()
            .iter()
            .all(|c| c.geometry_list_present && c.unresolved_geometry_count == 1)
    );
    assert_eq!(
        snapshot
            .collmaps()
            .iter()
            .map(|c| c.geometry_list_contents)
            .collect::<Vec<_>>(),
        [536_871_936, 1_024]
    );
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 4096];
    let mut total = 0;
    let mut bounded = (&mut file).take(length + 1);
    loop {
        let count = bounded.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        total += count;
        digest.update(&buffer[..count]);
    }
    assert_eq!(total, 4_760_040);
    assert_eq!(
        <[u8; 32]>::from(digest.finalize()),
        hash("a4f4024660d26cf564012c629099d64941b5b47ce77701aa89da62fe50b8fce0")
    );
}
