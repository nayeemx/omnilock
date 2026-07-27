import {
  LayoutGrid, SlidersHorizontal, FolderLock, KeyRound,
  Monitor, Settings2, Database, Terminal, RefreshCw,
  LayoutDashboard, Activity,
} from "lucide-react";

export type TabId = "dashboard" | "monitor" | "apps" | "presets" | "vault" | "security";
export type SetupStep = 1 | 2;

export const tabs: { id: TabId; label: string; icon: React.ElementType }[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "monitor", label: "System Monitor", icon: Activity },
  { id: "apps", label: "Application Locker", icon: LayoutGrid },
  { id: "presets", label: "System Presets", icon: SlidersHorizontal },
  { id: "vault", label: "File & Drive Vault", icon: FolderLock },
  { id: "security", label: "Security & 2FA", icon: KeyRound },
];

export const securityQuestions = [
  "What is your mother's maiden name?",
  "What was the name of your first pet?",
  "What city were you born in?",
  "What is the name of your favorite teacher?",
  "What was your childhood nickname?",
];

export const presetMeta: Record<string, { label: string; desc: string; icon: React.ElementType }> = {
  task_manager: { label: "Task Manager", desc: "Prevent Ctrl+Shift+Esc launches", icon: Monitor },
  control_panel: { label: "Control Panel", desc: "Block system-wide settings access", icon: Settings2 },
  registry_editor: { label: "Registry Editor", desc: "Protect HKLM & HKCU hives", icon: Database },
  cmd: { label: "Command Prompt", desc: "Block shell command execution", icon: Terminal },
  powershell: { label: "PowerShell", desc: "Block scripting host", icon: Terminal },
  system_restore: { label: "System Restore", desc: "Prevent restore-point rollbacks", icon: RefreshCw },
};
