import { useState, useEffect, useCallback } from "react";
import { getLockHistory, type HistoryEntry } from "../../lib/tauri-bridge";
import { RefreshCw, Search, Clock, AlertTriangle, Shield, Activity, HardDrive, FileText } from "lucide-react";

const componentIcons: Record<string, React.ElementType> = {
  LOCK: Shield,
  UNLOCK: Clock,
  RESCUE: AlertTriangle,
  DRIVE: HardDrive,
  RECOVER: AlertTriangle,
  BACKUP: FileText,
  SERVICE: Activity,
  BIOMETRIC: Activity,
  SETUP: Activity,
};

function componentColor(component: string): string {
  switch (component) {
    case "LOCK": return "var(--destructive)";
    case "UNLOCK": return "var(--success)";
    case "RESCUE": case "RECOVER": return "var(--warning)";
    case "DRIVE": return "var(--primary)";
    default: return "var(--muted-foreground)";
  }
}

function formatTime(ts: string): string {
  const secs = parseInt(ts, 10);
  if (isNaN(secs)) return ts;
  const d = new Date(secs * 1000);
  return d.toLocaleString();
}

export function HistoryPage() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getLockHistory(500);
      setEntries(data);
    } catch (e) {
      console.error("Failed to load history:", e);
    }
    setLoading(false);
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = filter
    ? entries.filter(e =>
        e.component.toLowerCase().includes(filter.toLowerCase()) ||
        e.action.toLowerCase().includes(filter.toLowerCase())
      )
    : entries;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Lock History</h1>
          <p className="text-sm text-[color:var(--muted-foreground)] mt-1">
            Audit trail of all lock/unlock operations
          </p>
        </div>
        <button onClick={load} className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm
          bg-surface border border-surface-border hover:bg-surface-hover transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          Refresh
        </button>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[color:var(--muted-foreground)]" />
        <input
          type="text"
          placeholder="Filter by component or action..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="w-full pl-10 pr-4 py-2 rounded-lg bg-surface border border-surface-border
            text-sm focus:outline-none focus:border-[color:var(--primary)] transition-colors"
        />
      </div>

      <div className="glass rounded-2xl overflow-hidden">
        <div className="max-h-[600px] overflow-y-auto">
          {loading ? (
            <div className="flex items-center justify-center py-12 text-sm text-[color:var(--muted-foreground)]">
              Loading history...
            </div>
          ) : filtered.length === 0 ? (
            <div className="flex items-center justify-center py-12 text-sm text-[color:var(--muted-foreground)]">
              {filter ? "No entries match your filter" : "No lock history yet"}
            </div>
          ) : (
            <table className="w-full">
              <thead className="sticky top-0 bg-surface/95 backdrop-blur">
                <tr className="text-[11px] uppercase tracking-wider text-[color:var(--muted-foreground)] border-b border-surface-border">
                  <th className="text-left px-4 py-3 font-medium">Time</th>
                  <th className="text-left px-4 py-3 font-medium">Type</th>
                  <th className="text-left px-4 py-3 font-medium">Action</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((entry, i) => {
                  const Icon = componentIcons[entry.component] || Activity;
                  return (
                    <tr key={i} className="border-b border-surface-border/50 hover:bg-surface/50 transition-colors">
                      <td className="px-4 py-3 text-xs text-[color:var(--muted-foreground)] whitespace-nowrap">
                        {formatTime(entry.timestamp)}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <Icon className="w-3.5 h-3.5" style={{ color: componentColor(entry.component) }} />
                          <span className="text-xs font-medium" style={{ color: componentColor(entry.component) }}>
                            {entry.component}
                          </span>
                        </div>
                      </td>
                      <td className="px-4 py-3 text-xs text-[color:var(--foreground)] break-all">
                        {entry.action}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </div>

      <div className="text-[11px] text-[color:var(--muted-foreground)] text-right">
        {filtered.length} entries
      </div>
    </div>
  );
}
