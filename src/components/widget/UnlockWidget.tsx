import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Lock, Shield, X } from "lucide-react";
import { widgetUnlock } from "../../lib/tauri-bridge";

interface UnlockTarget {
  target_type: string;
  target_id: string;
  display_name: string;
}

const typeLabels: Record<string, string> = {
  file: "File",
  folder: "Folder",
  app: "Application",
  drive: "Drive",
};

export function UnlockWidget() {
  const [target, setTarget] = useState<UnlockTarget | null>(null);
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    const unlisten = listen<UnlockTarget>("unlock-target", (event) => {
      setTarget(event.payload);
      setPassword("");
      setError("");
      setSuccess(false);
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const handleSubmit = async () => {
    if (!password) return;
    setLoading(true);
    setError("");
    try {
      await widgetUnlock(password);
      setSuccess(true);
      setTimeout(() => {
        setSuccess(false);
        setTarget(null);
        setPassword("");
      }, 2000);
    } catch (e: any) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleSubmit();
  };

  return (
    <div className="min-h-screen flex items-center justify-center" style={{ background: "transparent" }}>
      <div className="w-[400px] rounded-2xl overflow-hidden shadow-2xl"
           style={{ background: "oklch(0.16 0.005 270 / 0.95)", border: "1px solid oklch(1 0 0 / 0.08)" }}>
        <div data-tauri-drag-region
             className="h-10 w-full flex items-center justify-center cursor-move"
             style={{ background: "var(--gradient-brand)" }}>
          <Shield className="w-4 h-4 text-white mr-2" />
          <span className="text-xs font-medium text-white/90">OmniLock</span>
        </div>

        <div className="p-6">
          {target && !success ? (
            <>
              <div className="flex items-center gap-3 mb-5">
                <div className="w-10 h-10 rounded-xl grid place-items-center"
                     style={{ background: "color-mix(in oklab, var(--cyan) 15%, transparent)" }}>
                  <Lock className="w-5 h-5 text-[color:var(--primary)]" />
                </div>
                <div>
                  <div className="text-sm font-medium">Unlock {typeLabels[target.target_type] || target.target_type}</div>
                  <div className="text-xs text-[color:var(--muted-foreground)] truncate max-w-[260px]">{target.display_name}</div>
                </div>
              </div>

              <p className="text-xs text-[color:var(--muted-foreground)] mb-4">
                Enter your master password to access this item.
              </p>

              {error && (
                <div className="mb-3 p-2.5 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-xs text-[color:var(--destructive)]">
                  {error}
                </div>
              )}

              <input
                type="password"
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Master password"
                autoFocus
                className="w-full px-4 py-3 rounded-lg bg-white/[0.04] border border-white/10 text-sm outline-none focus:border-[color:var(--primary)]/50 mb-4 placeholder:text-[color:var(--muted-foreground)]"
              />

              <button
                onClick={handleSubmit}
                disabled={!password || loading}
                className="w-full px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                style={{ background: "var(--gradient-brand)" }}>
                {loading ? "Verifying..." : "Unlock"} <Lock className="w-4 h-4" />
              </button>
            </>
          ) : success ? (
            <div className="text-center py-4">
              <div className="w-12 h-12 mx-auto mb-3 rounded-full grid place-items-center" style={{ background: "color-mix(in oklab, var(--success) 15%, transparent)" }}>
                <Shield className="w-6 h-6 text-[color:var(--success)]" />
              </div>
              <div className="text-sm font-medium text-[color:var(--success)]">Unlocked Successfully</div>
              <div className="text-xs text-[color:var(--muted-foreground)] mt-1">You can now access this item.</div>
            </div>
          ) : (
            <div className="text-center py-6 text-sm text-[color:var(--muted-foreground)]">
              No locked items to unlock.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
