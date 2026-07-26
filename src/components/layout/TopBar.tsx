import { Lock, Fingerprint, Zap } from "lucide-react";

export function TopBar({ onLockNow, totpEnabled }: { onLockNow: () => void; totpEnabled?: boolean }) {
  return (
    <header className="h-16 px-8 border-b border-[color:var(--border)] flex items-center justify-between glass-subtle">
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/[0.04] border border-white/10">
          <span className="relative flex w-2 h-2">
            <span className="animate-ping absolute inline-flex w-full h-full rounded-full bg-[color:var(--success)] opacity-60"></span>
            <span className="relative inline-flex w-2 h-2 rounded-full bg-[color:var(--success)]"></span>
          </span>
          <span className="text-xs">Vault unlocked</span>
        </div>
        {totpEnabled && (
          <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white/[0.04] border border-white/10 text-xs">
            <Fingerprint className="w-3.5 h-3.5 text-[color:var(--violet)]" />
            2FA enforced
          </div>
        )}
      </div>
      <div className="flex items-center gap-3">
        <button className="px-3 py-1.5 rounded-lg text-xs text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] border border-transparent hover:border-white/10 flex items-center gap-2">
          <Zap className="w-3.5 h-3.5" /> Win + Alt + L
        </button>
        <button onClick={onLockNow}
                className="px-4 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan"
                style={{ background: "var(--gradient-brand)" }}>
          <Lock className="w-4 h-4" /> Lock Now
        </button>
      </div>
    </header>
  );
}
