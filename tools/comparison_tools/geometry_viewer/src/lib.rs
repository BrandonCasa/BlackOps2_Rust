#![forbid(unsafe_code)]

//! Chosen Bevy-frame diagnostic policy, not original camera or rendering semantics.

use std::{fmt, io};

use bevy::{camera::ScalingMode, mesh::VertexAttributeValues, prelude::*};
use bo2_assets::geometry_snapshot::{
    GeometrySnapshot, SnapshotError, SnapshotSurface, read_geometry_snapshot,
};

pub mod camera;

pub const IMAGE_SIDE: u32 = 512;
pub const IMAGE_BYTES: usize = IMAGE_SIDE as usize * IMAGE_SIDE as usize * 4;
pub const VIEW_NAMES: [&str; 3] = ["raw-z", "raw-x", "raw-oblique"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticError {
    Io {
        stage: &'static str,
        kind: io::ErrorKind,
    },
    Snapshot(SnapshotError),
    Appearance(bo2_assets::appearance::AppearanceError),
    NotRegularFile,
    WorkspacePath,
    EmptyGeometry,
    UnframeableGeometry,
    EngineUnavailable,
    OverlayFeaturesUnavailable,
    EnginePanic,
    PipelineFailure,
    Deadline,
    ReadbackLayout,
    BackgroundOnly,
    Encoding,
    Preservation,
    InteractiveValidation,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { stage, kind } => write!(f, "{stage} I/O ({kind:?})"),
            Self::Snapshot(error) => error.fmt(f),
            other => write!(f, "{other:?}"),
        }
    }
}
impl std::error::Error for DiagnosticError {}

pub fn io_error(stage: &'static str, error: io::Error) -> DiagnosticError {
    DiagnosticError::Io {
        stage,
        kind: error.kind(),
    }
}

pub struct DiagnosticView {
    pub transform: Transform,
    pub projection: OrthographicProjection,
}

pub struct PreparedGeometry {
    pub mesh: Mesh,
    pub surfaces: Vec<SnapshotSurface>,
    pub views: [DiagnosticView; 3],
    pub positions: usize,
    pub indices: usize,
}

/// Frame all stored positions, including unreferenced records, without editing them.
/// Admission bounds f32 camera arithmetic and rejects poorly resolved/extreme data.
/// These limits, sphere framing and views are tool policies, never BO2 conventions.
pub fn views(positions: &[[f32; 3]]) -> Result<[DiagnosticView; 3], DiagnosticError> {
    if positions.is_empty() {
        return Err(DiagnosticError::EmptyGeometry);
    }
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for position in positions {
        for axis in 0..3 {
            let value = f64::from(position[axis]);
            if !value.is_finite() || value.abs() > 10_000_000.0 {
                return Err(DiagnosticError::UnframeableGeometry);
            }
            low[axis] = low[axis].min(value);
            high[axis] = high[axis].max(value);
        }
    }
    let center = std::array::from_fn::<_, 3, _>(|i| (low[i] + high[i]) * 0.5);
    let radius = (0..3)
        .map(|i| ((high[i] - low[i]) * 0.5).powi(2))
        .sum::<f64>()
        .sqrt();
    let magnitude = low
        .into_iter()
        .chain(high)
        .map(f64::abs)
        .fold(0.0, f64::max);
    if !(0.001..=1_000_000.0).contains(&radius) || radius < magnitude * 0.000_01 {
        return Err(DiagnosticError::UnframeableGeometry);
    }
    let center = Vec3::from_array(center.map(|v| v as f32));
    let radius = radius as f32;
    // Sphere fits every orthographic view with at least 10% image margin per edge.
    // Camera distance = 3 radii; geometry depth = [2r,4r], clip interval = [r,5r].
    Ok([
        (Vec3::Z, Vec3::Y),
        (Vec3::X, Vec3::Z),
        (Vec3::ONE.normalize(), Vec3::Y),
    ]
    .map(|(direction, up)| DiagnosticView {
        transform: Transform::from_translation(center + direction * (3.0 * radius))
            .looking_at(center, up),
        projection: OrthographicProjection {
            near: radius,
            far: 5.0 * radius,
            scaling_mode: ScalingMode::Fixed {
                width: 2.5 * radius,
                height: 2.5 * radius,
            },
            ..OrthographicProjection::default_3d()
        },
    }))
}

pub fn prepare(snapshot: GeometrySnapshot) -> Result<PreparedGeometry, DiagnosticError> {
    if snapshot.indices().is_empty() {
        return Err(DiagnosticError::EmptyGeometry);
    }
    let positions = snapshot.positions().len();
    let indices = snapshot.indices().len();
    let bo2_rendering::gray_map::GrayMap { mesh, surfaces } =
        bo2_rendering::gray_map::prepare_gray_map(snapshot);
    let Some(VertexAttributeValues::Float32x3(canonical_positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err(DiagnosticError::Preservation);
    };
    let views = views(canonical_positions)?;
    Ok(PreparedGeometry {
        mesh,
        surfaces,
        views,
        positions,
        indices,
    })
}

/// Arbitrary asymmetric 3D tetrahedron; never generated from licensed input.
pub fn synthetic_snapshot() -> Result<GeometrySnapshot, DiagnosticError> {
    let positions = [
        [-2.0_f32, -1.0, 0.0],
        [3.0, -1.0, 0.0],
        [-1.0, 2.0, 0.0],
        [0.5, 0.25, 4.0],
    ];
    let indices = [0_u32, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3];
    let mut bytes = Vec::with_capacity(208);
    bytes.extend_from_slice(b"BO2DGS\0\0");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    for value in [4_u64, 12, 1, 208] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0; 16]);
    for p in positions {
        for v in p {
            bytes.extend_from_slice(&v.to_bits().to_le_bytes());
        }
    }
    for i in indices {
        bytes.extend_from_slice(&i.to_le_bytes());
    }
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&12_u64.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    for v in [-2.0_f32, -1.0, 0.0, 3.0, 2.0, 4.0] {
        bytes.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    read_geometry_snapshot(&mut bytes.as_slice(), 208).map_err(DiagnosticError::Snapshot)
}

/// Require the chosen dark opaque background, a clear border, and neutral foreground.
/// Qualification additionally compares synthetic silhouettes against projected triangles.
pub fn foreground_pixels(bytes: &[u8]) -> Result<usize, DiagnosticError> {
    if bytes.len() != IMAGE_BYTES {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let background = &bytes[..4];
    if background[3] != 255 || background[..3].iter().any(|channel| *channel > 20) {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let mut count = 0;
    for (i, pixel) in bytes.as_chunks::<4>().0.iter().enumerate() {
        let x = i % IMAGE_SIDE as usize;
        let y = i / IMAGE_SIDE as usize;
        if x < 8 || y < 8 || x >= IMAGE_SIDE as usize - 8 || y >= IMAGE_SIDE as usize - 8 {
            if pixel != background {
                return Err(DiagnosticError::ReadbackLayout);
            }
        } else if pixel != background {
            if pixel[3] != 255 || pixel[0] < 24 || pixel[0] != pixel[1] || pixel[1] != pixel[2] {
                return Err(DiagnosticError::ReadbackLayout);
            }
            count += 1;
        }
    }
    if count == 0 {
        return Err(DiagnosticError::BackgroundOnly);
    }
    Ok(count)
}
