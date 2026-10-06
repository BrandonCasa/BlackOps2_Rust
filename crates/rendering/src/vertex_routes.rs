//! Native semantic routing to admitted world attributes; registers are not semantics.
use crate::original_material::{SourceAttribute, VertexInput, original_world_mesh};
use bevy::{mesh::VertexAttributeValues, prelude::Mesh};
use bo2_assets::{
    full_scene::FullSceneInput,
    secondary_stream::{SourceSecondaryCorner, SourceTexcoord},
    shader_bank::{ShaderBankInput, ShaderDeclaration, ShaderPass, ShaderStage},
    vertex_signature::{
        InputComponentType, InputSystemValue, ShaderVertexSignature, StreamSource,
        decode_stream_route, source_for_input,
    },
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VertexRouteError {
    MissingSignature,
    SignatureIdentity,
    ReflectionMismatch,
    InvalidRoute,
    ConflictingDestination,
    UnsupportedSourceFormat(u32),
    UnsupportedInput,
    MissingSemantic { location: u32 },
    MissingSource { source: StreamSource, format: u32 },
    ComponentMismatch { location: u32 },
    MissingSecondary,
    InvalidAttributes,
}
fn attribute(
    source: StreamSource,
    components: u8,
    format: u32,
    location: u32,
) -> Result<SourceAttribute, VertexRouteError> {
    use SourceAttribute as A;
    use StreamSource as S;
    use VertexRouteError as E;
    let result = match (source, components) {
        (S::Position, 3) => A::Position3,
        (S::Position, 4) => A::Position4,
        (S::Color, 4) => A::Color4,
        (S::Texcoord0, 2) => A::Uv0,
        (S::Texcoord1, 2) => A::Uv1,
        (S::Normal, 3) => A::Normal3,
        (S::Normal, 4) => A::Normal4,
        (S::Tangent, 3) => A::Tangent3,
        (S::Tangent, 4) => A::Tangent4,
        (S::Texcoord2, 4) if matches!(format, 1 | 2 | 3 | 4 | 5 | 7) => A::SecondaryTexcoord0,
        (S::Texcoord3, 4) if format == 7 => A::SecondaryTexcoord1,
        (S::NormalTransform0, 4) if matches!(format, 2 | 4 | 5 | 7) => A::SecondaryNormalTransform0,
        (S::NormalTransform1, 4) if format == 5 => A::SecondaryNormalTransform1,
        (
            S::Texcoord2
            | S::Texcoord3
            | S::NormalTransform0
            | S::NormalTransform1
            | S::BlendWeight,
            _,
        ) => return Err(E::MissingSource { source, format }),
        _ => return Err(E::ComponentMismatch { location }),
    };
    Ok(result)
}
/// The source signature is hash-admitted independently from translated reflection.
pub fn route_vertex_signature(
    declaration: &ShaderDeclaration,
    signature: &ShaderVertexSignature,
    routing: &[[u8; 2]],
    format: u32,
) -> Result<Vec<VertexInput>, VertexRouteError> {
    use VertexRouteError as E;
    if !matches!(format, 0 | 1 | 2 | 3 | 4 | 5 | 7) {
        return Err(E::UnsupportedSourceFormat(format));
    }
    if declaration.stage != ShaderStage::Vertex
        || signature.source_shader_sha256 != declaration.source_sha256
    {
        return Err(E::SignatureIdentity);
    }
    let mut destinations = BTreeSet::new();
    for pair in routing {
        let route = decode_stream_route(*pair).map_err(|_| E::InvalidRoute)?;
        if !destinations.insert(route.destination.semantic()) {
            return Err(E::ConflictingDestination);
        }
    }
    if signature.inputs.len() != declaration.inputs.len() {
        return Err(E::ReflectionMismatch);
    }
    let mut locations = BTreeSet::new();
    let mut result = Vec::new();
    for input in &signature.inputs {
        if input.component_type != InputComponentType::Float32
            || input.system_value != InputSystemValue::None
            || !matches!(input.component_mask, 1 | 3 | 7 | 15)
            || input.used_mask & !input.component_mask != 0
            || !locations.insert(input.register)
        {
            return Err(E::UnsupportedInput);
        }
        let components = input.component_mask.count_ones() as u8;
        if !declaration
            .inputs
            .iter()
            .any(|i| i.location == input.register && i.components == components)
        {
            return Err(E::ReflectionMismatch);
        }
        let source = source_for_input(routing, input).map_err(|_| E::MissingSemantic {
            location: input.register,
        })?;
        result.push(VertexInput {
            location: input.register,
            attribute: attribute(source, components, format, input.register)?,
        });
    }
    result.sort_by_key(|i| i.location);
    Ok(result)
}
/// Public endpoint for selecting a real material/pass shader route.
pub fn world_vertex_route(
    bank: &ShaderBankInput,
    pass: &ShaderPass,
    format: u32,
) -> Result<Vec<VertexInput>, VertexRouteError> {
    let signature = bank
        .signature_for_shader(pass.vertex_shader)
        .map_err(|_| VertexRouteError::MissingSignature)?;
    let declaration = &bank
        .shaders
        .get(pass.vertex_shader)
        .ok_or(VertexRouteError::MissingSignature)?
        .declaration;
    route_vertex_signature(declaration, signature, &pass.routing, format)
}
fn expand(uv: SourceTexcoord) -> [f32; 4] {
    match uv {
        SourceTexcoord::Float2([x, y]) => [x, y, 0.0, 1.0],
        SourceTexcoord::Float4(v) => v,
    }
}
/// Build only requested secondary inputs. HALF2 format expansion follows native
/// Direct3D IA defaults (missing z=0,w=1); absent stream data never receives defaults.
pub fn routed_world_mesh(
    input: &FullSceneInput,
    surface: usize,
    secondary: Option<&[SourceSecondaryCorner]>,
    route: &[VertexInput],
) -> Result<Mesh, VertexRouteError> {
    use SourceAttribute as A;
    use VertexRouteError as E;
    let format = input
        .surfaces
        .get(surface)
        .ok_or(E::InvalidAttributes)?
        .source_vertex_format;
    if !matches!(format, 0 | 1 | 2 | 3 | 4 | 5 | 7) {
        return Err(E::UnsupportedSourceFormat(format));
    }
    let range = input
        .geometry
        .surfaces()
        .get(surface)
        .ok_or(E::InvalidAttributes)?
        .index_range();
    let mut mesh = original_world_mesh(input, surface).map_err(|_| E::InvalidAttributes)?;
    let mut seen = BTreeSet::new();
    for r in route {
        if !seen.insert(r.location) {
            return Err(E::ConflictingDestination);
        }
        if mesh.attribute(r.attribute.attribute()).is_some() {
            continue;
        }
        if r.attribute == A::Position3 {
            let Some(VertexAttributeValues::Float32x4(positions)) =
                mesh.attribute(A::Position4.attribute())
            else {
                return Err(E::InvalidAttributes);
            };
            let positions: Vec<[f32; 3]> = positions.iter().map(|p| [p[0], p[1], p[2]]).collect();
            mesh.insert_attribute(A::Position3.attribute(), positions);
            continue;
        }
        let corners = secondary.ok_or(E::MissingSecondary)?;
        if corners.len() != input.geometry.indices().len() {
            return Err(E::InvalidAttributes);
        }
        let source = match r.attribute {
            A::SecondaryTexcoord0 => StreamSource::Texcoord2,
            A::SecondaryTexcoord1 => StreamSource::Texcoord3,
            A::SecondaryNormalTransform0 => StreamSource::NormalTransform0,
            A::SecondaryNormalTransform1 => StreamSource::NormalTransform1,
            _ => return Err(E::InvalidAttributes),
        };
        attribute(source, 4, format, r.location)?;
        let mut values = Vec::new();
        for corner in range.clone() {
            let a = corners[corner];
            let value = match r.attribute {
                A::SecondaryTexcoord0 => {
                    let uv = a.texcoord3.ok_or(E::MissingSecondary)?;
                    if matches!(format, 1 | 2) != matches!(uv, SourceTexcoord::Float2(_)) {
                        return Err(E::InvalidAttributes);
                    }
                    expand(uv)
                }
                A::SecondaryTexcoord1 => {
                    let [x, y] = a.texcoord4.ok_or(E::MissingSecondary)?;
                    [x, y, 0.0, 1.0]
                }
                A::SecondaryNormalTransform0 => a.color5.ok_or(E::MissingSecondary)?,
                A::SecondaryNormalTransform1 => a.color6.ok_or(E::MissingSecondary)?,
                _ => return Err(E::InvalidAttributes),
            };
            if value.iter().any(|c| !c.is_finite())
                || (matches!(
                    r.attribute,
                    A::SecondaryNormalTransform0 | A::SecondaryNormalTransform1
                ) && value.iter().any(|c| !(0.0..=1.0).contains(c)))
            {
                return Err(E::InvalidAttributes);
            }
            values.push(value);
        }
        mesh.insert_attribute(r.attribute.attribute(), values);
    }
    Ok(mesh)
}
