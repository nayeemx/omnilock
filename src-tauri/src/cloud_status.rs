#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Storage::Provider::{
    StorageProviderItemProperties, StorageProviderItemProperty,
    StorageProviderItemPropertyDefinition, StorageProviderSyncRootInfo,
    StorageProviderSyncRootManager,
};
use windows::Storage::{StorageFile, StorageFolder};
use windows::core::HSTRING;

use crate::logger;

static CLOUD_STATUS_ACTIVE: AtomicBool = AtomicBool::new(false);
static SYNC_ROOT_ID: Mutex<Option<String>> = Mutex::new(None);

const LOCK_STATE_PROPERTY_ID: i32 = 1;

pub fn is_available() -> bool {
    CLOUD_STATUS_ACTIVE.load(Ordering::SeqCst)
}

pub fn try_init(vault_root_path: &str) -> bool {
    let path = Path::new(vault_root_path);
    if !path.exists() {
        logger::log("CLOUD", &format!("Vault root does not exist: {}", vault_root_path));
        return false;
    }

    if !path.is_dir() {
        logger::log("CLOUD", &format!("Vault root is not a directory: {}", vault_root_path));
        return false;
    }

    match register_sync_root(vault_root_path) {
        Ok(()) => {
            logger::log("CLOUD", &format!("Sync root registered for: {}", vault_root_path));
            CLOUD_STATUS_ACTIVE.store(true, Ordering::SeqCst);
            true
        }
        Err(e) => {
            logger::log("CLOUD", &format!("Sync root registration failed: {} (falling back to companion files)", e));
            register_sync_root_registry(vault_root_path);
            CLOUD_STATUS_ACTIVE.store(true, Ordering::SeqCst);
            true
        }
    }
}

pub fn set_lock_state(path: &str, locked: bool) {
    if !CLOUD_STATUS_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    let result = set_item_property(path, locked);
    match &result {
        Ok(()) => logger::log("CLOUD", &format!("set_lock_state {} locked={}", path, locked)),
        Err(e) => logger::log("CLOUD", &format!("set_lock_state failed for {}: {}", path, e)),
    }

    write_companion_status(path, locked).ok();
    result.ok();
}

pub fn shutdown() {
    if !CLOUD_STATUS_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    CLOUD_STATUS_ACTIVE.store(false, Ordering::SeqCst);

    if let Ok(guard) = SYNC_ROOT_ID.lock() {
        if let Some(ref id) = *guard {
            let id_hstring = HSTRING::from(id);
            let _ = StorageProviderSyncRootManager::Unregister(&id_hstring);
            logger::log("CLOUD", &format!("Sync root unregistered: {}", id));
        }
    }

    let reg_key = format!(r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager\OmniLock");
    let _ = std::process::Command::new("reg")
        .args(["delete", &reg_key, "/f"])
        .output();
    logger::log("CLOUD", "Cloud status module shut down");
}

fn register_sync_root(vault_root: &str) -> Result<(), String> {
    let id = format!("OmniLock_{}", uuid::Uuid::new_v4());
    let info = StorageProviderSyncRootInfo::new().map_err(|e| format!("Failed to create SyncRootInfo: {}", e))?;

    let id_hstring = HSTRING::from(&id);
    info.SetId(&id_hstring).map_err(|e| format!("Failed to set Id: {}", e))?;

    let path_str = vault_root.trim_end_matches('\\');
    let folder = StorageFolder::GetFolderFromPathAsync(&HSTRING::from(path_str))
        .and_then(|op| op.get())
        .map_err(|e| format!("Failed to get StorageFolder for path: {}", e))?;
    info.SetPath(&folder).map_err(|e| format!("Failed to set Path: {}", e))?;

    let display_name = HSTRING::from("OmniLock");
    info.SetDisplayNameResource(&display_name).map_err(|e| format!("Failed to set DisplayName: {}", e))?;

    let icon_resource = HSTRING::from("%SystemRoot%\\system32\\imageres.dll,204");
    info.SetIconResource(&icon_resource).map_err(|e| format!("Failed to set IconResource: {}", e))?;

    let prop_def = StorageProviderItemPropertyDefinition::new()
        .map_err(|e| format!("Failed to create property def: {}", e))?;
    prop_def.SetId(LOCK_STATE_PROPERTY_ID)
        .map_err(|e| format!("Failed to set property def id: {}", e))?;
    let prop_name = HSTRING::from("Lock Status");
    prop_def.SetDisplayNameResource(&prop_name)
        .map_err(|e| format!("Failed to set property def name: {}", e))?;

    if let Ok(defs) = info.StorageProviderItemPropertyDefinitions() {
        let _ = defs.Append(&prop_def);
    }

    StorageProviderSyncRootManager::Register(&info)
        .map_err(|e| format!("Register failed: {}", e))?;

    if let Ok(mut guard) = SYNC_ROOT_ID.lock() {
        *guard = Some(id);
    }

    Ok(())
}

fn register_sync_root_registry(vault_root: &str) {
    let reg_base = r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager\OmniLock";
    let _ = std::process::Command::new("reg")
        .args(["add", reg_base, "/ve", "/d", "OmniLock", "/f"])
        .output();

    let id_path = format!(r"{}\SyncRootId", reg_base);
    let _ = std::process::Command::new("reg")
        .args(["add", &id_path, "/ve", "/d", "OmniLock", "/f"])
        .output();

    let user_sync_roots = format!(r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\UserSyncRoots\OmniLock");
    let _ = std::process::Command::new("reg")
        .args(["add", &user_sync_roots, "/v", "IsSyncRoot", "/t", "REG_DWORD", "/d", "1", "/f"])
        .output();

    let _ = std::process::Command::new("reg")
        .args(["add", reg_base, "/v", "Path", "/d", vault_root, "/f"])
        .output();

    let _ = std::process::Command::new("reg")
        .args(["add", reg_base, "/v", "DisplayName", "/d", "OmniLock", "/f"])
        .output();

    SyncRootId::store("OmniLock");
    logger::log("CLOUD", "Sync root registered via registry fallback");
}

struct SyncRootId;
impl SyncRootId {
    fn store(id: &str) {
        if let Ok(mut guard) = SYNC_ROOT_ID.lock() {
            *guard = Some(id.to_string());
        }
    }
}

fn icon_path(locked: bool) -> String {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let name = if locked { "lock_256.png" } else { "unlock_256.png" };
    dir.join("icons").join(name).to_string_lossy().to_string()
}

fn set_item_property(path: &str, locked: bool) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("Path does not exist".to_string());
    }

    let prop = StorageProviderItemProperty::new()
        .map_err(|e| format!("Failed to create property: {}", e))?;
    prop.SetId(LOCK_STATE_PROPERTY_ID)
        .map_err(|e| format!("Failed to set property id: {}", e))?;

    let value_hstring = HSTRING::from(if locked { "Locked" } else { "Unlocked" });
    prop.SetValue(&value_hstring)
        .map_err(|e| format!("Failed to set property value: {}", e))?;

    let icon_path_str = icon_path(locked);
    let icon_hstring = HSTRING::from(&icon_path_str);
    prop.SetIconResource(&icon_hstring)
        .map_err(|e| format!("Failed to set property icon: {}", e))?;

    let iterable = windows_collections::IIterable::from(vec![Some(prop)]);

    if p.is_dir() {
        let path_str = path.trim_end_matches('\\');
        let folder = StorageFolder::GetFolderFromPathAsync(&HSTRING::from(path_str))
            .and_then(|op| op.get())
            .map_err(|e| format!("Failed to get folder: {}", e))?;
        let _ = StorageProviderItemProperties::SetAsync(&folder, &iterable);
    } else {
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path))
            .and_then(|op| op.get())
            .map_err(|e| format!("Failed to get file: {}", e))?;
        let _ = StorageProviderItemProperties::SetAsync(&file, &iterable);
    }

    Ok(())
}

fn write_companion_status(path: &str, locked: bool) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Ok(());
    }

    let parent = if p.is_dir() {
        p.to_path_buf()
    } else {
        p.parent().map(|x| x.to_path_buf()).unwrap_or_else(|| p.to_path_buf())
    };

    let status_dir = parent.join(".omnilock");
    std::fs::create_dir_all(&status_dir).map_err(|e| format!("Cannot create status dir: {}", e))?;

    let file_name = p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "root".to_string());

    let status_path = status_dir.join(format!("{}.json", file_name));
    let status = serde_json::json!({
        "locked": locked,
        "path": path,
        "updated_at": std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });

    std::fs::write(&status_path, serde_json::to_string_pretty(&status).unwrap())
        .map_err(|e| format!("Cannot write status file: {}", e))?;

    Ok(())
}
