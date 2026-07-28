use sysinfo::{System, Networks};
use serde::Serialize;
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
    let output = crate::hidden_cmd("powershell")
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

fn wmo_description(code: i32) -> String {
    match code {
        0 => "Clear sky".to_string(),
        1 => "Mainly clear".to_string(),
        2 => "Partly cloudy".to_string(),
        3 => "Overcast".to_string(),
        45 | 48 => "Fog".to_string(),
        51 | 53 | 55 => "Drizzle".to_string(),
        56 | 57 => "Freezing drizzle".to_string(),
        61 | 63 | 65 => "Rain".to_string(),
        66 | 67 => "Freezing rain".to_string(),
        71 | 73 | 75 => "Snow".to_string(),
        77 => "Snow grains".to_string(),
        80 | 81 | 82 => "Rain showers".to_string(),
        85 | 86 => "Snow showers".to_string(),
        95 => "Thunderstorm".to_string(),
        96 | 99 => "Thunderstorm with hail".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn wmo_icon(code: i32) -> String {
    match code {
        0 => "sun".to_string(),
        1 | 2 => "cloud-sun".to_string(),
        3 => "cloud".to_string(),
        45 | 48 => "cloud-fog".to_string(),
        51 | 53 | 55 | 56 | 57 => "cloud-drizzle".to_string(),
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => "cloud-rain".to_string(),
        71 | 73 | 75 | 77 | 85 | 86 => "cloud-snow".to_string(),
        95 | 96 | 99 => "cloud-lightning".to_string(),
        _ => "cloud".to_string(),
    }
}

pub async fn get_weather(location: Option<String>) -> Result<WeatherData, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let (lat, lon, display_name) = match &location {
        Some(loc) if !loc.trim().is_empty() => {
            let geo_url = format!(
                "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en",
                urlencoding::encode(loc.trim())
            );
            let geo_resp = client.get(&geo_url).send().await
                .map_err(|e| format!("Geocoding request failed: {}", e))?;
            let geo_body: serde_json::Value = geo_resp.json().await
                .map_err(|e| format!("Failed to parse geocoding: {}", e))?;

            let result = geo_body.get("results")
                .and_then(|r| r.get(0))
                .ok_or_else(|| format!("Location '{}' not found", loc.trim()))?;

            let lat = result.get("latitude").and_then(|v| v.as_f64())
                .ok_or("Missing latitude")?;
            let lon = result.get("longitude").and_then(|v| v.as_f64())
                .ok_or("Missing longitude")?;
            let name = result.get("name").and_then(|v| v.as_str())
                .unwrap_or(loc.trim());
            let country = result.get("country").and_then(|v| v.as_str()).unwrap_or("");
            let display = if country.is_empty() {
                name.to_string()
            } else {
                format!("{}, {}", name, country)
            };
            (lat, lon, display)
        }
        _ => {
            let ip_resp = client.get("https://ipapi.co/json/").send().await
                .map_err(|e| format!("IP geolocation failed: {}", e))?;
            let ip_body: serde_json::Value = ip_resp.json().await
                .map_err(|e| format!("Failed to parse IP location: {}", e))?;
            let lat = ip_body.get("latitude").and_then(|v| v.as_f64()).unwrap_or(23.81);
            let lon = ip_body.get("longitude").and_then(|v| v.as_f64()).unwrap_or(90.41);
            let city = ip_body.get("city").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let country = ip_body.get("country_name").and_then(|v| v.as_str()).unwrap_or("");
            let display = if country.is_empty() {
                city.to_string()
            } else {
                format!("{}, {}", city, country)
            };
            (lat, lon, display)
        }
    };

    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m&timezone=auto",
        lat, lon
    );

    let weather_resp = client.get(&weather_url).send().await
        .map_err(|e| format!("Weather request failed: {}", e))?;
    let weather_body: serde_json::Value = weather_resp.json().await
        .map_err(|e| format!("Failed to parse weather: {}", e))?;

    let current = weather_body.get("current")
        .ok_or("No current weather data")?;

    let temp_c = current.get("temperature_2m").and_then(|v| v.as_f64())
        .map(|t| t as i32).unwrap_or(0);
    let temp_f = (temp_c as f64 * 9.0 / 5.0 + 32.0) as i32;
    let humidity = current.get("relative_humidity_2m").and_then(|v| v.as_f64())
        .map(|h| h as u32).unwrap_or(0);
    let wind_kph = current.get("wind_speed_10m").and_then(|v| v.as_f64())
        .map(|w| w as u32).unwrap_or(0);
    let feels_like_c = current.get("apparent_temperature").and_then(|v| v.as_f64())
        .map(|t| t as i32).unwrap_or(0);
    let weather_code = current.get("weather_code").and_then(|v| v.as_f64())
        .map(|c| c as i32).unwrap_or(0);

    let description = wmo_description(weather_code);
    let icon = wmo_icon(weather_code);

    Ok(WeatherData {
        temp_c,
        temp_f,
        description,
        humidity,
        wind_kph,
        feels_like_c,
        location: display_name,
        icon,
    })
}
