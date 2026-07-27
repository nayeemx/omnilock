use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::Cryptography::*;

const GITHUB_CLIENT_ID: &str = "Ov23liplaceholder";
const GITHUB_DEVICE_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_API: &str = "https://api.github.com";
const GIST_DESCRIPTION: &str = "OmniLock Vault Backup";
const VAULT_FILENAME: &str = "vault.enc";
const META_FILENAME: &str = "vault.meta.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GistInfo {
    pub id: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMeta {
    pub github_user: String,
    pub github_user_id: u64,
    pub avatar_url: String,
    pub gist_id: String,
    pub last_sync: u64,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub connected: bool,
    pub github_user: Option<String>,
    pub avatar_url: Option<String>,
    pub last_sync: Option<u64>,
    pub device_id: String,
}

fn sync_meta_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(appdata).join("InnologyBD\\OmniLock");
    fs::create_dir_all(&dir).ok();
    dir.join("sync.meta.json")
}

fn ensure_device_id() -> String {
    let meta_path = sync_meta_path().parent().unwrap().join("device_id");
    if let Ok(id) = fs::read_to_string(&meta_path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _ = fs::write(&meta_path, &id);
    id
}

pub fn get_sync_status() -> SyncStatus {
    let meta_path = sync_meta_path();
    let device_id = ensure_device_id();
    if let Ok(data) = fs::read_to_string(&meta_path) {
        if let Ok(meta) = serde_json::from_str::<SyncMeta>(&data) {
            return SyncStatus {
                connected: true,
                github_user: Some(meta.github_user),
                avatar_url: Some(meta.avatar_url),
                last_sync: Some(meta.last_sync),
                device_id,
            };
        }
    }
    SyncStatus {
        connected: false,
        github_user: None,
        avatar_url: None,
        last_sync: None,
        device_id,
    }
}

fn load_sync_meta() -> Option<SyncMeta> {
    let data = fs::read_to_string(sync_meta_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_sync_meta(meta: &SyncMeta) -> Result<(), String> {
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(sync_meta_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_github_token() -> Result<String, String> {
    let _meta = load_sync_meta().ok_or("GitHub not connected")?;
    let token_path = sync_meta_path().parent().unwrap().join("github.token");
    let data = fs::read(&token_path).map_err(|_| "GitHub token not found")?;
    if data.is_empty() {
        return Err("GitHub token is empty".to_string());
    }
    // Try DPAPI decrypt first (new format)
    if let Ok(token) = dpapi_decrypt(&data) {
        return Ok(token);
    }
    // Fallback: try as plaintext (legacy format), then re-encrypt
    let token = String::from_utf8(data).map_err(|e| format!("Invalid token data: {}", e))?;
    let token = token.trim().to_string();
    if !token.is_empty() {
        let _ = save_github_token(&token);
    }
    Ok(token)
}

fn save_github_token(token: &str) -> Result<(), String> {
    let token_path = sync_meta_path().parent().unwrap().join("github.token");
    let encrypted = dpapi_encrypt(token.as_bytes()).map_err(|e| format!("Failed to encrypt token: {}", e))?;
    fs::write(token_path, encrypted).map_err(|e| e.to_string())?;
    Ok(())
}

fn dpapi_encrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output: CRYPT_INTEGER_BLOB = std::mem::zeroed();
        let ok = CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        );
        if ok == 0 {
            return Err(format!("CryptProtectData failed: {}", std::io::Error::last_os_error()));
        }
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as *mut _);
        Ok(result)
    }
}

fn dpapi_decrypt(data: &[u8]) -> Result<String, String> {
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output: CRYPT_INTEGER_BLOB = std::mem::zeroed();
        let ok = CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        );
        if ok == 0 {
            return Err(format!("CryptUnprotectData failed: {}", std::io::Error::last_os_error()));
        }
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as *mut _);
        String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

fn save_github_user(user: &GitHubUser) -> Result<(), String> {
    let path = sync_meta_path().parent().unwrap().join("github.user.json");
    let json = serde_json::to_string_pretty(user).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_github_user() -> Option<GitHubUser> {
    let path = sync_meta_path().parent().unwrap().join("github.user.json");
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub async fn start_device_flow() -> Result<DeviceCodeResponse, String> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", GITHUB_CLIENT_ID),
        ("scope", "gist"),
    ];
    let resp = client
        .post(GITHUB_DEVICE_URL)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to start device flow: {}", e))?;

    let body: DeviceCodeResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse device flow response: {}", e))?;
    Ok(body)
}

pub async fn poll_for_token(
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    loop {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - start;
        if elapsed > expires_in {
            return Err("Device code expired. Please try again.".to_string());
        }

        let params = [
            ("client_id", GITHUB_CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let resp = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token poll failed: {}", e))?;

        let body: OAuthTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        if let Some(token) = body.access_token {
            save_github_token(&token)?;
            let user = fetch_github_user(&token).await?;
            save_github_user(&user)?;
            return Ok(token);
        }

        match body.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => {
                tokio::time::sleep(Duration::from_secs(interval + 5)).await;
                continue;
            }
            Some("expired_token") => {
                return Err("Device code expired. Please try again.".to_string());
            }
            Some("access_denied") => {
                return Err("Authorization denied by user.".to_string());
            }
            Some(e) => {
                return Err(format!("OAuth error: {}", e));
            }
            None => {
                return Err("Unexpected response from GitHub".to_string());
            }
        }

        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

async fn fetch_github_user(token: &str) -> Result<GitHubUser, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/user", GITHUB_API))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "OmniLock/0.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GitHub user: {}", e))?;

    resp.json()
        .await
        .map_err(|e| format!("Failed to parse GitHub user: {}", e))
}

pub async fn create_or_find_gist(token: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/gists?per_page=100", GITHUB_API))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "OmniLock/0.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to list gists: {}", e))?;

    let gists: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse gists: {}", e))?;

    for gist in &gists {
        if let Some(desc) = gist["description"].as_str() {
            if desc == GIST_DESCRIPTION {
                return Ok(gist["id"].as_str().unwrap_or("").to_string());
            }
        }
    }

    let body = serde_json::json!({
        "description": GIST_DESCRIPTION,
        "public": false,
        "files": {
            VAULT_FILENAME: { "content": "" },
            META_FILENAME: { "content": "{}" }
        }
    });

    let resp = client
        .post(format!("{}/gists", GITHUB_API))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "OmniLock/0.1.0")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to create gist: {}", e))?;

    let gist: GistInfo = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse created gist: {}", e))?;

    Ok(gist.id)
}

pub async fn upload_vault(
    token: &str,
    gist_id: &str,
    vault_data: &[u8],
    meta: &SyncMeta,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let vault_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        vault_data,
    );
    let meta_json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "files": {
            VAULT_FILENAME: { "content": vault_b64 },
            META_FILENAME: { "content": meta_json }
        }
    });

    let resp = client
        .patch(format!("{}/gists/{}", GITHUB_API, gist_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "OmniLock/0.1.0")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to upload vault: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Upload failed ({}): {}", status, text))
    }
}

pub async fn download_vault(
    token: &str,
    gist_id: &str,
) -> Result<(Vec<u8>, SyncMeta), String> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/gists/{}", GITHUB_API, gist_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "OmniLock/0.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to download gist: {}", e))?;

    let gist: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse gist: {}", e))?;

    let vault_content = gist["files"][VAULT_FILENAME]["content"]
        .as_str()
        .unwrap_or("");
    let meta_content = gist["files"][META_FILENAME]["content"]
        .as_str()
        .unwrap_or("{}");

    let vault_data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        vault_content,
    )
    .map_err(|e| format!("Failed to decode vault data: {}", e))?;

    let meta: SyncMeta = serde_json::from_str(meta_content)
        .map_err(|e| format!("Failed to parse sync meta: {}", e))?;

    Ok((vault_data, meta))
}

pub async fn sync_to_cloud(
    vault_data: &[u8],
) -> Result<SyncStatus, String> {
    let token = load_github_token()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let user = load_github_user().ok_or("GitHub user not found")?;
    let device_id = ensure_device_id();

    let gist_id = if let Some(meta) = load_sync_meta() {
        meta.gist_id
    } else {
        create_or_find_gist(&token).await?
    };

    let meta = SyncMeta {
        github_user: user.login.clone(),
        github_user_id: user.id,
        avatar_url: user.avatar_url.clone().unwrap_or_default(),
        gist_id: gist_id.clone(),
        last_sync: now,
        device_id,
    };

    upload_vault(&token, &gist_id, vault_data, &meta).await?;
    save_sync_meta(&meta)?;

    Ok(SyncStatus {
        connected: true,
        github_user: Some(user.login),
        avatar_url: user.avatar_url,
        last_sync: Some(now),
        device_id: meta.device_id,
    })
}

pub async fn sync_from_cloud() -> Result<Vec<u8>, String> {
    let token = load_github_token()?;
    let meta = load_sync_meta().ok_or("GitHub not connected")?;

    let (vault_data, _remote_meta) = download_vault(&token, &meta.gist_id).await?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut updated_meta = meta;
    updated_meta.last_sync = now;
    save_sync_meta(&updated_meta)?;

    Ok(vault_data)
}

pub fn disconnect_github() -> Result<(), String> {
    let meta_path = sync_meta_path();
    let dir = meta_path.parent().unwrap();
    let _ = fs::remove_file(dir.join("sync.meta.json"));
    let _ = fs::remove_file(dir.join("github.token"));
    let _ = fs::remove_file(dir.join("github.user.json"));
    Ok(())
}

pub async fn verify_github_token(token: &str) -> Result<GitHubUser, String> {
    fetch_github_user(token).await
}

pub fn connect_with_token(token: String, user: GitHubUser) -> Result<SyncStatus, String> {
    save_github_token(&token)?;
    save_github_user(&user)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let meta = SyncMeta {
        github_user: user.login.clone(),
        github_user_id: user.id,
        avatar_url: user.avatar_url.clone().unwrap_or_default(),
        gist_id: String::new(),
        last_sync: now,
        device_id: ensure_device_id(),
    };
    save_sync_meta(&meta)?;

    Ok(SyncStatus {
        connected: true,
        github_user: Some(user.login),
        avatar_url: user.avatar_url,
        last_sync: None,
        device_id: meta.device_id,
    })
}
