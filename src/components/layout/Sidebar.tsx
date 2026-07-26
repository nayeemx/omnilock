import { Shield, Activity } from "lucide-react";
import { type VaultConfigDto } from "../../lib/tauri-bridge";
import { tabs, type TabId } from "../types";

export function Sidebar({ tab, setTab, config }: { tab: TabId; setTab: (t: TabId) => void; config: VaultConfigDto | null }) {
  return (
    <aside className="w-72 shrink-0 p-5 border-r border-[color:var(--border)] glass-subtle flex flex-col gap-2">
      <div className="flex items-center gap-3 px-2 pt-1 pb-6">
        <div className="relative w-11 h-11 rounded-xl grid place-items-center" style={{ background: "var(--gradient-brand)" }}>
          <Shield className="w-6 h-6 text-primary-foreground" strokeWidth={2.5} />
          <div className="absolute inset-0 rounded-xl glow-cyan opacity-60" />
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
                        ? "bg-white/[0.06] text-[color:var(--foreground)] border border-white/10"
                        : "text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] hover:bg-white/[0.03] border border-transparent"
                    }`}>
              <Icon className={`w-4 h-4 ${active ? "text-[color:var(--primary)]" : ""}`} />
              <span className="flex-1 text-left">{label}</span>
              {active && <span className="w-1.5 h-1.5 rounded-full bg-[color:var(--primary)] glow-cyan" />}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto glass rounded-xl p-4">
        <div className="flex items-center gap-2 mb-3">
          <Activity className="w-4 h-4 text-[color:var(--success)]" />
          <span className="text-xs uppercase tracking-widest text-[color:var(--muted-foreground)]">Guardian Daemon</span>
        </div>
        <div className="flex items-baseline justify-between mb-2">
          <span className="text-2xl font-semibold">Active</span>
          <span className="text-xs text-[color:var(--success)]">● Healthy</span>
        </div>
        <div className="text-xs text-[color:var(--muted-foreground)]">omnilock-guard.exe · PID 4812</div>
        <div className="mt-3 pt-3 border-t border-[color:var(--border)] text-xs text-[color:var(--muted-foreground)]">
          Uptime <span className="text-[color:var(--foreground)]">14d 07h 22m</span>
        </div>
      </div>
    </aside>
  );
}
