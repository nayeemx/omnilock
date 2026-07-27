import { Lock, Unlock } from "lucide-react";

export function StatusPill({ locked }: { locked: boolean }) {
  return locked ? (
    <span className="text-xs px-2.5 py-1 rounded-full bg-[color:var(--primary)]/10 text-[color:var(--primary)] border border-[color:var(--primary)]/30 flex items-center gap-1.5">
      <Lock className="w-3 h-3" /> Protected
    </span>
  ) : (
    <span className="text-xs px-2.5 py-1 rounded-full bg-surface text-[color:var(--muted-foreground)] border border-surface-border flex items-center gap-1.5">
      <Unlock className="w-3 h-3" /> Unlocked
    </span>
  );
}
