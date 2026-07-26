import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface VaultStatusDto {
  initialized: boolean;
  totp_enabled: boolean;
  publisher: string;
  version: string;
}

export interface VaultConfigDto {
  locked_apps: { name: string; path: string; sha256: string; enabled: boolean }[];
  system_presets: {
    task_manager: boolean;
    control_panel: boolean;
    registry_editor: boolean;
    powershell: boolean;
    cmd: boolean;
    system_restore: boolean;
  };
  installer_guard_enabled: boolean;
  locked_files: string[];
  locked_folders: string[];
  locked_drives: string[];
  auto_lock_minutes: number;
  totp_enabled: boolean;
  recovery_key: string;
  security_question: string;
}

export async function getVaultStatus(): Promise<VaultStatusDto> {
  return invoke("cmd_get_vault_status");
}

export async function getVaultConfig(): Promise<VaultConfigDto> {
  return invoke("cmd_get_vault_config");
}

export async function setupVault(payload: {
  master_password: string;
  security_question: string;
  security_answer: string;
  totp_secret: string;
}): Promise<void> {
  return invoke("cmd_setup_vault", { payload });
}

export async function unlockSession(authPayload: {
  master_password: string;
  totp_code: string;
}): Promise<void> {
  return invoke("cmd_unlock_session", { authPayload });
}

export async function toggleSystemPreset(presetId: string, enabled: boolean): Promise<void> {
  return invoke("cmd_toggle_system_preset", { presetId, enabled });
}

export async function toggleInstallerGuard(enabled: boolean): Promise<void> {
  return invoke("cmd_toggle_installer_guard", { enabled });
}

export async function triggerPanicLock(): Promise<void> {
  return invoke("cmd_trigger_panic_lock");
}

export async function addLockedDrive(driveLetter: string): Promise<void> {
  return invoke("cmd_add_locked_drive", { driveLetter });
}

export async function removeLockedDrive(driveLetter: string): Promise<void> {
  return invoke("cmd_remove_locked_drive", { driveLetter });
}

export async function addLockedFile(path: string): Promise<void> {
  return invoke("cmd_add_locked_file", { path });
}

export async function removeLockedFile(path: string): Promise<void> {
  return invoke("cmd_remove_locked_file", { path });
}

export async function addLockedFolder(path: string): Promise<void> {
  return invoke("cmd_add_locked_folder", { path });
}

export async function removeLockedFolder(path: string): Promise<void> {
  return invoke("cmd_remove_locked_folder", { path });
}

export async function toggleLockedApp(name: string, enabled: boolean): Promise<void> {
  return invoke("cmd_toggle_locked_app", { name, enabled });
}

export async function addLockedApp(name: string, path: string, sha256: string): Promise<void> {
  return invoke("cmd_add_locked_app", { name, path, sha256 });
}

export async function removeLockedApp(name: string): Promise<void> {
  return invoke("cmd_remove_locked_app", { name });
}

export async function generateTotpSecret(): Promise<string> {
  return invoke("cmd_generate_totp");
}

export async function generateTotpQr(secret: string): Promise<string> {
  return invoke("cmd_generate_totp_qr", { secret });
}

export async function enable2FA(secret: string, code: string): Promise<void> {
  return invoke("cmd_enable_2fa", { secret, code });
}

export async function disable2FA(): Promise<void> {
  return invoke("cmd_disable_2fa");
}

export async function listDrives(): Promise<string[]> {
  return invoke("cmd_list_drives");
}

export async function listProcesses(): Promise<[string, string, string][]> {
  return invoke("cmd_list_processes");
}

export async function setAutoLock(minutes: number): Promise<void> {
  return invoke("cmd_set_auto_lock", { minutes });
}

export async function getSecurityQuestion(): Promise<string> {
  return invoke("cmd_get_security_question");
}

export async function resetPassword(newPassword: string, answer: string): Promise<void> {
  return invoke("cmd_reset_password", { newPassword, answer });
}

export async function checkForUpdates(): Promise<{ available: boolean; version?: string; notes?: string } | null> {
  try {
    const update = await check();
    if (update) {
      return {
        available: true,
        version: update.version,
        notes: update.body,
      };
    }
    return { available: false };
  } catch (e) {
    console.error("Update check failed:", e);
    return null;
  }
}

export async function installUpdate(): Promise<void> {
  const update = await check();
  if (update) {
    let downloaded = 0;
    let contentLength = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength = event.data.contentLength || 0;
          console.log(`Downloading ${contentLength} bytes...`);
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          console.log(`Downloaded ${downloaded}/${contentLength} bytes`);
          break;
        case "Finished":
          console.log("Download complete, installing...");
          break;
      }
    });
    await relaunch();
  }
}
