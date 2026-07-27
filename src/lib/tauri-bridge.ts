import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface VaultStatusDto {
  initialized: boolean;
  totp_enabled: boolean;
  publisher: string;
  version: string;
  github_connected: boolean;
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
  usb_key_enabled: boolean;
  usb_key_drive_label: string;
  cloud_sync_enabled: boolean;
  github_username: string;
}

export interface WatchdogStatusDto {
  pid: number;
  uptime_secs: number;
  process_count: number;
  status: string;
}

export interface SystemInfoDto {
  os: string;
  arch: string;
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
}): Promise<string> {
  return invoke("cmd_unlock_session", { authPayload });
}

export async function toggleSystemPreset(presetId: string, enabled: boolean): Promise<void> {
  return invoke("cmd_toggle_system_preset", { presetId, enabled });
}

export async function toggleInstallerGuard(enabled: boolean): Promise<void> {
  return invoke("cmd_toggle_installer_guard", { enabled });
}

export async function lockNow(): Promise<void> {
  return invoke("cmd_lock_now");
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

export async function getRecoveryKey(): Promise<string> {
  return invoke("cmd_get_recovery_key");
}

export async function recoverWithKey(newPassword: string, recoveryKey: string): Promise<void> {
  return invoke("cmd_recover_with_key", { newPassword, recoveryKey });
}

export interface UsbDriveInfo {
  letter: string;
  label: string;
  serial: number;
}

export async function listUsbDrives(): Promise<UsbDriveInfo[]> {
  return invoke("cmd_list_usb_drives");
}

export async function enrollUsbKey(driveLetter: string): Promise<void> {
  return invoke("cmd_enroll_usb_key", { driveLetter });
}

export async function removeUsbKey(): Promise<void> {
  return invoke("cmd_remove_usb_key");
}

export async function detectUsbKey(): Promise<string | null> {
  return invoke("cmd_detect_usb_key");
}

export async function recoverWithUsbKey(newPassword: string): Promise<void> {
  return invoke("cmd_recover_with_usb_key", { newPassword });
}

export async function getWatchdogStatus(): Promise<WatchdogStatusDto> {
  return invoke("cmd_get_watchdog_status");
}

export async function getSystemInfo(): Promise<SystemInfoDto> {
  return invoke("cmd_get_system_info");
}

export async function showWidget(targetType: string, targetId: string, displayName: string): Promise<void> {
  return invoke("cmd_show_widget", { targetType, targetId, displayName });
}

export async function hideWidget(): Promise<void> {
  return invoke("cmd_hide_widget");
}

export async function widgetUnlock(password: string): Promise<void> {
  return invoke("cmd_widget_unlock", { password });
}

export async function widgetListLocked(): Promise<{ target_type: string; target_id: string; display_name: string }[]> {
  return invoke("cmd_widget_list_locked");
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

export async function installUpdate(): Promise<{ success: boolean; error?: string }> {
  try {
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
      relaunch();
      return { success: true };
    }
    return { success: false, error: "No update available" };
  } catch (e: any) {
    console.error("Install failed:", e);
    return { success: false, error: String(e) };
  }
}

export interface GitHubSyncStatusDto {
  connected: boolean;
  github_user: string | null;
  avatar_url: string | null;
  last_sync: number | null;
  device_id: string;
}

export interface GitHubDeviceFlowDto {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export async function githubStartDeviceFlow(): Promise<GitHubDeviceFlowDto> {
  return invoke("cmd_github_start_device_flow");
}

export async function githubPollToken(
  deviceCode: string,
  interval: number,
  expiresIn: number
): Promise<GitHubSyncStatusDto> {
  return invoke("cmd_github_poll_token", { deviceCode, interval, expiresIn });
}

export async function githubGetStatus(): Promise<GitHubSyncStatusDto> {
  return invoke("cmd_github_get_status");
}

export async function githubConnectToken(token: string): Promise<GitHubSyncStatusDto> {
  return invoke("cmd_github_connect_token", { token });
}

export async function githubDisconnect(): Promise<void> {
  return invoke("cmd_github_disconnect");
}

export async function githubSyncToCloud(): Promise<GitHubSyncStatusDto> {
  return invoke("cmd_github_sync_to_cloud");
}

export async function githubSyncFromCloud(): Promise<void> {
  return invoke("cmd_github_sync_from_cloud");
}

export async function openExternalUrl(url: string): Promise<void> {
  return invoke("cmd_open_external_url", { url });
}
