import { Shield } from "lucide-react";

export function Footer({ version }: { version?: string }) {
  return (
    <footer className="px-8 py-4 border-t border-[color:var(--border)] glass-subtle flex items-center justify-between text-xs text-[color:var(--muted-foreground)]">
      <div className="flex items-center gap-2">
        <Shield className="w-3.5 h-3.5 text-[color:var(--primary)]" />
        OmniLock Security System · Developed by <span className="text-[color:var(--foreground)]">InnologyBD</span>
      </div>
      <div className="flex items-center gap-4">
        <span>v{version || "?"}</span>
        <span>Windows 11 · x64</span>
        <span className="flex items-center gap-1.5"><span className="w-1.5 h-1.5 rounded-full bg-[color:var(--success)]" /> All systems nominal</span>
      </div>
    </footer>
  );
}
