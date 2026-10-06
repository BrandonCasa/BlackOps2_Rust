//! Nonexpressive measured engine-generated image definitions, keyed by exact source identity.
use crate::appearance::AppearanceError;

/// SHA256 of the source image identifier, including the source reference prefix.
pub const IDENTITY_NORMAL_SOURCE_SHA256: &str =
    "3343bb032d046705946aeb4c58e273a17c592f51f69a12df0eefe62053b66eca";

/// A shared-zone material image reference, distinct from runtime shadow atlases.
pub const SHADOW_MATERIAL_SOURCE_SHA256: &str =
    "df04ff1564886176b0257baa1e44483e9add8571251fd8f12dea9e3e97014de1";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalBuiltinDefinition {
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub format: crate::full_scene::DdsFormat,
    pub dds_sha256: &'static str,
    pub source_container_sha256: &'static str,
}
/// The source reference resolves to a real shared-zone image, not generated pixels.
pub fn external_definition_for_source(
    source_sha256: &str,
) -> Result<ExternalBuiltinDefinition, AppearanceError> {
    if source_sha256 != SHADOW_MATERIAL_SOURCE_SHA256 {
        return Err(AppearanceError::Unsupported);
    }
    Ok(ExternalBuiltinDefinition {
        width: 256,
        height: 256,
        mips: 9,
        format: crate::full_scene::DdsFormat::Bc1,
        dds_sha256: "96409c359760f7c2b7a03b2720931c4bb4272e0e1cdc888d16a1bc01585fa3f4",
        source_container_sha256: "dc44f775074e010e37f3fbdf60fc21af1de2582634432cd6057d6eecd0099942",
    })
}
/// Admit the external source image only with its qualified exact dependency identity.
pub fn load_external_builtin(
    root: &std::path::Path,
    source_sha256: &str,
    image: &crate::full_scene::SceneImage,
) -> Result<crate::full_scene::SceneImageInput, AppearanceError> {
    let d = external_definition_for_source(source_sha256)?;
    if image.sha256 != d.dds_sha256 {
        return Err(AppearanceError::Identity);
    }
    let dds = crate::full_scene::identified(root, &image.file, &image.sha256, 1024 * 1024)?;
    let metadata = crate::full_scene::inspect_dds(&dds)?;
    if (
        metadata.width,
        metadata.height,
        metadata.mips,
        metadata.format,
        metadata.cube,
    ) != (d.width, d.height, d.mips, d.format, false)
    {
        return Err(AppearanceError::Attributes);
    }
    Ok(crate::full_scene::SceneImageInput { dds, metadata })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinImageDefinition {
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub srgb: bool,
    pub rgba8: [u8; 4],
}

/// Exact observed identity-normal initializer. Sampling remains material-owned.
/// Unknown identities, including the shadow builtin, have no replacement image.
pub fn definition_for_source(
    source_sha256: &str,
) -> Result<BuiltinImageDefinition, AppearanceError> {
    if source_sha256 != IDENTITY_NORMAL_SOURCE_SHA256 {
        return Err(AppearanceError::Unsupported);
    }
    Ok(BuiltinImageDefinition {
        width: 1,
        height: 1,
        mips: 1,
        srgb: false,
        rgba8: [128, 128, 255, 128],
    })
}

/// Check generated external bytes against the measured definition as well as
/// the enclosing manifest's file hash. Only linear RGBA8 is admitted.
pub fn validate_external_builtin(source_sha256: &str, dds: &[u8]) -> Result<(), AppearanceError> {
    use crate::full_scene::{DdsFormat, inspect_dds};
    let definition = definition_for_source(source_sha256)?;
    let metadata = inspect_dds(dds)?;
    if metadata.format != DdsFormat::Rgba8
        || metadata.width != definition.width
        || metadata.height != definition.height
        || metadata.mips != definition.mips
        || metadata.cube
        || dds.get(dds.len().saturating_sub(4)..) != Some(&definition.rgba8[..])
    {
        return Err(AppearanceError::Identity);
    }
    Ok(())
}
