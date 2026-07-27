import { useState } from "react";
import {
  ShieldCheck, ShieldAlert, Cpu, Activity,
  Search, Plus, RefreshCw, X, Loader2, CheckCircle2,
} from "lucide-react";
import { listProcesses, listInstalledApps, addLockedApp, toggleLockedApp, removeLockedApp, showWidget, type VaultConfigDto } from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";
import { Stat } from "../shared/Stat";
import { StatusPill } from "../shared/StatusPill";
import { Toggle } from "../shared/Toggle";

type AppEntry = [string, string, string];

export function AppLockerPage({ config, refresh }: { config: VaultConfigDto | null; refresh: () => Promise<void> }) {
  const [processes, setProcesses] = useState<AppEntry[]>([]);
  const [installedApps, setInstalledApps] = useState<AppEntry[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [search, setSearch] = useState("");
  const [lockedSearch, setLockedSearch] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [lockingApp, setLockingApp] = useState<string | null>(null);
  const [removingApp, setRemovingApp] = useState<string | null>(null);
  const [togglingApp, setTogglingApp] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"running" | "installed">("running");

  const lockedApps = config?.locked_apps || [];
  const filteredLocked = lockedApps.filter(app =>
    app.name.toLowerCase().includes(lockedSearch.toLowerCase()) ||
    app.path.toLowerCase().includes(lockedSearch.toLowerCase())
  );

  const filteredProcesses = processes.filter(([name, path]) =>
    name.toLowerCase().includes(search.toLowerCase()) || path.toLowerCase().includes(search.toLowerCase())
  );

  const filteredInstalled = installedApps.filter(([name, path]) =>
    name.toLowerCase().includes(search.toLowerCase()) || path.toLowerCase().includes(search.toLowerCase())
  );

  const clearMessages = () => { setError(""); setSuccess(""); };

  const handleScan = async () => {
    clearMessages();
    setScanning(true);
    setShowAdd(true);
    setSearch("");
    try {
      const [procs, installed] = await Promise.all([listProcesses(), listInstalledApps()]);
      setProcesses(procs);
      setInstalledApps(installed);
      if (procs.length === 0 && installed.length === 0) setError("No processes or installed apps found.");
    } catch (e: any) {
      setError("Failed to scan: " + e);
    }
    setScanning(false);
  };

  const handleOpenModal = () => {
    setSearch("");
    setShowAdd(true);
    if (processes.length === 0 && installedApps.length === 0) {
      handleScan();
    }
  };

  const handleAdd = async (name: string, path: string, sha256: string) => {
    clearMessages();
    setLockingApp(name);
    try {
      await addLockedApp(name, path, sha256);
      setSuccess(`"${name}" added to locked apps.`);
      await refresh();
    } catch (e: any) {
      setError("Failed to add app: " + e);
    }
    setLockingApp(null);
  };

  const handleToggle = async (name: string, enabled: boolean) => {
    clearMessages();
    setTogglingApp(name);
    try {
      await toggleLockedApp(name, enabled);
      setSuccess(`"${name}" ${enabled ? "enabled" : "disabled"}.`);
      await refresh();
    } catch (e: any) {
      setError("Failed to toggle: " + e);
      await refresh();
    }
    setTogglingApp(null);
  };

  const handleRemove = async (name: string) => {
    clearMessages();
    setRemovingApp(name);
    try {
      await removeLockedApp(name);
      setSuccess(`"${name}" removed.`);
      await refresh();
    } catch (e: any) {
      setError("Failed to remove " + e);
    }
    setRemovingApp(null);
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
          <div className="flex-1 flex items-center gap-2 px-3 py-2 rounded-lg bg-surface border border-surface-border">
            <Search className="w-4 h-4 text-[color:var(--muted-foreground)]" />
            <input placeholder="Search locked apps..."
                   value={lockedSearch} onChange={e => setLockedSearch(e.target.value)}
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </div>
          <button onClick={handleScan} disabled={scanning}
                  className="px-4 py-2 rounded-lg text-sm bg-surface border border-surface-border flex items-center gap-2 hover:bg-surface-active">
            <RefreshCw className={`w-4 h-4 ${scanning ? "animate-spin" : ""}`} /> Scan
          </button>
          <button onClick={handleOpenModal}
                  className="px-4 py-2 rounded-lg text-sm font-medium flex items-center gap-2 text-[color:var(--primary-foreground)] glow-cyan"
                  style={{ background: "var(--gradient-brand)" }}>
            <Plus className="w-4 h-4" /> Add Application
          </button>
        </div>

        <div className="divide-y divide-surface-border">
          {lockedApps.length === 0 && (
            <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
              No apps locked yet. Click "Scan" to find running processes and add them.
            </div>
          )}
          {filteredLocked.map(app => (
            <div key={app.name} className="px-5 py-4 flex items-center gap-4 hover:bg-surface transition">
              <div className="w-11 h-11 rounded-xl bg-surface border border-surface-border grid place-items-center text-xl">
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
              <Toggle on={app.enabled} onChange={(v) => handleToggle(app.name, v)} disabled={togglingApp === app.name} />
              {togglingApp === app.name ? (
                <Loader2 className="w-4 h-4 animate-spin text-[color:var(--primary)]" />
              ) : null}
              <button onClick={() => handleRemove(app.name)} disabled={removingApp === app.name}
                      className="p-1.5 rounded-lg hover:bg-surface-hover text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)] disabled:opacity-40">
                {removingApp === app.name ? <Loader2 className="w-4 h-4 animate-spin" /> : <X className="w-4 h-4" />}
              </button>
              <button onClick={() => showWidget("app", app.path, app.name)}
                      className="px-3 py-1.5 rounded-lg text-xs border border-[color:var(--success)]/30 text-[color:var(--success)] hover:bg-[color:var(--success)]/10">
                Unlock
              </button>
            </div>
          ))}
          {lockedSearch && filteredLocked.length === 0 && lockedApps.length > 0 && (
            <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
              No locked apps match "{lockedSearch}".
            </div>
          )}
        </div>
      </div>

      {showAdd && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass rounded-2xl w-full max-w-2xl max-h-[80vh] flex flex-col">
            <div className="p-5 border-b border-[color:var(--border)] flex items-center justify-between">
              <h3 className="font-semibold">Add Application to Lock</h3>
              <button onClick={() => setShowAdd(false)} className="p-1.5 rounded-lg hover:bg-surface-hover"><X className="w-4 h-4" /></button>
            </div>
            <div className="p-4">
              <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-surface border border-surface-border">
                <Search className="w-4 h-4 text-[color:var(--muted-foreground)]" />
                <input placeholder="Search apps..." value={search} onChange={e => setSearch(e.target.value)}
                       className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
              </div>
            </div>
            <div className="flex gap-1 px-4">
              <button onClick={() => setActiveTab("running")}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition ${activeTab === "running" ? "bg-[color:var(--primary)]/15 text-[color:var(--primary)]" : "text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]"}`}>
                Running ({processes.length})
              </button>
              <button onClick={() => setActiveTab("installed")}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition ${activeTab === "installed" ? "bg-[color:var(--primary)]/15 text-[color:var(--primary)]" : "text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]"}`}>
                Installed ({installedApps.length})
              </button>
            </div>
            <div className="flex-1 overflow-auto divide-y divide-surface-border">
              {scanning ? (
                <div className="p-8 flex flex-col items-center gap-3 text-[color:var(--muted-foreground)] text-sm">
                  <Loader2 className="w-6 h-6 animate-spin text-[color:var(--primary)]" />
                  Scanning applications...
                </div>
              ) : (
                <>
                  {activeTab === "running" && filteredProcesses.map(([name, path, sha]) => (
                    <div key={name + path} className="px-5 py-3 flex items-center gap-4 hover:bg-surface">
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-sm">{name}</div>
                        <div className="text-xs text-[color:var(--muted-foreground)] truncate">{path}</div>
                      </div>
                      <code className="text-[10px] text-[color:var(--muted-foreground)]">{sha ? sha.slice(0, 12) + "…" : "—"}</code>
                      {lockingApp === name ? (
                        <div className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-[color:var(--primary)]">
                          <Loader2 className="w-3 h-3 animate-spin" /> Locking...
                        </div>
                      ) : (
                        <button onClick={() => handleAdd(name, path, sha)}
                                className="px-3 py-1.5 rounded-lg text-xs font-medium text-[color:var(--primary-foreground)] flex items-center gap-1"
                                style={{ background: "var(--gradient-brand)" }}>
                          <Plus className="w-3 h-3" /> Lock
                        </button>
                      )}
                    </div>
                  ))}
                  {activeTab === "installed" && filteredInstalled.map(([name, path, sha]) => (
                    <div key={name + path} className="px-5 py-3 flex items-center gap-4 hover:bg-surface">
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-sm">{name}</div>
                        <div className="text-xs text-[color:var(--muted-foreground)] truncate">{path || "No install path"}</div>
                      </div>
                      {lockingApp === name ? (
                        <div className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-[color:var(--primary)]">
                          <Loader2 className="w-3 h-3 animate-spin" /> Locking...
                        </div>
                      ) : (
                        <button onClick={() => handleAdd(name, path, sha)}
                                className="px-3 py-1.5 rounded-lg text-xs font-medium text-[color:var(--primary-foreground)] flex items-center gap-1"
                                style={{ background: "var(--gradient-brand)" }}>
                          <Plus className="w-3 h-3" /> Lock
                        </button>
                      )}
                    </div>
                  ))}
                  {activeTab === "running" && filteredProcesses.length === 0 && (
                    <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
                      {processes.length === 0 ? "Click Scan to load running processes." : "No matching processes."}
                    </div>
                  )}
                  {activeTab === "installed" && filteredInstalled.length === 0 && (
                    <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
                      {installedApps.length === 0 ? "No installed apps found." : "No matching apps."}
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
