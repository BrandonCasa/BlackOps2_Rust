use super::*;

pub(crate) fn pass(material: &Value, index: usize) -> Result<&Value> {
    let name = material["name"].as_str().unwrap_or("<unnamed>");
    let technique = material["techniques"]
        .as_array()
        .and_then(|techniques| {
            techniques
                .iter()
                .find(|technique| technique["index"] == index)
        })
        .ok_or_else(|| format!("Material {name} is missing technique {index}"))?;
    let passes = technique["passes"]
        .as_array()
        .ok_or_else(|| format!("Material {name} technique {index} is missing passes"))?;
    match passes.as_slice() {
        [pass] => Ok(pass),
        _ => Err(format!(
            "Material {name} technique {index} requires one pass, found {}",
            passes.len()
        )
        .into()),
    }
}

pub(crate) fn texture_binding(material: &Value, hash: u32) -> Result<&Value> {
    material["textures"]
        .as_array()
        .and_then(|textures| textures.iter().find(|texture| texture["hash"] == hash))
        .ok_or_else(|| {
            format!(
                "Material {} is missing texture binding {hash}",
                material["name"]
            )
            .into()
        })
}

pub(crate) fn literal(material: &Value, hash: u32) -> Result<[f32; 4]> {
    let constant = material["constants"]
        .as_array()
        .and_then(|constants| constants.iter().find(|constant| constant["hash"] == hash))
        .ok_or_else(|| format!("Material {} is missing constant {hash}", material["name"]))?;
    let literal: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
    if literal.iter().any(|value| !value.is_finite()) {
        return Err(format!(
            "Material {} has a non-finite constant {hash}",
            material["name"]
        )
        .into());
    }
    Ok(literal)
}
