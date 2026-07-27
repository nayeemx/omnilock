use sysinfo::{System, Networks};
use serde::Serialize;
use std::process::Command;

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

static mut PREV_NET_SENT: u64 = 0;
static mut PREV_NET_RECV: u64 = 0;
static mut LAST_NET_TIME: Option<std::time::Instant> = None;

pub fn get_system_stats() -> SystemStats {
    let mut sys = System::new_all();
    sys.refresh_cpu();
    sys.refresh_memory();

    let cpus = sys.cpus();
    let cpu_usage = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };
    let cpu_cores = cpus.len();
    let cpu_name = cpus.first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let ram_total = sys.total_memory() / 1024 / 1024;
    let ram_used = sys.used_memory() / 1024 / 1024;
    let ram_pct = if ram_total > 0 { (ram_used as f32 / ram_total as f32) * 100.0 } else { 0.0 };

    let (gpu_name, gpu_vram, gpu_usage) = get_gpu_info();

    let (sent, recv, sent_rate, recv_rate) = get_network_stats();

    SystemStats {
        cpu_usage,
        cpu_cores,
        cpu_name,
        ram_total_mb: ram_total,
        ram_used_mb: ram_used,
        ram_usage_pct: ram_pct,
        gpu_name,
        gpu_vram_mb: gpu_vram,
        gpu_usage_pct: gpu_usage,
        net_sent_mb: sent,
        net_recv_mb: recv,
        net_sent_rate: sent_rate,
        net_recv_rate: recv_rate,
        uptime_secs: System::uptime(),
    }
}

fn get_gpu_info() -> (String, u64, f32) {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM | ConvertTo-Json"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let trimmed = stdout.trim();

            if trimmed.starts_with('[') {
                if let Ok(arr) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(first) = arr.get(0) {
                        return extract_gpu_from_value(first);
                    }
                }
            } else if trimmed.starts_with('{') {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    return extract_gpu_from_value(&obj);
                }
            }
        }
        Err(_) => {}
    }
    ("Unknown GPU".to_string(), 0, 0.0)
}

fn extract_gpu_from_value(v: &serde_json::Value) -> (String, u64, f32) {
    let name = v.get("Name")
        .and_then(|n| n.as_str())
        .unwrap_or("Unknown GPU")
        .to_string();
    let vram = v.get("AdapterRAM")
        .and_then(|r| r.as_u64())
        .unwrap_or(0) / 1024 / 1024;
    (name, vram, 0.0)
}

fn get_network_stats() -> (f64, f64, f64, f64) {
    let networks = Networks::new_with_refreshed_list();

    let mut total_sent: u64 = 0;
    let mut total_recv: u64 = 0;
    for (_name, data) in &networks {
        total_sent += data.total_transmitted();
        total_recv += data.total_received();
    }

    let sent_mb = total_sent as f64 / 1024.0 / 1024.0;
    let recv_mb = total_recv as f64 / 1024.0 / 1024.0;

    let now = std::time::Instant::now();
    let (sent_rate, recv_rate) = unsafe {
        if let Some(last) = LAST_NET_TIME {
            let elapsed = now.duration_since(last).as_secs_f64();
            if elapsed > 0.0 {
                let ds = total_sent.saturating_sub(PREV_NET_SENT);
                let dr = total_recv.saturating_sub(PREV_NET_RECV);
                PREV_NET_SENT = total_sent;
                PREV_NET_RECV = total_recv;
                LAST_NET_TIME = Some(now);
                (ds as f64 / elapsed / 1024.0, dr as f64 / elapsed / 1024.0)
            } else {
                (0.0, 0.0)
            }
        } else {
            PREV_NET_SENT = total_sent;
            PREV_NET_RECV = total_recv;
            LAST_NET_TIME = Some(now);
            (0.0, 0.0)
        }
    };

    (sent_mb, recv_mb, sent_rate, recv_rate)
}

pub async fn get_weather() -> Result<WeatherData, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://wttr.in/?format=j1")
        .timeout(std::time::Duration::from_secs(5))
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
