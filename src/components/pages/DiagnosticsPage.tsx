import { useState, useEffect } from "react";
import { ShieldAlert, Fingerprint, HardDrive, FileLock2, Bug, RefreshCw, CheckCircle2, XCircle, AlertTriangle, Cog, Activity } from "lucide-react";
import { getDiagnostics, type DiagnosticsDto } from "../../lib/tauri-bridge";
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
                      {item.deny_ace_present
                        ? <CheckCircle2 className="w-3.5 h-3.5 shrink-0" style={{ color: "var(--success)" }} />
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
                <p className="text-xs text-[color:var(--muted-foreground)]">{data.biometric.reason}</p>
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
        </>
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
