import React from "react";

export function Field({ label, icon: Icon, children }: { label: string; icon: React.ElementType; children: React.ReactNode }) {
  return (
    <label className="block">
      <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)] mb-1.5">{label}</div>
      <div className="flex items-center gap-2 px-3 py-2.5 rounded-lg bg-white/[0.03] border border-white/10 focus-within:border-[color:var(--primary)]/50 transition-colors">
        <Icon className="w-4 h-4 text-[color:var(--muted-foreground)]" />
        {children}
      </div>
    </label>
  );
}
