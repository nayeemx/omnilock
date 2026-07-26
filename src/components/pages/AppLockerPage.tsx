import { useState } from "react";
import {
  ShieldCheck, ShieldAlert, Cpu, Activity,
  Search, Plus, RefreshCw, X,
} from "lucide-react";
import { listProcesses, addLockedApp, toggleLockedApp, removeLockedApp, showWidget, type VaultConfigDto } from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";
import { Stat } from "../shared/Stat";
import { StatusPill } from "../shared/StatusPill";
import { Toggle } from "../shared/Toggle";

export function AppLockerPage({ config, refresh }: { config: VaultConfigDto | null; refresh: () => Promise<void> }) {
  const [processes, setProcesses] = useState<[string, string, string][]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [search, setSearch] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  const lockedApps = config?.locked_apps || [];
  const filtered = processes.filter(([name, path]) =>
    name.toLowerCase().includes(search.toLowerCase()) || path.toLowerCase().includes(search.toLowerCase())
  );

  const clearMessages = () => { setError(""); setSuccess(""); };

  const handleScan = async () => {
    clearMessages();
    setScanning(true);
    try {
      const procs = await listProcesses();
      setProcesses(procs);
      setShowAdd(true);
      if (procs.length === 0) setError("No running processes found.");
    } catch (e: any) {
      setError("Failed to scan processes: " + e);
    }
    setScanning(false);
  };

  const handleAdd = async (name: string, path: string, sha256: string) => {
    clearMessages();
    try {
      await addLockedApp(name, path, sha256);
      setSuccess(`"${name}" added to locked apps.`);
      await refresh();
    } catch (e: any) {
      setError("Failed to add app: " + e);
    }
  };

  const handleToggle = async (name: string, enabled: boolean) => {
    clearMessages();
    try {
      await toggleLockedApp(name, enabled);
      setSuccess(`"${name}" ${enabled ? "enabled" : "disabled"}.`);
      await refresh();
    } catch (e: any) {
      setError("Failed to toggle: " + e);
      await refresh();
    }
  };

  const handleRemove = async (name: string) => {
    clearMessages();
    try {
      await removeLockedApp(name);
      setSuccess(`"${name}" removed.`);
      await refresh();
    } catch (e: any) {
      setError("Failed to remove: " + e);
    }
  };

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <SectionHeader
        eyebrow="FR-APP · Process Guard"
        title="Application Protection"
        subtitle="Deep process monitoring with SHA-256 binary hashing. Renaming a locked executable does not bypass protection."
      />

      {error && <div className="p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">{error}</div>}
      {success && <div className="p-3 rounded-lg bg-[color:var(--success)]/15 border border-[color:var(--success)]/30 text-sm text-[color:var(--success)]">{success}</div>}

      <div className="grid grid-cols-3 gap-4">
        <Stat label="Protected Apps" value={String(lockedApps.length)} icon={ShieldCheck} tone="cyan" />
        <Stat label="Active" value={String(lockedApps.filter(a => a.enabled).length)} icon={ShieldAlert} tone="violet" />
        <Stat label="Inactive" value={String(lockedApps.filter(a => !a.enabled).length)} icon={Cpu} tone="success" />
      </div>

      <div className="glass rounded-2xl overflow-hidden">
        <div className="p-5 border-b border-[color:var(--border)] flex items-center gap-3">
          <div className="flex-1 flex items-center gap-2 px-3 py-2 rounded-lg bg-white/[0.03] border border-white/10">
            <Search className="w-4 h-4 text-[color:var(--muted-foreground)]" />
            <input placeholder="Search locked apps..."
                   value={search} onChange={e => setSearch(e.target.value)}
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </div>
          <button onClick={handleScan} disabled={scanning}
                  className="px-4 py-2 rounded-lg text-sm bg-white/[0.04] border border-white/10 flex items-center gap-2 hover:bg-white/[0.08]">
            <RefreshCw className={`w-4 h-4 ${scanning ? "animate-spin" : ""}`} /> Scan
          </button>
          <button onClick={() => { setProcesses([]); setShowAdd(true); handleScan(); }}
                  className="px-4 py-2 rounded-lg text-sm font-medium flex items-center gap-2 text-[color:var(--primary-foreground)] glow-cyan"
                  style={{ background: "var(--gradient-brand)" }}>
            <Plus className="w-4 h-4" /> Add Application
          </button>
        </div>

        <div className="divide-y divide-white/[0.06]">
          {lockedApps.length === 0 && (
            <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
              No apps locked yet. Click "Scan" to find running processes and add them.
            </div>
          )}
          {lockedApps.map(app => (
            <div key={app.name} className="px-5 py-4 flex items-center gap-4 hover:bg-white/[0.02] transition">
              <div className="w-11 h-11 rounded-xl bg-white/[0.04] border border-white/10 grid place-items-center text-xl">
                {app.name.includes("chrome") ? "🌐" : app.name.includes("telegram") ? "💬" : app.name.includes("code") ? "💻" : app.name.includes("discord") ? "🎧" : "⚙️"}
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium">{app.name}</span>
                </div>
                <div className="text-xs text-[color:var(--muted-foreground)] truncate">{app.path}</div>
              </div>
              <div className="hidden md:flex flex-col items-end mr-2">
                <span className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)]">SHA-256</span>
                <code className="text-xs">{app.sha256 ? app.sha256.slice(0, 16) + "…" : "—"}</code>
              </div>
              <StatusPill locked={app.enabled} />
              <Toggle on={app.enabled} onChange={(v) => handleToggle(app.name, v)} />
              <button onClick={() => handleRemove(app.name)} className="p-1.5 rounded-lg hover:bg-white/[0.06] text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)]">
                <X className="w-4 h-4" />
              </button>
              <button onClick={() => showWidget("app", app.path, app.name)}
                      className="px-3 py-1.5 rounded-lg text-xs border border-[color:var(--success)]/30 text-[color:var(--success)] hover:bg-[color:var(--success)]/10">
                Unlock
              </button>
            </div>
          ))}
        </div>
      </div>

      {showAdd && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass rounded-2xl w-full max-w-2xl max-h-[80vh] flex flex-col">
            <div className="p-5 border-b border-[color:var(--border)] flex items-center justify-between">
              <h3 className="font-semibold">Add Application to Lock</h3>
              <button onClick={() => setShowAdd(false)} className="p-1.5 rounded-lg hover:bg-white/[0.06]"><X className="w-4 h-4" /></button>
            </div>
            <div className="p-4">
              <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-white/[0.03] border border-white/10">
                <Search className="w-4 h-4 text-[color:var(--muted-foreground)]" />
                <input placeholder="Filter processes..." value={search} onChange={e => setSearch(e.target.value)}
                       className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
              </div>
            </div>
            <div className="flex-1 overflow-auto divide-y divide-white/[0.06]">
              {filtered.map(([name, path, sha]) => (
                <div key={name + path} className="px-5 py-3 flex items-center gap-4 hover:bg-white/[0.02]">
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm">{name}</div>
                    <div className="text-xs text-[color:var(--muted-foreground)] truncate">{path}</div>
                  </div>
                  <code className="text-[10px] text-[color:var(--muted-foreground)]">{sha ? sha.slice(0, 12) + "…" : "—"}</code>
                  <button onClick={() => { handleAdd(name, path, sha); }}
                          className="px-3 py-1.5 rounded-lg text-xs font-medium text-[color:var(--primary-foreground)] flex items-center gap-1"
                          style={{ background: "var(--gradient-brand)" }}>
                    <Plus className="w-3 h-3" /> Lock
                  </button>
                </div>
              ))}
              {filtered.length === 0 && (
                <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
                  {processes.length === 0 ? "Click Scan to load running processes." : "No matching processes."}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
