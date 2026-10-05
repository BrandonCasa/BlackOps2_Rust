use super::*;
use bo2_assets::appearance::load_material_scene;
use bo2_rendering::appearance::material_scene;
fn load(path: &Path) -> Result<(PreparedGeometry, Image), DiagnosticError> {
    let path = path
        .canonicalize()
        .map_err(|e| io_error("scene identity", e))?;
    outside_workspace(&path)?;
    let input = load_material_scene(&path).map_err(DiagnosticError::Appearance)?;
    let source_surface = input.surface;
    let scene = material_scene(input, bevy::image::CompressedImageFormats::BC)
        .map_err(DiagnosticError::Appearance)?;
    let mesh = scene.mesh;
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err(DiagnosticError::Preservation);
    };
    let views = bo2_geometry_viewer::views(positions)?;
    let count = positions.len();
    println!(
        "Appearance preview: surface {source_surface}; original triangle corners/UV order; diagnostic unlit color, linear sampling; original transfer function, material equations and lighting INVESTIGATING"
    );
    Ok((
        PreparedGeometry {
            mesh,
            surfaces: vec![scene.surface_record],
            views,
            positions: count,
            indices: count,
        },
        scene.image,
    ))
}
pub fn interactive(path: &Path, frames: Option<u32>, smoke: bool) -> Result<(), DiagnosticError> {
    let (geometry, image) = load(path)?;
    super::interactive::run_with_image(geometry, frames, smoke, Some(image))
}
pub fn execute(path: &Path, output: &Path) -> Result<(), DiagnosticError> {
    let (geometry, image) = load(path)?;
    let output = new_output(output)?;
    let report = render(geometry, Some(image))?;
    let mut coverage = Vec::new();
    for (i, bytes) in report.images.iter().enumerate() {
        let background = &bytes[..4];
        let foreground = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| *p != background)
            .count();
        let chromatic = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] != p[1] || p[1] != p[2])
            .count();
        coverage.push((foreground, chromatic));
        let mut file = BufWriter::new(exclusive_file(
            &output.join(format!("{}.png", VIEW_NAMES[i])),
        )?);
        PngEncoder::new(&mut file)
            .write_image(
                bytes,
                IMAGE_SIDE,
                IMAGE_SIDE,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|_| DiagnosticError::Encoding)?;
        file.flush().map_err(|e| io_error("PNG flush", e))?;
    }
    if coverage.iter().all(|&(foreground, _)| foreground == 0) {
        return Err(DiagnosticError::BackgroundOnly);
    }
    println!("Appearance foreground/chromatic pixels: {coverage:?}");
    println!(
        "Appearance GPU preview: {} updates; indexed mesh/UV/image prepared; three 512x512 captures; full appearance unsupported",
        report.frames
    );
    Ok(())
}
