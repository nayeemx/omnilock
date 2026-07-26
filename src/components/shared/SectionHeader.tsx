export function SectionHeader({ eyebrow, title, subtitle }: { eyebrow: string; title: string; subtitle: string }) {
  return (
    <div>
      <div className="text-[11px] uppercase tracking-widest text-[color:var(--primary)] mb-2">{eyebrow}</div>
      <h1 className="text-3xl font-semibold tracking-tight">{title}</h1>
      <p className="text-sm text-[color:var(--muted-foreground)] mt-2 max-w-2xl">{subtitle}</p>
    </div>
  );
}
