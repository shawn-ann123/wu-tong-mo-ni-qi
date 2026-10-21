use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{
    Backends, InstanceFlags, PowerPreference, RenderCreation, WgpuSettings,
};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuTier {
    NvidiaDiscrete,
    IntelDiscrete,
    AmdDiscrete,
    OtherDiscrete,
    IntelIntegrated,
    OtherIntegrated,
    Cpu,
}

impl GpuTier {
    pub fn priority_rank(self) -> u8 {
        match self {
            Self::NvidiaDiscrete => 0,
            Self::IntelDiscrete => 1,
            Self::AmdDiscrete => 2,
            Self::OtherDiscrete => 3,
            Self::IntelIntegrated => 4,
            Self::OtherIntegrated => 5,
            Self::Cpu => 6,
        }
    }

    pub fn is_integrated(self) -> bool {
        matches!(self, Self::IntelIntegrated | Self::OtherIntegrated)
    }

    pub fn is_intel(self) -> bool {
        matches!(self, Self::IntelIntegrated | Self::IntelDiscrete)
    }
}

#[derive(Debug, Clone)]
pub struct AdapterCandidate {
    pub name: String,
    pub vendor: String,
    pub tier: GpuTier,
    pub is_vulkan: bool,
    pub is_uma: bool,
}

#[derive(Resource, Debug, Clone)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub vendor: String,
    pub tier: GpuTier,
    pub is_vulkan: bool,
    pub is_uma: bool,
}

impl GpuAdapterInfo {
    pub fn is_intel(&self) -> bool {
        self.tier.is_intel()
    }

    pub fn is_integrated(&self) -> bool {
        self.tier.is_integrated()
    }
}

static SELECTED_ADAPTER: OnceLock<AdapterCandidate> = OnceLock::new();

pub fn selected_adapter() -> Option<&'static AdapterCandidate> {
    SELECTED_ADAPTER.get()
}

fn classify_adapter(info: &wgpu::AdapterInfo) -> GpuTier {
    let name_lower = info.name.to_ascii_lowercase();

    let is_nvidia = name_lower.contains("nvidia")
        || name_lower.contains("geforce")
        || name_lower.contains("rtx")
        || name_lower.contains("gtx")
        || name_lower.contains("quadro");

    let is_intel = name_lower.contains("intel")
        || name_lower.contains("arc")
        || name_lower.contains("iris")
        || name_lower.contains("uhd")
        || name_lower.contains("hd graphics");

    let is_amd =
        name_lower.contains("amd") || name_lower.contains("radeon") || name_lower.contains("rx ");

    let is_discrete = info.device_type == wgpu::DeviceType::DiscreteGpu;
    let is_integrated = info.device_type == wgpu::DeviceType::IntegratedGpu;
    let is_cpu = info.device_type == wgpu::DeviceType::Cpu;

    if is_cpu {
        return GpuTier::Cpu;
    }

    if is_nvidia && is_discrete {
        return GpuTier::NvidiaDiscrete;
    }
    if is_intel && is_discrete {
        return GpuTier::IntelDiscrete;
    }
    if is_amd && is_discrete {
        return GpuTier::AmdDiscrete;
    }
    if is_intel && is_integrated {
        return GpuTier::IntelIntegrated;
    }
    if is_discrete {
        return GpuTier::OtherDiscrete;
    }
    if is_integrated {
        return GpuTier::OtherIntegrated;
    }

    GpuTier::OtherIntegrated
}

fn enumerate_candidates() -> Vec<AdapterCandidate> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: Backends::VULKAN,
        flags: InstanceFlags::default() | InstanceFlags::ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let adapters = futures::executor::block_on(instance.enumerate_adapters(Backends::VULKAN));

    let mut candidates = Vec::new();

    for adapter in adapters {
        let info = adapter.get_info();
        let tier = classify_adapter(&info);
        let is_vulkan = info.backend == wgpu::Backend::Vulkan;

        let is_uma = tier.is_integrated()
            || info.device_type == wgpu::DeviceType::IntegratedGpu
            || info.device_type == wgpu::DeviceType::Cpu;

        candidates.push(AdapterCandidate {
            name: info.name.clone(),
            vendor: format!("{:?}", info.device_type),
            tier,
            is_vulkan,
            is_uma,
        });
    }

    candidates.sort_by_key(|c| {
        let vulkan_bonus = if c.is_vulkan { 0u8 } else { 10u8 };
        (c.tier.priority_rank() + vulkan_bonus, c.name.clone())
    });

    candidates
}

pub fn detect_best_adapter() -> Option<AdapterCandidate> {
    let candidates = enumerate_candidates();

    if candidates.is_empty() {
        return None;
    }

    for candidate in &candidates {
        bevy::log::info!(
            "GPU candidate: {} (vendor={}, tier={:?}, vulkan={}, uma={})",
            candidate.name,
            candidate.vendor,
            candidate.tier,
            candidate.is_vulkan,
            candidate.is_uma,
        );
    }

    let best = candidates.into_iter().next()?;
    bevy::log::info!(
        "Selected GPU: {} (tier={:?}, uma={})",
        best.name,
        best.tier,
        best.is_uma,
    );
    Some(best)
}

pub struct GpuSelectionResult {
    pub render_plugin: RenderPlugin,
    pub adapter_info: Option<GpuAdapterInfo>,
}

pub fn render_plugin_with_gpu_selection() -> GpuSelectionResult {
    let mut settings = WgpuSettings {
        power_preference: PowerPreference::HighPerformance,
        ..Default::default()
    };

    let adapter_info = if let Some(best) = detect_best_adapter() {
        let _ = SELECTED_ADAPTER.set(best.clone());
        settings.adapter_name = Some(best.name.clone());

        if best.tier.is_intel() {
            settings.backends = Some(Backends::VULKAN);
            bevy::log::info!(
                "Intel GPU detected ({}), forcing Vulkan backend for optimal UMA performance",
                best.name,
            );
        }

        if best.is_uma {
            bevy::log::info!(
                "UMA adapter detected ({}): readback path will avoid unnecessary copies",
                best.name,
            );
            settings.memory_hints = wgpu::MemoryHints::MemoryUsage;
        }

        Some(GpuAdapterInfo {
            name: best.name,
            vendor: best.vendor,
            tier: best.tier,
            is_vulkan: best.is_vulkan,
            is_uma: best.is_uma,
        })
    } else {
        bevy::log::warn!("No GPU adapter found, falling back to default adapter selection");
        None
    };

    let render_plugin = RenderPlugin {
        render_creation: RenderCreation::Automatic(Box::new(settings)),
        ..Default::default()
    };

    GpuSelectionResult {
        render_plugin,
        adapter_info,
    }
}
