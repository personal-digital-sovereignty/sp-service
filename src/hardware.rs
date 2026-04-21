use ash::{vk, Entry};
use sysinfo::System;
use std::ffi::CStr;
use std::sync::OnceLock;
use tracing::{warn, info};

#[derive(Clone, Debug)]
pub struct HardwareTelemetry {
    pub total_ram_gb: f64,
    pub used_ram_gb: f64,
    pub total_vram_gb: f64,
    pub used_vram_gb: f64, 
    pub gpu_name: String,
    /// True when GPU uses unified memory (Apple Silicon, iGPUs) —
    /// in this case, VRAM metrics mirror system RAM.
    pub unified_memory: bool,
}

/// Static hardware info that never changes (GPU name, total VRAM, driver type)
#[derive(Clone, Debug)]
struct StaticHardwareInfo {
    pub total_vram_bytes: u64,
    pub gpu_name: String,
    pub vram_source: VramSource,
}

/// Determines which mechanism to use for live VRAM readings
#[derive(Clone, Debug)]
#[allow(dead_code)]
enum VramSource {
    /// Linux: /sys/class/drm/cardN/device/mem_info_vram_used (AMD, Intel)
    LinuxSysfs(String), // Stores the discovered path
    /// NVIDIA on any OS: nvidia-smi CLI
    NvidiaSmi,
    /// Apple Silicon / iGPUs: use system RAM as proxy (unified memory)
    UnifiedMemory,
    /// No supported method — report 0
    Unavailable,
}

use std::sync::RwLock;
use std::time::{Instant, Duration};

lazy_static::lazy_static! {
    static ref STATIC_HARDWARE_CACHE: RwLock<(Option<StaticHardwareInfo>, Instant)> = 
        RwLock::new((None, Instant::now() - Duration::from_secs(1000)));
}


struct VkInstanceGuard(ash::Instance);
impl Drop for VkInstanceGuard {
    fn drop(&mut self) {
        unsafe { self.0.destroy_instance(None); }
    }
}

/// Dynamically calculates safe Context Windows based on GPU VRAM availability, with fallback to System RAM.
pub fn calculate_safe_context_window(telemetry: &HardwareTelemetry) -> u64 {
    let governing_memory = if telemetry.total_vram_gb > 0.0 {
        telemetry.total_vram_gb
    } else {
        telemetry.total_ram_gb
    };

    if governing_memory < 8.0 {
        8192 
    } else if governing_memory < 12.0 {
        16384 
    } else if governing_memory < 16.0 {
        32768 
    } else if governing_memory < 24.0 {
        65536 
    } else if governing_memory < 48.0 {
        98304 
    } else {
        131072
    }
}

/// Detect which sysfs path exists for VRAM usage on Linux
#[cfg(target_os = "linux")]
fn detect_sysfs_vram_path() -> Option<String> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path().join("device/mem_info_vram_used");
            if path.exists() {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Detect if nvidia-smi is available (Linux, Windows)
fn detect_nvidia_smi() -> bool {
    std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=memory.used")
        .arg("--format=csv,noheader,nounits")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Determine the best VRAM source for this hardware/OS combination
fn detect_vram_source(gpu_name: &str) -> VramSource {
    // macOS: Apple Silicon uses unified memory
    #[cfg(target_os = "macos")]
    {
        // Apple GPUs and M-series chips use unified memory
        if gpu_name.contains("Apple") || gpu_name == "N/A" {
            return VramSource::UnifiedMemory;
        }
    }

    // Linux: Try sysfs first (AMD/Intel), then nvidia-smi
    #[cfg(target_os = "linux")]
    {
        if let Some(path) = detect_sysfs_vram_path() {
            info!("🖥️ [Hardware] VRAM source: Linux sysfs ({})", path);
            return VramSource::LinuxSysfs(path);
        }
    }

    // All OS: Try nvidia-smi as universal NVIDIA fallback
    if detect_nvidia_smi() {
        info!("🖥️ [Hardware] VRAM source: nvidia-smi CLI");
        return VramSource::NvidiaSmi;
    }

    // macOS without Apple GPU and without NVIDIA — likely Intel iGPU
    #[cfg(target_os = "macos")]
    {
        return VramSource::UnifiedMemory;
    }

    // Windows without NVIDIA — AMD/Intel iGPU, no reliable source
    #[cfg(not(target_os = "macos"))]
    {
        warn!("⚠️ [Hardware] No VRAM monitoring available for GPU: {}", gpu_name);
        VramSource::Unavailable
    }
}

/// Query static GPU info via Vulkan (Recached every 60s via dyn-TTL)
fn get_static_gpu_info() -> StaticHardwareInfo {
    let needs_reload = {
        let guard = STATIC_HARDWARE_CACHE.read().unwrap();
        guard.0.is_none() || guard.1.elapsed() > Duration::from_secs(60)
    };

    if needs_reload {
        let mut total_vram_bytes: u64 = 0;
        let mut gpu_name = "N/A".to_string();

        unsafe {
            if let Ok(entry) = Entry::load() {
                let app_info = vk::ApplicationInfo::default()
                    .api_version(vk::make_api_version(0, 1, 1, 0));
                let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);
                
                if let Ok(instance) = entry.create_instance(&create_info, None) {
                    let _guard = VkInstanceGuard(instance.clone());
                    
                    if let Ok(pdevices) = instance.enumerate_physical_devices() {
                        let mut max_vram: u64 = 0;
                        for &pdevice in pdevices.iter() {
                            let props = instance.get_physical_device_properties(pdevice);
                            let name_cstr = CStr::from_ptr(props.device_name.as_ptr());
                            let current_gpu_name = name_cstr.to_string_lossy().into_owned();
                            
                            let mem_props = instance.get_physical_device_memory_properties(pdevice);
                            let mut current_vram: u64 = 0;
                            for i in 0..mem_props.memory_heap_count {
                                let heap = mem_props.memory_heaps[i as usize];
                                if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) {
                                    current_vram += heap.size;
                                }
                            }
                            
                            if current_vram > max_vram {
                                max_vram = current_vram;
                                gpu_name = current_gpu_name;
                            }
                        }
                        total_vram_bytes = max_vram;
                    }
                } else {
                    warn!("⚠️ [Hardware] Failed to initialize Vulkan Instance.");
                }
            } else {
                warn!("⚠️ [Hardware] Vulkan Loader NOT FOUND on Host.");
            }
        }

        let vram_source = detect_vram_source(&gpu_name);

        info!("🖥️ [Hardware] GPU Telemetry Recached: {}, VRAM: {:.1} GB, Source: {:?}", 
            gpu_name, total_vram_bytes as f64 / 1024.0 / 1024.0 / 1024.0, vram_source);

        let new_info = StaticHardwareInfo { total_vram_bytes, gpu_name, vram_source };
        let mut write_guard = STATIC_HARDWARE_CACHE.write().unwrap();
        *write_guard = (Some(new_info.clone()), Instant::now());
        return new_info;
    }

    STATIC_HARDWARE_CACHE.read().unwrap().0.clone().unwrap()
}

/// Read VRAM usage from Linux sysfs (amdgpu / i915)
fn read_sysfs_vram(path: &str) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Read VRAM usage from nvidia-smi CLI (Linux, Windows, macOS with NVIDIA eGPU)
/// Output format: memory used in MiB (one line per GPU, we take the max)
fn read_nvidia_smi_vram() -> u64 {
    let output = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=memory.used")
        .arg("--format=csv,noheader,nounits")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.trim().parse::<u64>().ok())
                .max()
                .map(|mib| mib * 1024 * 1024) // MiB → bytes
                .unwrap_or(0)
        }
        _ => 0,
    }
}

/// Captures live hardware telemetry. Static GPU data is cached; RAM and VRAM usage are refreshed on each call.
pub fn capture_hardware_telemetry() -> HardwareTelemetry {
    let static_info = get_static_gpu_info();

    // Dynamic: RAM refreshed every call (cheap — no Vulkan)
    let mut sys = System::new_all();
    sys.refresh_memory();
    let total_ram_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let used_ram_gb = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

    // Dynamic: VRAM usage from platform-specific source
    let (used_vram_bytes, unified_memory) = match &static_info.vram_source {
        VramSource::LinuxSysfs(path) => (read_sysfs_vram(path), false),
        VramSource::NvidiaSmi => (read_nvidia_smi_vram(), false),
        VramSource::UnifiedMemory => {
            // Apple Silicon / iGPU: report system memory usage as "VRAM"
            let used_bytes = sys.used_memory();
            (used_bytes, true)
        }
        VramSource::Unavailable => (0, false),
    };

    let used_vram_gb = used_vram_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_vram_gb = if unified_memory {
        total_ram_gb // Apple Silicon: VRAM total = RAM total
    } else {
        static_info.total_vram_bytes as f64 / 1024.0 / 1024.0 / 1024.0
    };

    HardwareTelemetry {
        total_ram_gb,
        used_ram_gb,
        total_vram_gb,
        used_vram_gb,
        gpu_name: static_info.gpu_name,
        unified_memory,
    }
}
