//! Qualified source shader semantics and original stream routing identities.
use crate::{
    appearance::AppearanceError,
    shader_bank::{ShaderDeclaration, ShaderStage, TechniqueFamily},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputComponentType {
    Float32,
    Uint32,
    Sint32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSystemValue {
    None,
    VertexId,
    InstanceId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceVertexInput {
    pub semantic_name: String,
    pub semantic_index: u32,
    pub register: u32,
    pub component_mask: u8,
    pub used_mask: u8,
    pub component_type: InputComponentType,
    pub system_value: InputSystemValue,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderVertexSignature {
    pub source_shader_sha256: String,
    pub inputs: Vec<SourceVertexInput>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamSource {
    Position,
    Color,
    Texcoord0,
    Normal,
    Tangent,
    Texcoord1,
    Texcoord2,
    Texcoord3,
    NormalTransform0,
    NormalTransform1,
    BlendWeight,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamDestination {
    Position,
    Normal,
    Color(u8),
    Depth,
    Texcoord(u8),
    BlendWeight,
}
impl StreamDestination {
    pub fn semantic(self) -> (&'static str, u32) {
        match self {
            Self::Position => ("POSITION", 0),
            Self::Normal => ("NORMAL", 0),
            Self::Color(n) => ("COLOR", u32::from(n)),
            Self::Depth => ("DEPTH", 0),
            Self::Texcoord(n) => ("TEXCOORD", u32::from(n)),
            Self::BlendWeight => ("BLENDWEIGHT", 0),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamRoute {
    pub source: StreamSource,
    pub destination: StreamDestination,
}
/// Numeric identities come from the pinned original-format declarations.
pub fn decode_stream_route(pair: [u8; 2]) -> Result<StreamRoute, AppearanceError> {
    use StreamDestination as D;
    use StreamSource as S;
    let source = match pair[0] {
        0 => S::Position,
        1 => S::Color,
        2 => S::Texcoord0,
        3 => S::Normal,
        4 => S::Tangent,
        5 => S::Texcoord1,
        6 => S::Texcoord2,
        7 => S::Texcoord3,
        8 => S::NormalTransform0,
        9 => S::NormalTransform1,
        10 => S::BlendWeight,
        _ => return Err(AppearanceError::Unsupported),
    };
    let destination = match pair[1] {
        0 => D::Position,
        1 => D::Normal,
        2..=3 => D::Color(pair[1] - 2),
        4 => D::Depth,
        5..=18 => D::Texcoord(pair[1] - 5),
        19 => D::BlendWeight,
        _ => return Err(AppearanceError::Unsupported),
    };
    Ok(StreamRoute {
        source,
        destination,
    })
}
/// Resolve a source stream through semantic identity. Registers alone are not routes.
pub fn source_for_input(
    routing: &[[u8; 2]],
    input: &SourceVertexInput,
) -> Result<StreamSource, AppearanceError> {
    if input.system_value != InputSystemValue::None {
        return Err(AppearanceError::Unsupported);
    }
    let mut found = None;
    for pair in routing {
        let route = decode_stream_route(*pair)?;
        let (name, index) = route.destination.semantic();
        if name.eq_ignore_ascii_case(&input.semantic_name)
            && index == input.semantic_index
            && found.replace(route.source).is_some()
        {
            return Err(AppearanceError::Attributes);
        }
    }
    found.ok_or(AppearanceError::Attributes)
}
pub(crate) fn validate_signatures(
    signatures: &[ShaderVertexSignature],
    shaders: &[ShaderDeclaration],
    families: &[TechniqueFamily],
) -> Result<(), AppearanceError> {
    use AppearanceError as E;
    if signatures.len() > 4096 {
        return Err(E::Limit);
    }
    let mut by_hash = BTreeMap::new();
    for s in signatures {
        let shader = shaders
            .iter()
            .find(|d| d.source_sha256 == s.source_shader_sha256)
            .ok_or(E::Reference)?;
        if shader.stage != ShaderStage::Vertex
            || by_hash.insert(&s.source_shader_sha256, s).is_some()
        {
            return Err(E::Reference);
        }
        if s.inputs.len() > 32 {
            return Err(E::Limit);
        }
        let mut registers = BTreeSet::new();
        let mut semantics = BTreeSet::new();
        let mut generic = 0;
        for i in &s.inputs {
            if i.semantic_name.is_empty() || i.semantic_name.len() > 64 {
                return Err(E::Limit);
            }
            if i.register >= 32
                || !registers.insert(i.register)
                || i.component_mask == 0
                || i.component_mask > 15
                || i.used_mask & !i.component_mask != 0
            {
                return Err(E::Attributes);
            }
            let semantic = i.semantic_name.to_ascii_uppercase();
            if !semantics.insert((semantic.clone(), i.semantic_index)) {
                return Err(E::Attributes);
            }
            if i.system_value != InputSystemValue::None {
                let expected = match i.system_value {
                    InputSystemValue::VertexId => "SV_VERTEXID",
                    InputSystemValue::InstanceId => "SV_INSTANCEID",
                    InputSystemValue::None => return Err(E::Attributes),
                };
                if semantic != expected
                    || i.semantic_index != 0
                    || i.component_mask != 1
                    || i.component_type != InputComponentType::Uint32
                {
                    return Err(E::Unsupported);
                }
                continue;
            }
            if i.component_type != InputComponentType::Float32 {
                return Err(E::Unsupported);
            }
            let valid = match semantic.as_str() {
                "POSITION" | "NORMAL" | "DEPTH" | "BLENDWEIGHT" => i.semantic_index == 0,
                "COLOR" => i.semantic_index <= 1,
                "TEXCOORD" => i.semantic_index <= 13,
                _ => false,
            };
            if !valid {
                return Err(E::Unsupported);
            }
            let count = i.component_mask.count_ones() as u8;
            if i.component_mask != (1 << count) - 1
                || !shader
                    .inputs
                    .iter()
                    .any(|v| v.location == i.register && v.components == count)
            {
                return Err(E::Attributes);
            }
            generic += 1;
        }
        if generic != shader.inputs.len() {
            return Err(E::Attributes);
        }
    }
    if shaders
        .iter()
        .filter(|s| s.stage == ShaderStage::Vertex)
        .any(|s| !by_hash.contains_key(&s.source_sha256))
    {
        return Err(E::Attributes);
    }
    for f in families {
        for t in &f.techniques {
            for p in &t.passes {
                let shader = &shaders[p.vertex_shader];
                let sig = by_hash.get(&shader.source_sha256).ok_or(E::Attributes)?;
                let mut dests = BTreeSet::new();
                for pair in &p.routing {
                    let route = decode_stream_route(*pair)?;
                    if !dests.insert(route.destination.semantic()) {
                        return Err(E::Attributes);
                    }
                }
                for i in &sig.inputs {
                    if i.system_value == InputSystemValue::None
                        && !dests.iter().any(|(name, index)| {
                            name.eq_ignore_ascii_case(&i.semantic_name)
                                && *index == i.semantic_index
                        })
                    {
                        return Err(E::Attributes);
                    }
                }
            }
        }
    }
    Ok(())
}
