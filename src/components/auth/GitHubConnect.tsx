import { useState, useEffect, useCallback } from "react";
import { Github, Copy, Check, ExternalLink, Loader2, Unlink, Cloud, CloudOff, RefreshCw } from "lucide-react";
import {
  githubStartDeviceFlow,
  githubPollToken,
  githubGetStatus,
  githubDisconnect,
  githubSyncToCloud,
  githubSyncFromCloud,
  openExternalUrl,
  type GitHubSyncStatusDto,
  type GitHubDeviceFlowDto,
} from "../../lib/tauri-bridge";

type FlowStep = "idle" | "device_code" | "polling" | "connected" | "error";

export function GitHubConnect({ onStatusChange }: { onStatusChange?: (connected: boolean) => void }) {
  const [status, setStatus] = useState<GitHubSyncStatusDto | null>(null);
  const [step, setStep] = useState<FlowStep>("idle");
  const [deviceFlow, setDeviceFlow] = useState<GitHubDeviceFlowDto | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  const [syncing, setSyncing] = useState(false);
  const [lastSyncTime, setLastSyncTime] = useState<string>("");

  const refreshStatus = useCallback(async () => {
    try {
      const s = await githubGetStatus();
      setStatus(s);
      if (s.last_sync) {
        const date = new Date(s.last_sync * 1000);
        setLastSyncTime(date.toLocaleString());
      }
      onStatusChange?.(s.connected);
    } catch (e) {
      console.error(e);
    }
  }, [onStatusChange]);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  const startDeviceFlow = async () => {
    setError("");
    setStep("device_code");
    try {
      const flow = await githubStartDeviceFlow();
      setDeviceFlow(flow);
      setStep("polling");

      openExternalUrl(flow.verification_uri);

      pollForCompletion(flow);
    } catch (e: any) {
      setError(String(e));
      setStep("idle");
    }
  };

  const pollForCompletion = async (flow: GitHubDeviceFlowDto) => {
    try {
      const result = await githubPollToken(flow.device_code, flow.interval, flow.expires_in);
      setStatus(result);
      setStep("connected");
      onStatusChange?.(result.connected);
    } catch (e: any) {
      if (String(e).includes("expired")) {
        setError("Code expired. Please try again.");
        setStep("idle");
      } else if (String(e).includes("denied")) {
        setError("Authorization denied.");
        setStep("idle");
      } else {
        setError(String(e));
        setStep("idle");
      }
    }
  };

  const handleDisconnect = async () => {
    try {
      await githubDisconnect();
      setStatus(null);
      setStep("idle");
      onStatusChange?.(false);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSyncToCloud = async () => {
    setSyncing(true);
    setError("");
    try {
      const result = await githubSyncToCloud();
      setStatus(result);
      if (result.last_sync) {
        const date = new Date(result.last_sync * 1000);
        setLastSyncTime(date.toLocaleString());
      }
    } catch (e: any) {
      setError(String(e));
    } finally {
      setSyncing(false);
    }
  };

  const handleSyncFromCloud = async () => {
    setSyncing(true);
    setError("");
    try {
      await githubSyncFromCloud();
      await refreshStatus();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setSyncing(false);
    }
  };

  const copyUserCode = () => {
    if (deviceFlow?.user_code) {
      navigator.clipboard.writeText(deviceFlow.user_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (status?.connected) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-white/10 flex items-center justify-center overflow-hidden">
              {status.avatar_url ? (
                <img src={status.avatar_url} alt="" className="w-full h-full object-cover" />
              ) : (
                <Github className="w-5 h-5 text-white/70" />
              )}
            </div>
            <div>
              <div className="text-sm font-medium">{status.github_user}</div>
              <div className="text-[11px] text-[color:var(--muted-foreground)]">
                {lastSyncTime ? `Last sync: ${lastSyncTime}` : "Not synced yet"}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1.5 text-[11px] px-2 py-1 rounded-full bg-[color:var(--success)]/15 text-[color:var(--success)]">
              <Cloud className="w-3 h-3" /> Connected
            </span>
          </div>
        </div>

        {error && (
          <div className="p-2 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-xs text-[color:var(--destructive)]">
            {error}
          </div>
        )}

        <div className="flex gap-2">
          <button
            onClick={handleSyncToCloud}
            disabled={syncing}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-xs font-medium bg-[color:var(--primary)]/15 text-[color:var(--primary)] hover:bg-[color:var(--primary)]/25 transition-colors disabled:opacity-50"
          >
            {syncing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Cloud className="w-3.5 h-3.5" />}
            Backup to Cloud
          </button>
          <button
            onClick={handleSyncFromCloud}
            disabled={syncing}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-xs font-medium bg-white/[0.05] border border-white/10 hover:bg-white/[0.08] transition-colors disabled:opacity-50"
          >
            {syncing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <RefreshCw className="w-3.5 h-3.5" />}
            Restore from Cloud
          </button>
        </div>

        <button
          onClick={handleDisconnect}
          className="flex items-center gap-1.5 text-[11px] text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)] transition-colors"
        >
          <Unlink className="w-3 h-3" /> Disconnect GitHub
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="text-center py-4">
        <div className="w-12 h-12 mx-auto mb-3 rounded-xl bg-white/[0.06] border border-white/10 grid place-items-center">
          <Github className="w-6 h-6 text-white/70" />
        </div>
        <h3 className="text-sm font-medium mb-1">Connect GitHub for Cloud Sync</h3>
        <p className="text-[11px] text-[color:var(--muted-foreground)] max-w-[280px] mx-auto">
          Link your GitHub account to backup and restore your vault across computers. Your data is encrypted end-to-end.
        </p>
      </div>

      {error && (
        <div className="p-2 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-xs text-[color:var(--destructive)]">
          {error}
        </div>
      )}

      {step === "polling" && deviceFlow ? (
        <div className="space-y-3">
          <div className="p-3 rounded-lg bg-white/[0.04] border border-white/10 text-center">
            <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)] mb-2">
              Enter this code on GitHub
            </div>
            <div className="flex items-center justify-center gap-2">
              <span className="text-2xl font-mono font-bold tracking-wider text-[color:var(--primary)]">
                {deviceFlow.user_code}
              </span>
              <button
                onClick={copyUserCode}
                className="p-1.5 rounded-md bg-white/[0.06] hover:bg-white/[0.1] transition-colors"
              >
                {copied ? <Check className="w-4 h-4 text-[color:var(--success)]" /> : <Copy className="w-4 h-4" />}
              </button>
            </div>
          </div>

          <div className="flex items-center justify-center gap-2 text-[11px] text-[color:var(--muted-foreground)]">
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
            Waiting for authorization...
          </div>

          <button
            onClick={() => { setStep("idle"); setError(""); }}
            className="w-full text-center text-[11px] text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]"
          >
            Cancel
          </button>
        </div>
      ) : (
        <button
          onClick={startDeviceFlow}
          disabled={step === "device_code"}
          className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] glow-cyan disabled:opacity-40"
          style={{ background: "var(--gradient-brand)" }}
        >
          {step === "device_code" ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Github className="w-4 h-4" />
          )}
          {step === "device_code" ? "Connecting..." : "Connect with GitHub"}
        </button>
      )}

      <div className="text-[10px] text-center text-[color:var(--muted-foreground)]">
        Uses GitHub Device Flow. No password shared with OmniLock.
      </div>
    </div>
  );
}
