use super::*;

pub(super) fn assemble(
    pack: &pack::Pack,
    models: Vec<serde_json::Value>,
    worlds: Vec<serde_json::Value>,
    images: Vec<serde_json::Value>,
    sounds: Vec<serde_json::Value>,
) -> Result<serde_json::Value> {
    use serde_json::{Value, json};
    let game: Value = serde_json::from_slice(pack.get("game.json")
        .map_err(|_| "Required game.json conversion is missing: native gameplay profiles, content tables, placements, presentation/animation bindings and UI/font definitions must be converted before writing origins.pack")?)?;
    if game["version"] != 2 {
        return Err("Required game.json must use runtime content version 2".into());
    }
    let content = game["content"]
        .as_object()
        .ok_or("Required gameplay content catalog is missing")?;
    let presentations = content.get("presentations").and_then(Value::as_array)
        .ok_or("Required gameplay presentations are missing")?.iter().filter(|row| !row["scene"].is_null()).map(|row| {
            let id = row["id"].as_str().ok_or("Gameplay presentation ID is missing")?;
            let scene = row["scene"].as_str().ok_or("Gameplay presentation scene is missing")?;
            let uri = scene.split('#').next().ok_or("Gameplay presentation URI is missing")?;
            let model = models.iter().find(|model| model["uri"] == uri || model["lods"].as_array().into_iter().flatten().any(|lod| lod["uri"] == uri))
                .ok_or_else(|| format!("Required gameplay presentation has no exact exported model binding: {id} ({scene})"))?;
            pack.get(uri)?;
            let animations = model["animations"].as_object().ok_or_else(|| format!("Native model animation bindings are missing: {id}"))?;
            Ok(json!({"id":id,"uri":uri,"animations":animations,"name":model["name"],"zone":model["zone"]}))
        }).collect::<Result<Vec<_>>>()?;
    for animation in content
        .get("animations")
        .and_then(Value::as_array)
        .ok_or("Required gameplay animations are missing")?
    {
        let id = animation["id"]
            .as_str()
            .ok_or("Gameplay animation ID is missing")?;
        let owner = presentations
            .iter()
            .find(|row| row["id"] == animation["skeleton"])
            .ok_or_else(|| format!("Required gameplay animation has no exported skeleton: {id}"))?;
        let matches: Vec<_> = owner["animations"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(name, _)| {
                id.to_ascii_lowercase()
                    .ends_with(&format!(".{}", name.to_ascii_lowercase()))
            })
            .collect();
        if matches.len() != 1 || matches[0].1.as_u64().is_none() {
            return Err(format!(
                "Required gameplay animation has no unique exported clip binding: {id}"
            )
            .into());
        }
    }
    let ui_assets = content.get("ui_assets").and_then(Value::as_array).ok_or("Required gameplay UI/font definitions are missing")?
        .iter().map(|row| {
            let id = row["id"].as_str().ok_or("Gameplay UI ID is missing")?;
            let uri = row["path"].as_str().ok_or("Gameplay UI image URI is missing")?;
            let image = images.iter().find(|image| image["uris"]["srgb"] == uri)
                .ok_or_else(|| format!("Required gameplay UI image has no exact exported image binding: {id} ({uri})"))?;
            pack.get(uri)?;
            Ok(json!({"id":id,"name":image["name"],"zone":image["zone"],"uri":uri}))
        }).collect::<Result<Vec<_>>>()?;
    let effects_uri = "prepared/effects.nativefx";
    let camo_uri = "prepared/camo.json";
    pack.get(effects_uri)
        .map_err(|_| "Required native FX conversion is missing; no runtime pack was written")?;
    let camo_uri = if game["delivery_scope"] == "first_playable_solo" {
        None
    } else {
        pack.get(camo_uri).map_err(
            |_| "Required native camo conversion is missing; no runtime pack was written",
        )?;
        Some(camo_uri)
    };
    if worlds.iter().all(|world| world["map"] != "zm_tomb") {
        return Err("Required Origins world scene is missing".into());
    }
    Ok(
        json!({"schema_version":1,"models":models,"worlds":worlds,"images":images,"sounds":sounds,
        "presentations":presentations,"ui_assets":ui_assets,"effects_uri":effects_uri,"camo_uri":camo_uri}),
    )
}
