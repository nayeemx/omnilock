import { useState, useEffect } from "react";
import { Activity, LogOut } from "lucide-react";
import { type VaultConfigDto, getWatchdogStatus, type WatchdogStatusDto, lockNow } from "../../lib/tauri-bridge";
import { tabs, type TabId } from "../types";

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function Sidebar({ tab, setTab, config, onLogout }: { tab: TabId; setTab: (t: TabId) => void; config: VaultConfigDto | null; onLogout: () => void }) {
  const [watchdog, setWatchdog] = useState<WatchdogStatusDto | null>(null);

  useEffect(() => {
    getWatchdogStatus().then(setWatchdog).catch(() => {});
    const interval = setInterval(() => {
      getWatchdogStatus().then(setWatchdog).catch(() => {});
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <aside className="w-72 shrink-0 p-5 border-r border-[color:var(--border)] glass-subtle flex flex-col gap-2">
      <div className="flex items-center gap-3 px-2 pt-1 pb-6">
        <div className="relative w-11 h-11 rounded-xl overflow-hidden glow-cyan">
          <img src="/icon.png" alt="OmniLock" className="w-full h-full object-cover" />
        </div>
        <div>
          <div className="text-lg font-semibold tracking-tight">OmniLock</div>
          <div className="text-[11px] text-[color:var(--muted-foreground)] tracking-wider uppercase">by InnologyBD</div>
        </div>
      </div>

      <div className="px-2 mb-2 text-[11px] uppercase tracking-widest text-[color:var(--muted-foreground)]">Modules</div>
      <nav className="flex flex-col gap-1">
        {tabs.map(({ id, label, icon: Icon }) => {
          const active = tab === id;
          return (
            <button key={id} onClick={() => setTab(id)}
                    className={`group flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all ${
                      active
                        ? "bg-surface text-[color:var(--foreground)] border border-surface-border"
                        : "text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] hover:bg-surface border border-transparent"
                    }`}>
              <Icon className={`w-4 h-4 ${active ? "text-[color:var(--primary)]" : ""}`} />
              <span className="flex-1 text-left">{label}</span>
              {active && <span className="w-1.5 h-1.5 rounded-full bg-[color:var(--primary)] glow-cyan" />}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto">
        <button onClick={async () => { await lockNow(); onLogout(); }}
                className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] hover:bg-surface border border-transparent transition-all mb-2">
          <LogOut className="w-4 h-4" />
          <span className="flex-1 text-left">Lock & Logout</span>
        </button>

        <div className="glass rounded-xl p-4">
        <div className="flex items-center gap-2 mb-3">
          <Activity className="w-4 h-4 text-[color:var(--success)]" />
          <span className="text-xs uppercase tracking-widest text-[color:var(--muted-foreground)]">Guardian Daemon</span>
        </div>
        <div className="flex items-baseline justify-between mb-2">
          <span className="text-2xl font-semibold">{watchdog?.status || "Starting"}</span>
          <span className="text-xs text-[color:var(--success)]">● {watchdog ? "Healthy" : "Initializing"}</span>
        </div>
        <div className="text-xs text-[color:var(--muted-foreground)]">
          omnilock.exe · PID {watchdog?.pid || "..."}
        </div>
        <div className="mt-3 pt-3 border-t border-[color:var(--border)] text-xs text-[color:var(--muted-foreground)]">
          Uptime <span className="text-[color:var(--foreground)]">{watchdog ? formatUptime(watchdog.uptime_secs) : "..."}</span>
        </div>
        </div>
      </div>
    </aside>
  );
}
