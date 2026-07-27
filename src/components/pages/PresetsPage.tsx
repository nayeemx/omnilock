import { useState } from "react";
import { PackageOpen } from "lucide-react";
import { toggleSystemPreset, toggleInstallerGuard, type VaultConfigDto } from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";
import { Toggle } from "../shared/Toggle";
import { presetMeta } from "../types";

export function PresetsPage({ config, refresh }: { config: VaultConfigDto | null; refresh: () => Promise<void> }) {
  const presets = config?.system_presets;
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  const handleToggle = async (id: string, enabled: boolean) => {
    setError(""); setSuccess("");
    try {
      await toggleSystemPreset(id, enabled);
      setSuccess(`${id.replace(/_/g, " ")} ${enabled ? "blocked" : "unblocked"}.`);
      await refresh();
    } catch (e: any) {
      setError("Failed: " + e);
      await refresh();
    }
  };

  const handleInstallerGuard = async (enabled: boolean) => {
    setError(""); setSuccess("");
    try {
      await toggleInstallerGuard(enabled);
      setSuccess(`Installer guard ${enabled ? "enabled" : "disabled"}.`);
      await refresh();
    } catch (e: any) {
      setError("Failed: " + e);
      await refresh();
    }
  };

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <SectionHeader
        eyebrow="FR-SYS · Lockdown"
        title="System Presets & Installer Guard"
        subtitle="One-click hardening for sensitive Windows utilities. Blocks installer processes before they can elevate."
      />

      {error && <div className="p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">{error}</div>}
      {success && <div className="p-3 rounded-lg bg-[color:var(--success)]/15 border border-[color:var(--success)]/30 text-sm text-[color:var(--success)]">{success}</div>}

      <div className="grid md:grid-cols-2 gap-4">
        {Object.entries(presetMeta).map(([id, meta]) => {
          const enabled = presets ? (presets as any)[id] : false;
          return (
            <div key={id} className="glass rounded-xl p-5 flex items-center gap-4">
              <div className={`w-12 h-12 rounded-xl grid place-items-center ${enabled ? "text-[color:var(--primary)]" : "text-[color:var(--muted-foreground)]"}`}
                   style={{ background: enabled ? "color-mix(in oklab, var(--cyan) 12%, transparent)" : "var(--surface)" }}>
                <meta.icon className="w-5 h-5" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="font-medium">{meta.label}</div>
                <div className="text-xs text-[color:var(--muted-foreground)] mt-0.5">{meta.desc}</div>
              </div>
              <Toggle on={!!enabled} onChange={(v) => handleToggle(id, v)} />
            </div>
          );
        })}
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <PackageOpen className="w-5 h-5 text-[color:var(--violet)]" />
          <div>
            <h3 className="font-semibold">Installer Guard</h3>
            <p className="text-xs text-[color:var(--muted-foreground)]">Intercepts installers before UAC elevation.</p>
          </div>
          {config?.installer_guard_enabled && (
            <span className="ml-auto text-xs px-2.5 py-1 rounded-full border border-[color:var(--violet)]/40 text-[color:var(--violet)] bg-[color:var(--violet)]/10">
              Active
            </span>
          )}
        </div>
        <div className="grid gap-2">
          {[
            { name: "MSI Installer", pattern: "msiexec.exe" },
            { name: "Setup Executables", pattern: "setup.exe, install.exe" },
            { name: "Self-extracting archives", pattern: "*.exe (SFX signature)" },
          ].map(r => (
            <div key={r.name} className="flex items-center gap-4 p-3 rounded-lg bg-surface border border-surface-border">
              <div className="flex-1">
                <div className="text-sm font-medium">{r.name}</div>
                <code className="text-xs text-[color:var(--muted-foreground)]">{r.pattern}</code>
              </div>
            </div>
          ))}
          <div className="mt-2">
            <Toggle on={!!config?.installer_guard_enabled} onChange={handleInstallerGuard} />
          </div>
        </div>
      </div>
    </div>
  );
}
