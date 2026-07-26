import { useState, useEffect } from "react";
import { Shield } from "lucide-react";
import { getSystemInfo, type SystemInfoDto } from "../../lib/tauri-bridge";

export function Footer({ version }: { version?: string }) {
  const [sysInfo, setSysInfo] = useState<SystemInfoDto | null>(null);

  useEffect(() => {
    getSystemInfo().then(setSysInfo).catch(() => {});
  }, []);

  return (
    <footer className="px-8 py-4 border-t border-[color:var(--border)] glass-subtle flex items-center justify-between text-xs text-[color:var(--muted-foreground)]">
      <div className="flex items-center gap-2">
        <Shield className="w-3.5 h-3.5 text-[color:var(--primary)]" />
        OmniLock Security System · Developed by <span className="text-[color:var(--foreground)]">InnologyBD</span>
      </div>
      <div className="flex items-center gap-4">
        <span>v{version || "?"}</span>
        <span>{sysInfo ? `${sysInfo.os} · ${sysInfo.arch}` : "..."}</span>
      </div>
    </footer>
  );
}
