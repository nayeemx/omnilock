export function Toggle({ on, onChange, disabled }: { on: boolean; onChange?: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button onClick={() => onChange?.(!on)} disabled={disabled}
            className={`relative w-11 h-6 rounded-full transition-all ${on ? "bg-transparent" : "bg-surface-active"} ${disabled ? "opacity-40 cursor-not-allowed" : ""}`}
            style={on ? { background: "var(--gradient-brand)", boxShadow: "var(--shadow-glow-cyan)" } : undefined}>
      <span className={`absolute top-0.5 w-5 h-5 rounded-full bg-white transition-all ${on ? "left-5" : "left-0.5"}`} />
    </button>
  );
}
