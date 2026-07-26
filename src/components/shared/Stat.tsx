import React from "react";

export function Stat({ label, value, icon: Icon, tone }: {
  label: string; value: string; icon: React.ElementType;
  tone: "cyan" | "violet" | "success" | "warning";
}) {
  const color = { cyan: "var(--cyan)", violet: "var(--violet)", success: "var(--success)", warning: "var(--warning)" }[tone];
  return (
    <div className="glass rounded-2xl p-5">
      <div className="flex items-center justify-between">
        <div className="w-9 h-9 rounded-lg grid place-items-center" style={{ background: `color-mix(in oklab, ${color} 15%, transparent)`, color }}>
          <Icon className="w-4 h-4" />
        </div>
      </div>
      <div className="mt-4 text-2xl font-semibold tracking-tight">{value}</div>
      <div className="text-xs text-[color:var(--muted-foreground)] mt-1">{label}</div>
    </div>
  );
}
