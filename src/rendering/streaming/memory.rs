use bevy::{
    asset::{AssetId, UntypedAssetId},
    pbr::PreparedMaterial,
    prelude::*,
    render::{
        erased_render_asset::{ErasedRenderAssets, ExtractedAssets as ExtractedMaterials},
        extract_resource::ExtractResource,
        material_bind_groups::{MaterialBindGroupAllocators, RenderMaterialBindings},
        render_asset::{ExtractedAssets, RenderAssets},
        render_resource::{Texture, TextureDescriptor},
        renderer::{RenderAdapterInfo, RenderQueue},
        texture::GpuImage,
    },
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub(super) enum CapacityError {
    Unavailable(String),
    Invalid(String),
}
impl std::fmt::Display for CapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) | Self::Invalid(message) => f.write_str(message),
        }
    }
}

pub(super) struct Capacity {
    pub bytes: u64,
    pub source: &'static str,
}

fn query(mut command: Command) -> Result<String, CapacityError> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CapacityError::Unavailable(error.to_string()))?;
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| CapacityError::Unavailable(error.to_string()))?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| CapacityError::Unavailable(error.to_string()))?;
            if !output.status.success() {
                return Err(CapacityError::Unavailable(
                    String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                ));
            }
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CapacityError::Unavailable(
                "GPU capacity query timed out".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn detect(adapter: &RenderAdapterInfo) -> Result<Capacity, CapacityError> {
    if adapter.vendor == 0x10de {
        let mut command = Command::new("nvidia-smi");
        command.args([
            "--query-gpu=pci.device_id,name,memory.total",
            "--format=csv,noheader,nounits",
        ]);
        if let Ok(output) = query(command) {
            let capacity = output
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = line.split(',').map(str::trim).collect();
                    if fields.len() != 3 || !fields[1].eq_ignore_ascii_case(&adapter.name) {
                        return None;
                    }
                    let pci = u32::from_str_radix(fields[0].trim_start_matches("0x"), 16).ok()?;
                    if pci & 0xffff != adapter.vendor || pci >> 16 != adapter.device {
                        return None;
                    }
                    fields[2].parse::<u64>().ok()?.checked_mul(1024 * 1024)
                })
                .filter(|bytes| *bytes > 0)
                .min();
            if let Some(bytes) = capacity {
                return Ok(Capacity {
                    bytes,
                    source: "NVIDIA driver dedicated memory",
                });
            }
        }
    }
    #[cfg(windows)]
    {
        let script = format!(
            "$ErrorActionPreference='Stop'; $name=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{}')); $values=Get-ChildItem -LiteralPath 'HKLM:/SYSTEM/CurrentControlSet/Control/Video' | ForEach-Object {{ $p=Get-ItemProperty -LiteralPath ($_.PSPath+'/0000') -ErrorAction SilentlyContinue; $size=$p.'HardwareInformation.qwMemorySize'; if ($p.DriverDesc -eq $name -and $p.MatchingDeviceId -like '*VEN_{:04X}&DEV_{:04X}*' -and ($size -is [UInt64] -or $size -is [Int64]) -and $size -gt 0) {{ $size }} }}; $values | Measure-Object -Minimum | ForEach-Object {{$_.Minimum.ToString('0',[Globalization.CultureInfo]::InvariantCulture)}}",
            base64_name(&adapter.name),
            adapter.vendor,
            adapter.device
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        if let Ok(output) = query(command) {
            if let Ok(bytes) = output.trim().parse::<u64>() {
                if bytes > 0 {
                    return Ok(Capacity {
                        bytes,
                        source: "selected Windows driver 64-bit dedicated memory",
                    });
                }
            }
        }
    }
    Err(CapacityError::Invalid(format!(
        "Cannot establish dedicated VRAM for selected adapter {} ({:04x}:{:04x}); texture streaming requires a driver capacity report",
        adapter.name, adapter.vendor, adapter.device
    )))
}

#[cfg(windows)]
fn base64_name(name: &str) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for block in name.as_bytes().chunks(3) {
        let value = (u32::from(block[0]) << 16)
            | (u32::from(*block.get(1).unwrap_or(&0)) << 8)
            | u32::from(*block.get(2).unwrap_or(&0));
        output.push(DIGITS[((value >> 18) & 63) as usize] as char);
        output.push(DIGITS[((value >> 12) & 63) as usize] as char);
        output.push(if block.len() > 1 {
            DIGITS[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if block.len() > 2 {
            DIGITS[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[derive(Resource, Clone, Default)]
pub(super) struct MemoryLedger(Arc<Mutex<u64>>);
impl MemoryLedger {
    pub fn retiring_bytes(&self) -> u64 {
        *self.0.lock().expect("Texture memory ledger")
    }
    pub fn replace(&self, old_bytes: u64) {
        *self.0.lock().expect("Texture memory ledger") += old_bytes;
    }
}

#[derive(Clone)]
struct Replacement {
    cumulative_bytes: u64,
    descriptor: TextureDescriptor<'static>,
}

#[derive(Resource, Clone, Default, ExtractResource)]
#[extract_app(bevy::render::RenderApp)]
pub(super) struct Replacements(HashMap<AssetId<Image>, Replacement>);
impl Replacements {
    pub fn record(
        &mut self,
        id: AssetId<Image>,
        bytes: u64,
        descriptor: TextureDescriptor<'static>,
    ) {
        let cumulative_bytes = self
            .0
            .get(&id)
            .map_or(0, |previous| previous.cumulative_bytes)
            + bytes;
        self.0.insert(
            id,
            Replacement {
                cumulative_bytes,
                descriptor,
            },
        );
    }
    fn matching<'a>(
        &'a self,
        extracted: &'a ExtractedAssets<GpuImage>,
        matched: &'a MatchedReplacements,
    ) -> impl Iterator<Item = (AssetId<Image>, &'a Replacement)> {
        extracted.extracted.iter().filter_map(|(id, source)| {
            let pending = self.0.get(id)?;
            (pending.cumulative_bytes > matched.0.get(id).copied().unwrap_or(0)
                && pending.descriptor == source.texture_descriptor)
                .then_some((*id, pending))
        })
    }
}
#[derive(Resource, Default)]
pub(super) struct MatchedReplacements(HashMap<AssetId<Image>, u64>);

#[derive(Resource, Default)]
pub(super) struct RetiringTextures(Vec<Retirement>);

#[derive(Resource, Default)]
pub(super) struct MaterialImages(HashMap<UntypedAssetId, Vec<UntypedAssetId>>);

fn observe_materials<M: Material>(
    extracted: &ExtractedMaterials<MeshMaterial3d<M>>,
    known: &mut MaterialImages,
    updated: &mut HashSet<UntypedAssetId>,
    visit: impl Fn(&M, &mut Vec<UntypedAssetId>),
) where
    M::Data: Eq + std::hash::Hash + Clone,
{
    for id in &extracted.removed {
        known.0.remove(&id.untyped());
    }
    for (id, material) in &extracted.extracted {
        let mut images = Vec::new();
        visit(material, &mut images);
        known.0.insert(id.untyped(), images);
        updated.insert(id.untyped());
    }
}

fn image_dependencies<A: Asset>(asset: &A, images: &mut Vec<UntypedAssetId>) {
    asset.visit_dependencies(&mut |id| {
        if id.type_id() == TypeId::of::<Image>() {
            images.push(id);
        }
    });
}

pub(super) fn invalidate_material_bindings(
    standard: Res<ExtractedMaterials<MeshMaterial3d<StandardMaterial>>>,
    opaque: Res<ExtractedMaterials<MeshMaterial3d<super::super::opaque::OpaqueMaterial>>>,
    diagnostic: Res<ExtractedMaterials<MeshMaterial3d<super::super::pbr::DiagnosticMaterial>>>,
    replacements: Res<Replacements>,
    extracted_images: Res<ExtractedAssets<GpuImage>>,
    matched: Res<MatchedReplacements>,
    mut known: ResMut<MaterialImages>,
    mut bindings: ResMut<RenderMaterialBindings>,
    mut allocators: ResMut<MaterialBindGroupAllocators>,
    mut prepared: ResMut<ErasedRenderAssets<PreparedMaterial>>,
) {
    let changed: HashSet<_> = replacements
        .matching(&extracted_images, &matched)
        .map(|(id, _)| id.untyped())
        .collect();
    let mut updated = HashSet::new();
    observe_materials(&standard, &mut known, &mut updated, image_dependencies);
    observe_materials(&opaque, &mut known, &mut updated, |material, images| {
        image_dependencies(&material.base, images);
        image_dependencies(&material.extension, images);
    });
    observe_materials(&diagnostic, &mut known, &mut updated, |material, images| {
        image_dependencies(&material.base, images);
    });
    updated.extend(known.0.iter().filter_map(|(id, images)| {
        images
            .iter()
            .any(|image| changed.contains(image))
            .then_some(*id)
    }));
    for id in updated {
        if let Some(binding) = bindings.remove(&id) {
            if let Some(allocator) = allocators.get_mut(&id.type_id()) {
                allocator.free(binding);
            }
        }
        prepared.remove(id);
    }
}
struct Retirement {
    id: AssetId<Image>,
    texture: Option<Texture>,
    expected: TextureDescriptor<'static>,
    bytes: u64,
    materials: Vec<UntypedAssetId>,
}

pub(super) fn capture_replaced(
    images: Res<RenderAssets<GpuImage>>,
    extracted: Res<ExtractedAssets<GpuImage>>,
    replacements: Res<Replacements>,
    mut matched: ResMut<MatchedReplacements>,
    mut retiring: ResMut<RetiringTextures>,
    known: Res<MaterialImages>,
) {
    let captured: Vec<_> = replacements
        .matching(&extracted, &matched)
        .map(|(id, pending)| (id, pending.clone()))
        .collect();
    for (id, pending) in captured {
        let previous = matched.0.insert(id, pending.cumulative_bytes).unwrap_or(0);
        retiring.0.push(Retirement {
            id,
            texture: images.get(id).map(|image| image.texture.clone()),
            expected: pending.descriptor.clone(),
            bytes: pending.cumulative_bytes - previous,
            materials: known
                .0
                .iter()
                .filter_map(|(material, images)| {
                    images.contains(&id.untyped()).then_some(*material)
                })
                .collect(),
        });
    }
}

pub(super) fn release_completed(
    queue: Res<RenderQueue>,
    ledger: Res<MemoryLedger>,
    mut retiring: ResMut<RetiringTextures>,
    images: Res<RenderAssets<GpuImage>>,
    materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    known: Res<MaterialImages>,
    bindings: Res<RenderMaterialBindings>,
) {
    if retiring.0.is_empty() {
        return;
    }
    let mut textures = Vec::new();
    retiring.0.retain_mut(|retirement| {
        let Some(current) = images.get(retirement.id) else {
            return true;
        };
        if retirement.materials.iter().any(|id| {
            known.0.contains_key(id) && materials.get(*id).is_none_or(|material| {
                    bindings.get(id).is_none_or(|binding| {
                        binding.group.0 != material.binding.group.0
                            || binding.slot.0 != material.binding.slot.0
                    })
            })
        }) {
            return true;
        }
        let reused = retirement
            .texture
            .as_ref()
            .is_some_and(|old| old.id() == current.texture.id());
        if let Some(old) = &retirement.texture {
            if images.iter().any(|(_, image)| image.texture_view.texture() == &**old) {
                if !reused {
                    warn!("Texture ownership: {:?} old allocation {:?} still owns a live native image view", retirement.id, old.id());
                }
                if !reused || current.texture_descriptor != retirement.expected {
                    return true;
                }
            }
        }
        if reused && current.texture_descriptor != retirement.expected {
            return true;
        }
        textures.push((
            if reused {
                None
            } else {
                retirement.texture.take()
            },
            retirement.bytes,
        ));
        false
    });
    if textures.is_empty() {
        return;
    }
    let ledger = ledger.clone();
    queue.on_submitted_work_done(move || {
        let mut bytes = 0;
        for (texture, size) in textures {
            if let Some(texture) = texture {
                texture.destroy();
            }
            bytes += size;
        }
        let mut retained = ledger.0.lock().expect("Texture memory ledger");
        *retained = retained.saturating_sub(bytes);
    });
}
