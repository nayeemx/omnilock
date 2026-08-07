import { useState, useEffect } from "react";
import { ShieldAlert, Fingerprint, HardDrive, FileLock2, Bug, RefreshCw, CheckCircle2, XCircle, AlertTriangle, Cog, Activity, RefreshCcw, ShieldCheck, Menu, Search, Wrench } from "lucide-react";
import { getDiagnostics, recoverAcl, installContextMenu, uninstallContextMenu, isContextMenuInstalled, scanAclDamage, bulkRecoverAcl, type DiagnosticsDto } from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";

export function DiagnosticsPage() {
  const [data, setData] = useState<DiagnosticsDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const load = async () => {
    setLoading(true);
    setError("");
    try {
      const d = await getDiagnostics();
      setData(d);
    } catch (e: any) {
      setError(String(e));
    }
    setLoading(false);
  };

  useEffect(() => { load(); }, []);

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <SectionHeader
        eyebrow="DIAG · Observability"
        title="System Diagnostics"
        subtitle="Live health checks for every feature. If something is broken, this page tells you exactly what and why."
      />

      {error && <div className="p-3 rounded-lg bg-[color:var(--destructive)]/15 border text-sm text-[color:var(--destructive)] border-[color:var(--destructive)]/30">{error}</div>}

      <div className="flex items-center gap-3">
        <button onClick={load} disabled={loading}
                className="px-4 py-2 rounded-lg text-sm border border-[color:var(--border)] flex items-center gap-2 hover:bg-surface disabled:opacity-40">
          <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
          {loading ? "Scanning..." : "Refresh"}
        </button>
        <span className="text-xs text-[color:var(--muted-foreground)]">{data ? `v${data.version}` : ""}</span>
      </div>

      {data && (
        <>
          <div className="grid md:grid-cols-2 gap-4">
            <StatusCard icon={FileLock2} title="Locked Items" color="var(--primary)">
              {data.locked_items_check.length === 0 ? (
                <p className="text-sm text-[color:var(--muted-foreground)]">No items locked</p>
              ) : (
                <div className="space-y-2">
                  {data.locked_items_check.map((item, i) => (
                    <div key={i} className="flex items-center gap-2 text-xs border-b border-surface-border pb-1 last:border-0">
                      {item.locked
                        ? <ShieldCheck className="w-3.5 h-3.5 shrink-0" style={{ color: "var(--success)" }} />
                        : <XCircle className="w-3.5 h-3.5 shrink-0" style={{ color: "var(--destructive)" }} />}
                      <span className="text-[10px] uppercase text-[color:var(--muted-foreground)]">{item.kind}</span>
                      <span className="font-mono truncate flex-1">{item.path}</span>
                      {!item.exists && <span className="text-[color:var(--destructive)]">MISSING</span>}
                      {item.check_error && <span className="text-[color:var(--destructive)]">{item.check_error}</span>}
                    </div>
                  ))}
                </div>
              )}
            </StatusCard>

            <StatusCard icon={Fingerprint} title="Biometric" color="var(--violet)">
              <div className="space-y-2 text-sm">
                <Row label="Hardware available" ok={data.biometric.hardware_available} />
                <Row label="Token saved" ok={data.biometric.token_exists} />
                <Row label="Token decrypts" ok={data.biometric.token_load_ok} />
                {!data.biometric.hardware_available && data.biometric.reason && (
                  <p className="text-xs text-[color:var(--destructive)]">{data.biometric.reason}</p>
                )}
                {data.biometric.last_error && <p className="text-xs text-[color:var(--destructive)]">{data.biometric.last_error}</p>}
              </div>
            </StatusCard>

            <StatusCard icon={Activity} title="Service" color="var(--success)">
              <div className="space-y-2 text-sm">
                <Row label="Windows Service running" ok={data.service.running} />
                <p className="text-xs text-[color:var(--muted-foreground)]">
                  {data.service.running ? "Guardian pipe daemon is active" : "Service not running — ACL enforcement still works via app"}
                </p>
              </div>
            </StatusCard>

            <StatusCard icon={Cog} title="Drives" color="var(--warning)">
              {data.drive_states.length === 0 ? (
                <p className="text-sm text-[color:var(--muted-foreground)]">No drives detected</p>
              ) : (
                <div className="space-y-1 text-sm">
                  {data.drive_states.map((d, i) => (
                    <div key={i} className="flex items-center gap-2 text-xs">
                      <span className="font-mono">{d.drive_letter}:\\</span>
                      {d.policy_active
                        ? <span className="text-[color:var(--success)]">Locked</span>
                        : <span className="text-[color:var(--muted-foreground)]">Unlocked</span>}
                    </div>
                  ))}
                </div>
              )}
            </StatusCard>
          </div>

          <div className="glass rounded-2xl overflow-hidden">
            <div className="p-5 border-b border-[color:var(--border)] flex items-center gap-3">
              <Bug className="w-5 h-5 text-[color:var(--muted-foreground)]" />
              <h3 className="font-semibold">Operation Log</h3>
              <span className="text-xs text-[color:var(--muted-foreground)]">Last 16KB</span>
            </div>
            <div className="p-5 max-h-96 overflow-auto">
              {data.log_tail ? (
                <pre className="text-xs font-mono leading-relaxed whitespace-pre-wrap break-all text-[color:var(--muted-foreground)]">
                  {data.log_tail}
                </pre>
              ) : (
                <p className="text-sm text-[color:var(--muted-foreground)]">No log entries yet</p>
              )}
            </div>
          </div>

          <ContextMenuSection />

          <div className="glass rounded-2xl overflow-hidden">
            <div className="p-5 border-b border-[color:var(--border)] flex items-center gap-3">
              <ShieldCheck className="w-5 h-5 text-[color:var(--warning)]" />
              <h3 className="font-semibold">ACL Recovery</h3>
              <span className="text-xs text-[color:var(--muted-foreground)]">Scan and repair files damaged by the old lock mechanism</span>
            </div>
            <div className="p-5 space-y-3">
              <p className="text-xs text-[color:var(--muted-foreground)]">
                The old lock applied SYSTEM ownership to files and folders, making them inaccessible.
                Scan any path to find affected items, then fix them all at once.
              </p>
              <AclScannerForm />
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function ContextMenuSection() {
  const [installed, setInstalled] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    isContextMenuInstalled().then(setInstalled).catch(() => setInstalled(false));
  }, []);

  const handleInstall = async () => {
    setLoading(true);
    setMsg("");
    try {
      const result = await installContextMenu();
      setInstalled(true);
      setMsg(result);
    } catch (e: any) {
      setMsg("Failed: " + e);
    }
    setLoading(false);
  };

  const handleUninstall = async () => {
    setLoading(true);
    setMsg("");
    try {
      const result = await uninstallContextMenu();
      setInstalled(false);
      setMsg(result);
    } catch (e: any) {
      setMsg("Failed: " + e);
    }
    setLoading(false);
  };

  return (
    <div className="glass rounded-2xl overflow-hidden">
      <div className="p-5 border-b border-[color:var(--border)] flex items-center gap-3">
        <Menu className="w-5 h-5 text-[color:var(--primary)]" />
        <h3 className="font-semibold">Shell Context Menu</h3>
        <span className="text-xs text-[color:var(--muted-foreground)]">Right-click Lock/Unlock in Explorer</span>
        {installed !== null && (
          <span className={`ml-auto text-xs flex items-center gap-1 ${installed ? 'text-[color:var(--success)]' : 'text-[color:var(--muted-foreground)]'}`}>
            {installed ? <CheckCircle2 className="w-3 h-3" /> : <XCircle className="w-3 h-3" />}
            {installed ? "Installed" : "Not installed"}
          </span>
        )}
      </div>
      <div className="p-5 space-y-3">
        <p className="text-xs text-[color:var(--muted-foreground)]">
          Registers "Lock with OmniLock" and "Unlock with OmniLock" in the right-click context menu for files, folders, and drives in Windows Explorer.
        </p>
        <div className="flex items-center gap-3">
          <button onClick={handleInstall} disabled={loading || installed === true}
                  className="px-4 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan disabled:opacity-40"
                  style={{ background: "var(--gradient-brand)" }}>
            {loading ? "Working..." : "Install Context Menu"}
          </button>
          <button onClick={handleUninstall} disabled={loading || installed === false}
                  className="px-4 py-2 rounded-lg text-sm bg-surface border border-surface-border flex items-center gap-2 hover:bg-surface-active disabled:opacity-40">
            {loading ? "Working..." : "Uninstall"}
          </button>
        </div>
        {msg && <div className="text-xs text-[color:var(--muted-foreground)]">{msg}</div>}
      </div>
    </div>
  );
}

function AclScannerForm() {
  const [path, setPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [fixing, setFixing] = useState(false);
  const [damaged, setDamaged] = useState<string[] | null>(null);
  const [results, setResults] = useState<[string, string][] | null>(null);
  const [scanError, setScanError] = useState("");

  const handleScan = async () => {
    if (!path.trim()) return;
    setScanning(true);
    setScanError("");
    setDamaged(null);
    setResults(null);
    try {
      const found = await scanAclDamage(path.trim());
      setDamaged(found);
    } catch (e: any) {
      setScanError(String(e));
    }
    setScanning(false);
  };

  const handleFixAll = async () => {
    if (!damaged || damaged.length === 0) return;
    setFixing(true);
    setResults(null);
    try {
      const res = await bulkRecoverAcl(damaged);
      setResults(res);
      setDamaged(null);
    } catch (e: any) {
      setScanError(String(e));
    }
    setFixing(false);
  };

  const handleFixOne = async (p: string) => {
    try {
      const res = await bulkRecoverAcl([p]);
      // Only remove from the damaged list when the fix actually succeeded,
      // so failed items stay visible for retry.
      if (res.length > 0 && res[0][1] === "ok") {
        setDamaged(prev => prev ? prev.filter(x => x !== p) : []);
      }
      setResults(prev => [...(prev ?? []), ...res]);
    } catch (e: any) {
      setScanError(String(e));
    }
  };

  return (
    <div className="space-y-4">
      {/* path input + scan */}
      <div className="flex items-center gap-2">
        <input
          value={path}
          onChange={e => setPath(e.target.value)}
          placeholder="Enter a folder path to scan (e.g. C:\Users\me\Documents)"
          className="flex-1 px-3 py-2 rounded-lg bg-surface border border-[color:var(--border)] text-sm font-mono outline-none focus:border-[color:var(--primary)]"
          onKeyDown={e => { if (e.key === "Enter") handleScan(); }}
        />
        <button onClick={handleScan} disabled={scanning || !path.trim()}
                className="px-4 py-2 rounded-lg text-sm border border-[color:var(--border)] flex items-center gap-2 hover:bg-surface disabled:opacity-40 shrink-0">
          <Search className={`w-4 h-4 ${scanning ? "animate-spin" : ""}`} />
          {scanning ? "Scanning..." : "Scan"}
        </button>
      </div>

      {scanError && (
        <div className="p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
          {scanError}
        </div>
      )}

      {/* scan results */}
      {damaged !== null && (
        <div className="space-y-3">
          {damaged.length === 0 ? (
            <div className="p-3 rounded-lg bg-[color:var(--success)]/15 border border-[color:var(--success)]/30 text-sm text-[color:var(--success)] flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 shrink-0" />
              No damaged items found under this path.
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between">
                <span className="text-sm text-[color:var(--warning)] flex items-center gap-2">
                  <AlertTriangle className="w-4 h-4" />
                  {damaged.length} damaged item{damaged.length !== 1 ? "s" : ""} found (owned by SYSTEM)
                </span>
                <button onClick={handleFixAll} disabled={fixing}
                        className="px-4 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan disabled:opacity-40"
                        style={{ background: "var(--gradient-brand)" }}>
                  <Wrench className={`w-4 h-4 ${fixing ? "animate-spin" : ""}`} />
                  {fixing ? "Fixing..." : `Fix All ${damaged.length} Items`}
                </button>
              </div>
              <div className="rounded-lg border border-[color:var(--border)] divide-y divide-[color:var(--border)] max-h-64 overflow-auto">
                {damaged.map((p, i) => (
                  <div key={i} className="flex items-center gap-2 px-3 py-2 text-xs">
                    <XCircle className="w-3.5 h-3.5 shrink-0 text-[color:var(--destructive)]" />
                    <span className="font-mono flex-1 truncate">{p}</span>
                    <button onClick={() => handleFixOne(p)}
                            className="px-2 py-1 rounded text-[10px] border border-[color:var(--border)] hover:bg-surface shrink-0">
                      Fix
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      )}

      {/* fix results */}
      {results !== null && results.length > 0 && (
        <div className="rounded-lg border border-[color:var(--border)] divide-y divide-[color:var(--border)] max-h-48 overflow-auto">
          {results.map(([p, status], i) => (
            <div key={i} className="flex items-center gap-2 px-3 py-2 text-xs">
              {status === "ok"
                ? <CheckCircle2 className="w-3.5 h-3.5 shrink-0 text-[color:var(--success)]" />
                : <XCircle className="w-3.5 h-3.5 shrink-0 text-[color:var(--destructive)]" />}
              <span className="font-mono flex-1 truncate">{p}</span>
              <span className={status === "ok" ? "text-[color:var(--success)]" : "text-[color:var(--destructive)] max-w-[160px] truncate"}>{status}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function RecoverAclForm() {
  const [path, setPath] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; msg: string } | null>(null);

  const handleRecover = async () => {
    if (!path.trim()) return;
    setLoading(true);
    setResult(null);
    try {
      const msg = await recoverAcl(path.trim());
      setResult({ ok: true, msg });
    } catch (e: any) {
      setResult({ ok: false, msg: String(e) });
    }
    setLoading(false);
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <input
          value={path}
          onChange={e => setPath(e.target.value)}
          placeholder="Enter full path to file or folder (e.g. C:\Users\me\locked-folder)"
          className="flex-1 px-3 py-2 rounded-lg bg-surface border border-[color:var(--border)] text-sm font-mono outline-none focus:border-[color:var(--primary)]"
          onKeyDown={e => { if (e.key === "Enter") handleRecover(); }}
        />
        <button onClick={handleRecover} disabled={loading || !path.trim()}
                className="px-4 py-2 rounded-lg text-sm border border-[color:var(--border)] flex items-center gap-2 hover:bg-surface disabled:opacity-40">
          <RefreshCcw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
          {loading ? "Recovering..." : "Recover ACL"}
        </button>
      </div>
      {result && (
        <div className={`p-3 rounded-lg text-sm ${result.ok ? "bg-[color:var(--success)]/15 text-[color:var(--success)]" : "bg-[color:var(--destructive)]/15 text-[color:var(--destructive)]"} border ${result.ok ? "border-[color:var(--success)]/30" : "border-[color:var(--destructive)]/30"}`}>
          {result.msg}
        </div>
      )}
    </div>
  );
}

function StatusCard({ icon: Icon, title, color, children }: { icon: any; title: string; color: string; children: React.ReactNode }) {
  return (
    <div className="glass rounded-2xl p-5">
      <div className="flex items-center gap-3 mb-4">
        <div className="w-10 h-10 rounded-lg grid place-items-center" style={{ background: `color-mix(in oklab, ${color} 15%, transparent)`, color }}>
          <Icon className="w-5 h-5" />
        </div>
        <h3 className="font-semibold">{title}</h3>
      </div>
      {children}
    </div>
  );
}

function Row({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className="flex items-center gap-2">
      {ok
        ? <CheckCircle2 className="w-4 h-4" style={{ color: "var(--success)" }} />
        : <XCircle className="w-4 h-4" style={{ color: "var(--destructive)" }} />}
      <span className={ok ? "" : "text-[color:var(--destructive)]"}>{label}</span>
    </div>
  );
}
