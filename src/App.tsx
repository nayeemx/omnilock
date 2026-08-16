import { useState, useEffect, useCallback } from "react";
import { Github, ArrowRight, Loader2, ShieldAlert } from "lucide-react";
import {
  getVaultStatus, getVaultConfig, lockNow,
  githubGetStatus, githubStartDeviceFlow, githubPollToken, openExternalUrl,
  hasBiometricToken,
  verifyLockedState, relockEntries, forgetUnlockedEntries,
  type VaultStatusDto, type VaultConfigDto, type GitHubSyncStatusDto, type UnlockTarget,
} from "./lib/tauri-bridge";
import { SetupWizard } from "./components/auth/SetupWizard";
import { LoginScreen } from "./components/auth/LoginScreen";
import { Sidebar } from "./components/layout/Sidebar";
import { TopBar } from "./components/layout/TopBar";
import { Footer } from "./components/layout/Footer";
import { AppLockerPage } from "./components/pages/AppLockerPage";
import { PresetsPage } from "./components/pages/PresetsPage";
import { VaultPage } from "./components/pages/VaultPage";
import { SecurityPage } from "./components/pages/SecurityPage";
import { SystemMonitorPage } from "./components/pages/SystemMonitorPage";
import { DashboardPage } from "./components/pages/DashboardPage";
import { DiagnosticsPage } from "./components/pages/DiagnosticsPage";
import { HistoryPage } from "./components/pages/HistoryPage";
import { UnlockWidget } from "./components/widget/UnlockWidget";
import { type TabId } from "./components/types";

function isWidgetMode(): boolean {
  return new URLSearchParams(window.location.search).has("widget");
}

export default function App() {
  const [widgetMode] = useState(isWidgetMode);
  const [vaultStatus, setVaultStatus] = useState<VaultStatusDto | null>(null);
  const [vaultConfig, setVaultConfig] = useState<VaultConfigDto | null>(null);
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [activeTab, setActiveTab] = useState<TabId>("dashboard");
  const [githubStatus, setGithubStatus] = useState<GitHubSyncStatusDto | null>(null);
  const [githubStep, setGithubStep] = useState<"idle" | "code" | "polling">("idle");
  const [githubUserCode, setGithubUserCode] = useState("");
  const [githubCopied, setGithubCopied] = useState(false);
  const [githubError, setGithubError] = useState("");
  const [showSkipGithub, setShowSkipGithub] = useState(false);
  const [hasBiometric, setHasBiometric] = useState(false);
  const [staleIssues, setStaleIssues] = useState<UnlockTarget[] | null>(null);
  const [resolvingStale, setResolvingStale] = useState(false);

  useEffect(() => {
    if (!widgetMode) {
      getVaultStatus().then(setVaultStatus).catch(console.error);
      githubGetStatus().then(setGithubStatus).catch(console.error);
      hasBiometricToken().then(setHasBiometric).catch(console.error);
    }
  }, [widgetMode]);

  const refreshConfig = useCallback(async () => {
    try {
      const config = await getVaultConfig();
      setVaultConfig(config);
    } catch (e) {
      console.error("Failed to fetch config:", e);
    }
  }, []);

  const handleUnlock = useCallback(async () => {
    try {
      const status = await getVaultStatus();
      setVaultStatus(status);
      if (status.initialized) {
        await refreshConfig();
        setIsUnlocked(true);
        // Entries may be marked locked in the vault but no longer locked on disk
        // (widget temp-unlock that was never re-locked, e.g. app closed while the
        // item was open). Prompt the user to re-lock or keep them unlocked.
        try {
          const issues = await verifyLockedState();
          if (issues.length > 0) {
            setStaleIssues(issues);
          }
        } catch {
          // verification is best-effort
        }
      }
    } catch (e: any) {
      console.error(e);
    }
  }, [refreshConfig]);

  const handleRelockAll = async () => {
    if (!staleIssues) return;
    setResolvingStale(true);
    try {
      const results = await relockEntries(staleIssues.map(i => i.target_id));
      const failed = results.filter(([, s]) => s !== "ok" && s !== "already_locked");
      if (failed.length > 0) {
        setStaleIssues(staleIssues.filter(i => failed.some(([p]) => p === i.target_id)));
      } else {
        setStaleIssues(null);
      }
      await refreshConfig();
    } catch (e: any) {
      console.error("relock failed:", e);
    }
    setResolvingStale(false);
  };

  const handleKeepUnlocked = async () => {
    if (!staleIssues) return;
    setResolvingStale(true);
    try {
      await forgetUnlockedEntries(staleIssues.map(i => i.target_id));
      setStaleIssues(null);
      await refreshConfig();
    } catch (e: any) {
      console.error("forget failed:", e);
    }
    setResolvingStale(false);
  };

  const handleLockNow = useCallback(async () => {
    try {
      await lockNow();
    } catch (e) {
      console.error("lockNow failed:", e);
    }
    const status = await getVaultStatus();
    setVaultStatus(status);
    setIsUnlocked(false);
    setVaultConfig(null);
  }, []);

  const handleGitHubConnect = async () => {
    setGithubError("");
    setGithubStep("code");
    try {
      const flow = await githubStartDeviceFlow();
      setGithubUserCode(flow.user_code);
      openExternalUrl(flow.verification_uri);

      const result = await githubPollToken(flow.device_code, flow.interval, flow.expires_in);
      setGithubStatus(result);
      setGithubStep("idle");
    } catch (e: any) {
      setGithubError(String(e));
      setGithubStep("idle");
    }
  };

  const copyGithubCode = () => {
    navigator.clipboard.writeText(githubUserCode);
    setGithubCopied(true);
    setTimeout(() => setGithubCopied(false), 2000);
  };

  if (widgetMode) {
    return <UnlockWidget />;
  }

  if (!vaultStatus) {
    return (
      <div className="flex items-center justify-center h-screen" style={{ background: "var(--background)" }}>
        <div className="text-center">
          <img src="/icon.png" alt="OmniLock" className="w-12 h-12 mx-auto mb-4 rounded-xl object-cover glow-cyan" />
          <div className="text-sm text-[color:var(--muted-foreground)] animate-pulse">Loading OmniLock...</div>
        </div>
      </div>
    );
  }

  if (!vaultStatus.initialized) {
    const showGithubPrompt = !showSkipGithub && githubStatus !== null && !githubStatus.connected;
    
    if (showGithubPrompt) {
      return (
        <div className="min-h-screen flex items-center justify-center p-6" style={{ background: "var(--background)" }}>
          <div className="w-full max-w-md">
            <div className="flex items-center gap-3 mb-8">
              <img src="/icon.png" alt="OmniLock" className="w-11 h-11 rounded-xl object-cover glow-cyan" />
              <div>
                <div className="text-lg font-semibold tracking-tight">OmniLock</div>
                <div className="text-[11px] text-[color:var(--muted-foreground)] tracking-wider uppercase">by InnologyBD</div>
              </div>
            </div>

            <div className="mb-6">
              <div className="text-[11px] uppercase tracking-widest text-[color:var(--primary)] mb-2">Step 1 of 3</div>
              <h2 className="text-3xl font-semibold tracking-tight">Connect GitHub</h2>
              <p className="text-sm text-[color:var(--muted-foreground)] mt-2">
                Link your GitHub account to backup and restore your vault across computers. Your data is encrypted end-to-end.
              </p>
            </div>

            {githubError && (
              <div className="mb-4 p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
                {githubError}
              </div>
            )}

            <div className="glass rounded-2xl p-6 space-y-4">
              {githubStep === "code" ? (
                <div className="space-y-4">
                  <div className="p-4 rounded-lg bg-surface border border-surface-border text-center">
                    <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)] mb-2">
                      Enter this code on GitHub
                    </div>
                    <div className="flex items-center justify-center gap-3">
                      <span className="text-3xl font-mono font-bold tracking-wider text-[color:var(--primary)]">
                        {githubUserCode}
                      </span>
                      <button onClick={copyGithubCode}
                              className="p-2 rounded-lg bg-surface hover:bg-surface-hover transition-colors">
                        {githubCopied ? (
                          <span className="text-[color:var(--success)] text-xs">Copied!</span>
                        ) : (
                          <span className="text-xs text-[color:var(--muted-foreground)]">Copy</span>
                        )}
                      </button>
                    </div>
                  </div>
                  <div className="flex items-center justify-center gap-2 text-sm text-[color:var(--muted-foreground)]">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    Waiting for authorization...
                  </div>
                  <button onClick={() => { setGithubStep("idle"); setGithubError(""); }}
                          className="w-full text-center text-sm text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]">
                    Cancel
                  </button>
                </div>
              ) : (
                <button onClick={handleGitHubConnect}
                        className="w-full flex items-center justify-center gap-3 px-4 py-3 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] glow-cyan"
                        style={{ background: "var(--gradient-brand)" }}>
                  <Github className="w-5 h-5" />
                  Connect with GitHub
                </button>
              )}
            </div>

            <button onClick={() => setShowSkipGithub(true)}
                    className="mt-4 w-full text-center text-sm text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]">
              Skip for now
            </button>

            <div className="mt-6 text-center text-[11px] text-[color:var(--muted-foreground)]">
              Uses GitHub Device Flow. No password shared with OmniLock.
            </div>
          </div>
        </div>
      );
    }

    return <SetupWizard onComplete={handleUnlock} />;
  }

  if (!isUnlocked) {
    return <LoginScreen totpEnabled={vaultStatus.totp_enabled} biometricEnabled={hasBiometric} onUnlock={handleUnlock} />;
  }

  if (staleIssues && staleIssues.length > 0) {
    return (
      <div className="min-h-screen flex items-center justify-center p-6" style={{ background: "var(--background)" }}>
        <div className="w-full max-w-md">
          <div className="flex items-center gap-3 mb-6">
            <img src="/icon.png" alt="OmniLock" className="w-11 h-11 rounded-xl object-cover glow-cyan" />
            <div>
              <div className="text-lg font-semibold tracking-tight">OmniLock</div>
              <div className="text-[11px] text-[color:var(--muted-foreground)] tracking-wider uppercase">by InnologyBD</div>
            </div>
          </div>
          <div className="glass rounded-2xl p-6 space-y-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg grid place-items-center bg-[color:var(--warning)]/15 border border-[color:var(--warning)]/30">
                <ShieldAlert className="w-5 h-5 text-[color:var(--warning)]" />
              </div>
              <div>
                <h2 className="font-semibold tracking-tight">Unlocked items detected</h2>
                <p className="text-xs text-[color:var(--muted-foreground)] mt-0.5">
                  {staleIssues.length} item{staleIssues.length > 1 ? "s" : ""} marked locked in the vault but not locked on disk.
                </p>
              </div>
            </div>
            <p className="text-sm text-[color:var(--muted-foreground)]">
              These were temporarily unlocked and never re-locked (the app was closed while they were open).
              Re-lock them now, or keep them unlocked and remove them from the vault.
            </p>
            <div className="max-h-40 overflow-auto rounded-lg bg-surface border border-surface-border divide-y divide-surface-border">
              {staleIssues.map(i => (
                <div key={i.target_type + i.target_id} className="px-3 py-2 text-xs flex items-center gap-2">
                  <span className="uppercase text-[9px] px-1.5 py-0.5 rounded bg-surface-active text-[color:var(--muted-foreground)]">{i.target_type}</span>
                  <code className="truncate">{i.target_id}</code>
                </div>
              ))}
            </div>
            <div className="flex gap-3">
              <button onClick={handleRelockAll} disabled={resolvingStale}
                      className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] glow-cyan disabled:opacity-40"
                      style={{ background: "var(--gradient-brand)" }}>
                {resolvingStale ? <Loader2 className="w-4 h-4 animate-spin" /> : null}
                Re-lock all
              </button>
              <button onClick={handleKeepUnlocked} disabled={resolvingStale}
                      className="flex-1 px-4 py-2.5 rounded-lg text-sm bg-surface border border-surface-border hover:bg-surface-active disabled:opacity-40">
                Keep unlocked
              </button>
            </div>
            {resolvingStale && <div className="text-xs text-[color:var(--muted-foreground)]">Applying...</div>}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex text-[color:var(--foreground)]">
      <Sidebar tab={activeTab} setTab={setActiveTab} config={vaultConfig} onLogout={handleLockNow} />
      <div className="flex-1 flex flex-col min-w-0">
        <TopBar onLockNow={handleLockNow} totpEnabled={vaultConfig?.totp_enabled} />
        <main className="flex-1 p-8 overflow-auto">
          {activeTab === "dashboard" && <DashboardPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "monitor" && <SystemMonitorPage />}
          {activeTab === "apps" && <AppLockerPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "presets" && <PresetsPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "vault" && <VaultPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "security" && <SecurityPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "history" && <HistoryPage />}
          {activeTab === "diagnostics" && <DiagnosticsPage />}
        </main>
        <Footer version={vaultStatus?.version} />
      </div>
    </div>
  );
}
