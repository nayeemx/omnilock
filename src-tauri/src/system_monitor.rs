use sysinfo::{System, Networks};
use serde::Serialize;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

#[derive(Serialize, Clone)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub cpu_name: String,
    pub ram_total_mb: u64,
    pub ram_used_mb: u64,
    pub ram_usage_pct: f32,
    pub gpu_name: String,
    pub gpu_vram_mb: u64,
    pub gpu_usage_pct: f32,
    pub net_sent_mb: f64,
    pub net_recv_mb: f64,
    pub net_sent_rate: f64,
    pub net_recv_rate: f64,
    pub uptime_secs: u64,
}

#[derive(Serialize, Clone)]
pub struct WeatherData {
    pub temp_c: i32,
    pub temp_f: i32,
    pub description: String,
    pub humidity: u32,
    pub wind_kph: u32,
    pub feels_like_c: i32,
    pub location: String,
    pub icon: String,
}

struct CachedGpu {
    name: String,
    vram_mb: u64,
}

struct MonitorState {
    sys: System,
    networks: Networks,
    prev_net_sent: u64,
    prev_net_recv: u64,
    last_net_time: Option<Instant>,
}

static GPU_CACHE: OnceLock<CachedGpu> = OnceLock::new();
static MONITOR_STATE: OnceLock<Arc<Mutex<MonitorState>>> = OnceLock::new();

fn get_state() -> &'static Arc<Mutex<MonitorState>> {
    MONITOR_STATE.get_or_init(|| {
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();
        let networks = Networks::new_with_refreshed_list();
        Arc::new(Mutex::new(MonitorState {
            sys,
            networks,
            prev_net_sent: 0,
            prev_net_recv: 0,
            last_net_time: None,
        }))
    })
}

pub async fn get_system_stats_async() -> SystemStats {
    let state = get_state().clone();
    tokio::task::spawn_blocking(move || {
        let mut state = state.lock().unwrap();

        state.sys.refresh_cpu();
        state.sys.refresh_memory();

        let cpus = state.sys.cpus();
        let cpu_usage = if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
        };
        let cpu_cores = cpus.len();
        let cpu_name = cpus.first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let ram_total = state.sys.total_memory() / 1024 / 1024;
        let ram_used = state.sys.used_memory() / 1024 / 1024;
        let ram_pct = if ram_total > 0 { (ram_used as f32 / ram_total as f32) * 100.0 } else { 0.0 };

        let gpu = GPU_CACHE.get_or_init(|| query_gpu_once());

        state.networks.refresh();
        let mut total_sent: u64 = 0;
        let mut total_recv: u64 = 0;
        for (_name, data) in &state.networks {
            total_sent += data.total_transmitted();
            total_recv += data.total_received();
        }

        let sent_mb = total_sent as f64 / 1024.0 / 1024.0;
        let recv_mb = total_recv as f64 / 1024.0 / 1024.0;

        let now = Instant::now();
        let (sent_rate, recv_rate) = if let Some(last) = state.last_net_time {
            let elapsed = now.duration_since(last).as_secs_f64();
            if elapsed > 0.0 {
                let ds = total_sent.saturating_sub(state.prev_net_sent);
                let dr = total_recv.saturating_sub(state.prev_net_recv);
                state.prev_net_sent = total_sent;
                state.prev_net_recv = total_recv;
                state.last_net_time = Some(now);
                (ds as f64 / elapsed / 1024.0, dr as f64 / elapsed / 1024.0)
            } else {
                (0.0, 0.0)
            }
        } else {
            state.prev_net_sent = total_sent;
            state.prev_net_recv = total_recv;
            state.last_net_time = Some(now);
            (0.0, 0.0)
        };

        SystemStats {
            cpu_usage,
            cpu_cores,
            cpu_name,
            ram_total_mb: ram_total,
            ram_used_mb: ram_used,
            ram_usage_pct: ram_pct,
            gpu_name: gpu.name.clone(),
            gpu_vram_mb: gpu.vram_mb,
            gpu_usage_pct: 0.0,
            net_sent_mb: sent_mb,
            net_recv_mb: recv_mb,
            net_sent_rate: sent_rate,
            net_recv_rate: recv_rate,
            uptime_secs: System::uptime(),
        }
    })
    .await
    .unwrap_or_else(|e| SystemStats {
        cpu_usage: 0.0,
        cpu_cores: 0,
        cpu_name: format!("Error: {}", e),
        ram_total_mb: 0,
        ram_used_mb: 0,
        ram_usage_pct: 0.0,
        gpu_name: "Unknown".to_string(),
        gpu_vram_mb: 0,
        gpu_usage_pct: 0.0,
        net_sent_mb: 0.0,
        net_recv_mb: 0.0,
        net_sent_rate: 0.0,
        net_recv_rate: 0.0,
        uptime_secs: 0,
    })
}

fn query_gpu_once() -> CachedGpu {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM | ConvertTo-Json"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let trimmed = stdout.trim();

        let val = if trimmed.starts_with('[') {
            serde_json::from_str::<serde_json::Value>(trimmed)
                .ok()
                .and_then(|arr| arr.get(0).cloned())
        } else if trimmed.starts_with('{') {
            serde_json::from_str::<serde_json::Value>(trimmed).ok()
        } else {
            None
        };

        if let Some(v) = val {
            let name = v.get("Name")
                .and_then(|n| n.as_str())
                .unwrap_or("Unknown GPU")
                .to_string();
            let vram = v.get("AdapterRAM")
                .and_then(|r| r.as_u64())
                .unwrap_or(0) / 1024 / 1024;
            return CachedGpu { name, vram_mb: vram };
        }
    }

    CachedGpu { name: "Unknown GPU".to_string(), vram_mb: 0 }
}

pub async fn get_weather() -> Result<WeatherData, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get("https://wttr.in/?format=j1")
        .send()
        .await
        .map_err(|e| format!("Weather request failed: {}", e))?;

    let body: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse weather: {}", e))?;

    let current = body.get("current_condition")
        .and_then(|c| c.get(0))
        .ok_or("No current condition data")?;

    let temp_c = current.get("temp_C").and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    let temp_f = current.get("temp_F").and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    let humidity = current.get("humidity").and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    let wind_kph = current.get("windspeedKmph").and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    let feels_like_c = current.get("FeelsLikeC").and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok()).unwrap_or(0);

    let desc_arr = current.get("weatherDesc")
        .and_then(|d| d.get(0));
    let description = desc_arr
        .and_then(|d| d.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let location = body.get("nearest_area")
        .and_then(|a| a.get(0))
        .and_then(|a| a.get("areaName"))
        .and_then(|a| a.get(0))
        .and_then(|a| a.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let icon = match temp_c {
        t if t > 30 => "sun".to_string(),
        t if t > 20 => "cloud-sun".to_string(),
        t if t > 10 => "cloud".to_string(),
        t if t > 0 => "cloud-drizzle".to_string(),
        _ => "snowflake".to_string(),
    };

    Ok(WeatherData {
        temp_c,
        temp_f,
        description,
        humidity,
        wind_kph,
        feels_like_c,
        location,
        icon,
    })
}
