import { useState, useEffect, useCallback } from "react";
import { Shield } from "lucide-react";
import {
  getVaultStatus, getVaultConfig, triggerPanicLock,
  type VaultStatusDto, type VaultConfigDto,
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
import { type TabId } from "./components/types";

export default function App() {
  const [vaultStatus, setVaultStatus] = useState<VaultStatusDto | null>(null);
  const [vaultConfig, setVaultConfig] = useState<VaultConfigDto | null>(null);
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [activeTab, setActiveTab] = useState<TabId>("apps");

  useEffect(() => {
    getVaultStatus().then(setVaultStatus).catch(console.error);
  }, []);

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
      }
    } catch (e: any) {
      console.error(e);
    }
  }, [refreshConfig]);

  const handleLockNow = useCallback(async () => {
    try {
      await triggerPanicLock();
    } catch {}
    const status = await getVaultStatus();
    setVaultStatus(status);
    setIsUnlocked(false);
    setVaultConfig(null);
  }, []);

  if (!vaultStatus) {
    return (
      <div className="flex items-center justify-center h-screen" style={{ background: "var(--background)" }}>
        <div className="text-center">
          <div className="w-12 h-12 mx-auto mb-4 rounded-xl grid place-items-center glow-cyan" style={{ background: "var(--gradient-brand)" }}>
            <Shield className="w-6 h-6 text-primary-foreground" strokeWidth={2.5} />
          </div>
          <div className="text-sm text-[color:var(--muted-foreground)] animate-pulse">Loading OmniLock...</div>
        </div>
      </div>
    );
  }

  if (!vaultStatus.initialized) {
    return <SetupWizard onComplete={handleUnlock} />;
  }

  if (!isUnlocked) {
    return <LoginScreen totpEnabled={vaultStatus.totp_enabled} onUnlock={handleUnlock} />;
  }

  return (
    <div className="min-h-screen flex text-[color:var(--foreground)]">
      <Sidebar tab={activeTab} setTab={setActiveTab} config={vaultConfig} />
      <div className="flex-1 flex flex-col min-w-0">
        <TopBar onLockNow={handleLockNow} totpEnabled={vaultConfig?.totp_enabled} />
        <main className="flex-1 p-8 overflow-auto">
          {activeTab === "apps" && <AppLockerPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "presets" && <PresetsPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "vault" && <VaultPage config={vaultConfig} refresh={refreshConfig} />}
          {activeTab === "security" && <SecurityPage config={vaultConfig} refresh={refreshConfig} />}
        </main>
        <Footer version={vaultStatus?.version} />
      </div>
    </div>
  );
}
