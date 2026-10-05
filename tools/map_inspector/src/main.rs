#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use bo2_assets::brush_collection::{CollectionError, read_brush_collection};
use bo2_assets::brush_snapshot::{BrushError, BrushSnapshot, read_brush_snapshot};
use bo2_assets::geometry_snapshot::{GeometrySnapshot, SnapshotError, read_geometry_snapshot};
use bo2_physics::{
    DVec3, Vec3,
    brush::{Convention, ConvexBrush, QueryError},
    brush_collection::BrushCollection,
    moving_brush_trace::{ContactPlane, MovingBrushRequest, TraceEffect, TraceResult},
};

const HELP: &str = "Usage: bo2-map-inspector [--] <diagnostic-snapshot-file>
       bo2-map-inspector --brush <one-schema28-record>
       bo2-map-inspector --brush-collection <one-schema34-collection>
       bo2-map-inspector --help

Read-only inspection of one project-owned diagnostic geometry exchange snapshot v1.
This is not a BO2 disk format. Emits bounded counts/categories/status only; no
positions, index values, bounds, paths or names. Source authenticity, rendering
and original-game fidelity remain unverified. Fixed resource policies apply.
No recursive discovery, configuration loading, automatic originals or output files.

--brush reads one project-owned external schema28 brush record (64KiB maximum).
It emits counts, endpoint-derived point-segment probes and precision diagnostics.
Diagnostic n dot point - distance <= 0 is not original player collision proof.
No source coordinates, bits, paths or materials are emitted.

--brush-collection reads an external schema34 collection (32MiB maximum).
All selected records, including diagnostic zero-side axial hulls, participate.
Source identity hashes are declarations; standalone inspection cannot authenticate
the original or prove runtime world ownership. Unsupported families stay explicit.
Representatives also exercise one-brush f32 moving point queries with diagnostic
zero extents/all-bits masks. Stationary queries and original output comparison
remain unsupported/unverified; these are not collection/world traces.

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
        Action::Brush(path) => inspect_brush(Path::new(&path)),
        Action::BrushCollection(path) => inspect_brush_collection(Path::new(&path)),
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
    Brush(OsString),
    BrushCollection(OsString),
}

fn arguments(mut arguments: impl Iterator<Item = OsString>) -> Result<Action, ()> {
    match (arguments.next(), arguments.next(), arguments.next()) {
        (Some(help), None, None) if help == "--help" || help == "-h" => Ok(Action::Help),
        (Some(path), None, None) if !path.to_string_lossy().starts_with('-') => {
            Ok(Action::Inspect(path))
        }
        (Some(separator), Some(path), None) if separator == "--" => Ok(Action::Inspect(path)),
        (Some(mode), Some(path), None) if mode == "--brush" => Ok(Action::Brush(path)),
        (Some(mode), Some(path), None) if mode == "--brush-collection" => {
            Ok(Action::BrushCollection(path))
        }
        _ => Err(()),
    }
}

#[derive(Debug)]
enum InspectorError {
    InputIo { kind: io::ErrorKind },
    NotRegularFile,
    Snapshot(SnapshotError),
    Brush(BrushError),
    Collection(CollectionError),
    Query(QueryError),
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
            Self::Brush(error) => error.fmt(output),
            Self::Collection(error) => error.fmt(output),
            Self::Query(error) => error.fmt(output),
            Self::OutputIo { kind } => write!(output, "output write failed ({kind:?})"),
        }
    }
}

fn inspect_brush(path: &Path) -> Result<(), InspectorError> {
    let mut file = File::open(path).map_err(input_error)?;
    let metadata = file.metadata().map_err(input_error)?;
    if !metadata.is_file() {
        return Err(InspectorError::NotRegularFile);
    }
    let snapshot = read_brush_snapshot(&mut file, metadata.len()).map_err(InspectorError::Brush)?;
    let hull = ConvexBrush::from_snapshot(&snapshot, Convention::DiagnosticMinusDistanceAtMostZero)
        .map_err(InspectorError::Query)?;
    let diagnostics = hull
        .precision_diagnostics(&snapshot)
        .map_err(InspectorError::Query)?;
    let [lo, hi] = hull.endpoints();
    let center = (lo + hi) / 2.0;
    let span = hi - lo;
    // These are named diagnostic probes, not player dimensions, velocities or
    // assertions that the endpoint midpoint must lie in every admitted hull.
    let mut probes = Vec::with_capacity(5);
    probes.push((
        "midpoint_stationary",
        hull.clip_segment(center, center)
            .map_err(InspectorError::Query)?,
    ));
    for axis in 0..3 {
        let mut offset = DVec3::ZERO;
        offset[axis] = span[axis];
        probes.push((
            ["x_crossing", "y_crossing", "z_crossing"][axis],
            hull.clip_segment(center - offset, center + offset)
                .map_err(InspectorError::Query)?,
        ));
    }
    probes.push((
        "outside_stationary",
        hull.clip_segment(hi + span, hi + span)
            .map_err(InspectorError::Query)?,
    ));
    let mut output = io::stdout().lock();
    writeln!(output, "inspection: schema28 brush contract valid\nside_records: {}\nvertex_records: {}\nconstraints: {}\nparallel_bounds_present: {}\nparallel_contents_present: {}\nconvention: diagnostic_n_dot_point_minus_distance_le_zero\nstrict_source_violations: {}\nold_tolerant_source_violations: {}\nmax_positive_residual_raw_units: {:.17}\nreconstructed_vertices: {}\nvertex_set_max_error_raw_units: {:?}\nf32_rounding_joint_witness: {:?}\nwitness_vertex_set_max_error_raw_units: {:?}\nmax_coefficient_rounding_cell_fraction: {:.12}\nmax_vertex_rounding_cell_fraction: {:.12}\noriginal_producer_rounding: unverified\noriginal_player_collision_fidelity: UNKNOWN",
        snapshot.sides().len(), snapshot.vertices_bits().len(), hull.planes().len(), snapshot.midpoint_bits().is_some(), snapshot.parallel_contents().is_some(),
        diagnostics.strict_violations, diagnostics.old_tolerant_violations, diagnostics.max_positive_residual,
        diagnostics.reconstructed_vertices, diagnostics.vertex_set_max_error, diagnostics.rounding_witness, diagnostics.witness_vertex_set_max_error,
        diagnostics.max_coefficient_cell_fraction, diagnostics.max_vertex_cell_fraction).map_err(output_error)?;
    for (name, hit) in probes {
        match hit {
            Some(hit) => writeln!(
                output,
                "probe_{name}: entering={:.12} exiting={:.12} start_inside={}",
                hit.entering, hit.exiting, hit.start_inside
            ),
            None => writeln!(output, "probe_{name}: no_hit"),
        }
        .map_err(output_error)?;
    }
    Ok(())
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

fn inspect_brush_collection(path: &Path) -> Result<(), InspectorError> {
    let mut file = File::open(path).map_err(input_error)?;
    let metadata = file.metadata().map_err(input_error)?;
    if !metadata.is_file() {
        return Err(InspectorError::NotRegularFile);
    }
    let snapshot =
        read_brush_collection(&mut file, metadata.len()).map_err(InspectorError::Collection)?;
    let collection =
        BrushCollection::from_snapshot(&snapshot, Convention::DiagnosticMinusDistanceAtMostZero)
            .map_err(InspectorError::Query)?;
    let zero = snapshot
        .records()
        .iter()
        .filter(|r| r.snapshot().sides().is_empty())
        .count();
    let sides: usize = snapshot
        .records()
        .iter()
        .map(|r| r.snapshot().sides().len())
        .sum();
    let vertices: usize = snapshot
        .records()
        .iter()
        .map(|r| r.snapshot().vertices_bits().len())
        .sum();
    let c = snapshot.coverage();
    let mut output = io::stdout().lock();
    let residuals = collection
        .source_vertex_diagnostics(&snapshot)
        .map_err(InspectorError::Query)?;
    writeln!(output,"inspection: schema34 diagnostic brush collection contract valid\nselected_records: {}\nadmitted_queryable_hulls: {}\nzero_side_axial_diagnostic_hulls: {}\nside_records: {}\nsource_vertex_records: {}\nreached_selection_nodes: {}\nselection_interpretation: diagnostic_relative_negative_adjacent_zero_empty\nidentity_binding: declarations_only_no_original_authentication\nsource_coordinates: untransformed\nmodel_providers_absent: {} of {}\nbox_excluded: {}\nunsupported_triangles: {}\nunsupported_partitions: {}\nunsupported_aabb_trees: {}\nunsupported_static_models: {}\nunsupported_dynents: {}\nunsupported: masks_player_hulls_terrain_runtime_transforms\nnearest_tie_policy: exact_entering_then_lowest_source_key\ndata_interpretation_fidelity: INVESTIGATING\noriginal_player_collision_fidelity: UNKNOWN",
      snapshot.records().len(),collection.brushes().len(),zero,sides,vertices,snapshot.reached_nodes(),c.absent_model_providers,c.models,c.box_excluded,c.triangles,c.partitions,c.aabb_trees,c.static_models,c.dynents.iter().map(|&n|u64::from(n)).sum::<u64>()).map_err(output_error)?;
    for (group, d) in [("all", residuals.all), ("zero_side", residuals.zero_side)] {
        writeln!(output,"source_residuals_{group}: strict_violations={} old_tolerant_violations={} strict_affected_brushes={} old_tolerant_affected_brushes={} max_positive_residual_raw_units={:.17}",d.strict_violations,d.old_tolerant_violations,d.strict_affected_brushes,d.old_tolerant_affected_brushes,d.max_positive_residual).map_err(output_error)?;
    }
    for (name, index) in [
        (
            "positive_representative",
            snapshot
                .records()
                .iter()
                .position(|r| !r.snapshot().sides().is_empty()),
        ),
        (
            "zero_side_representative",
            snapshot
                .records()
                .iter()
                .position(|r| {
                    r.snapshot().sides().is_empty() && r.snapshot().vertices_bits().len() != 8
                })
                .or_else(|| {
                    snapshot
                        .records()
                        .iter()
                        .position(|r| r.snapshot().sides().is_empty())
                }),
        ),
    ] {
        let Some(index) = index else {
            writeln!(output, "{name}: unavailable").map_err(output_error)?;
            continue;
        };
        let hull = collection.brushes()[index].hull();
        let s = snapshot.records()[index].snapshot();
        let d = hull
            .precision_diagnostics(s)
            .map_err(InspectorError::Query)?;
        writeln!(output,"{name}_precision: source_vertices={} reconstructed_vertices={} strict_violations={} old_tolerant_violations={} vertex_set_max_error_raw_units={:?} f32_joint_witness={:?}\n{name}_geometry: {}",s.vertices_bits().len(),d.reconstructed_vertices,d.strict_violations,d.old_tolerant_violations,d.vertex_set_max_error,d.rounding_witness,if s.sides().is_empty(){"diagnostic_six_axial_endpoints_source_shape_unproved"}else{"preserved_sides_plus_six_axial_endpoints"}).map_err(output_error)?;
        write_moving_probes(&mut output, name, s).map_err(output_error)?;
        let [lo, hi] = hull.endpoints();
        let center = (lo + hi) / 2.0;
        let span = hi - lo;
        let probes = [
            ("midpoint_stationary", center, center),
            (
                "x_crossing",
                center - DVec3::X * span.x,
                center + DVec3::X * span.x,
            ),
            (
                "y_crossing",
                center - DVec3::Y * span.y,
                center + DVec3::Y * span.y,
            ),
            (
                "z_crossing",
                center - DVec3::Z * span.z,
                center + DVec3::Z * span.z,
            ),
            ("outside_stationary", hi + span, hi + span),
        ];
        for (probe, start, end) in probes {
            match collection
                .clip_segment(start, end)
                .map_err(InspectorError::Query)?
            {
                Some(hit) => writeln!(
                    output,
                    "probe_{name}_{probe}: entering={:.12} exiting={:.12} start_inside={}",
                    hit.interval.entering, hit.interval.exiting, hit.interval.start_inside
                ),
                None => writeln!(output, "probe_{name}_{probe}: no_hit"),
            }
            .map_err(output_error)?;
        }
    }
    Ok(())
}
fn write_moving_probes(
    output: &mut impl Write,
    name: &str,
    brush: &BrushSnapshot,
) -> io::Result<()> {
    writeln!(
        output,
        "moving_f32_{name}: single_brush point_subset zero_extents all_bits_mask incoming_fraction=1 stationary=unsupported original_output_comparison=unverified"
    )?;
    let lo = brush.mins_bits().map(f32::from_bits);
    let hi = brush.maxs_bits().map(f32::from_bits);
    let center = std::array::from_fn::<_, 3, _>(|axis| (lo[axis] + hi[axis]) * 0.5);
    for (axis, label) in ["x", "y", "z"].into_iter().enumerate() {
        let mut start = center;
        let mut end = center;
        let span = hi[axis] - lo[axis];
        start[axis] -= span;
        end[axis] += span;
        let mut result = TraceResult {
            normal: Vec3::ZERO,
            fraction: 1.0,
            surface_flags: 0,
            contents: 0,
            startsolid: false,
            allsolid: false,
            opaque_flag_2c: 0,
        };
        let effect = MovingBrushRequest::new(
            Vec3::from_array(start),
            Vec3::from_array(end),
            Vec3::ZERO,
            Vec3::ZERO,
            u32::MAX,
        )
        .and_then(|request| request.trace(brush, &mut result));
        match effect {
            Ok(effect) => {
                let (status, plane) = match effect {
                    TraceEffect::Entry(ContactPlane::Axial { .. }) => ("entry", "axial"),
                    TraceEffect::Entry(ContactPlane::Side { .. }) => ("entry", "side"),
                    TraceEffect::Inside => ("inside", "none"),
                    TraceEffect::Rejected => ("rejected", "none"),
                    TraceEffect::MaskedOut => ("masked_out", "none"),
                };
                writeln!(
                    output,
                    "moving_f32_{name}_{label}_crossing: status={status} fraction={:.9} startsolid={} allsolid={} normal_present={} plane={plane}",
                    result.fraction,
                    result.startsolid,
                    result.allsolid,
                    matches!(effect, TraceEffect::Entry(_))
                )?;
            }
            Err(error) => writeln!(output, "moving_f32_{name}_{label}_crossing: {error}")?,
        }
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
