use bevy::render::settings::WgpuSettings;

pub(crate) fn settings() -> WgpuSettings {
    let mut settings = WgpuSettings::default();
    settings
        .instance_flags
        .remove(bevy::render::settings::InstanceFlags::DEBUG);
    #[cfg(windows)]
    {
        settings.dx12_shader_compiler = bevy::render::settings::Dx12Compiler::StaticDxc;
        if std::env::var_os("WGPU_VALIDATION").is_none()
            && settings
                .backends
                .is_some_and(|backends| backends.contains(bevy::render::settings::Backends::DX12))
        {
            settings
                .instance_flags
                .remove(bevy::render::settings::InstanceFlags::VALIDATION);
        }
    }
    #[cfg(windows)]
    {
        settings = prefer_dx12(settings);
    }
    settings
}

#[cfg(windows)]
fn prefer_dx12(mut settings: WgpuSettings) -> WgpuSettings {
    use bevy::render::settings::Backends;

    if Backends::from_env().is_some() {
        return settings;
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: Backends::DX12,
        flags: settings.instance_flags,
        backend_options: wgpu::BackendOptions {
            dx12: wgpu::Dx12BackendOptions {
                shader_compiler: settings.dx12_shader_compiler.clone(),
                ..Default::default()
            },
            ..Default::default()
        },
        memory_budget_thresholds: settings.instance_memory_budget_thresholds,
        display: None,
    });
    let desired_name = std::env::var("WGPU_ADAPTER_NAME")
        .ok()
        .or_else(|| settings.adapter_name.clone());
    let force_software = std::env::var("WGPU_FORCE_FALLBACK_ADAPTER")
        .map_or(settings.force_fallback_adapter, |value| {
            !(value.is_empty() || value == "0" || value == "false")
        });
    let supported = bevy::tasks::block_on(async {
        let mut adapters = instance.enumerate_adapters(Backends::DX12).await;
        adapters.sort_by_key(|adapter| match adapter.get_info().device_type {
            wgpu::DeviceType::DiscreteGpu => {
                if settings.power_preference == wgpu::PowerPreference::LowPower {
                    1
                } else {
                    0
                }
            }
            wgpu::DeviceType::IntegratedGpu => {
                if settings.power_preference == wgpu::PowerPreference::LowPower {
                    0
                } else {
                    1
                }
            }
            _ => 2,
        });
        adapters.into_iter().find(|adapter| {
            let info = adapter.get_info();
            let software = info.device_type == wgpu::DeviceType::Cpu;
            software == force_software
                && desired_name
                    .as_ref()
                    .is_none_or(|name| info.name.to_lowercase().contains(&name.to_lowercase()))
                && adapter.features().contains(settings.features)
                && settings.limits.check_limits(&adapter.limits())
        })
    });
    if let Some(adapter) = supported {
        settings.backends = Some(Backends::DX12);
        settings.adapter_name = Some(adapter.get_info().name);
    } else {
        eprintln!("No supported DirectX 12 adapter found; falling back to Vulkan.");
        settings.backends = Some(Backends::VULKAN);
    }
    settings
}
