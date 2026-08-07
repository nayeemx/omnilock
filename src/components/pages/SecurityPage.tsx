import { useState, useEffect } from "react";
import {
  QrCode, KeyRound, Copy, ArrowRight, CheckCircle2,
  AlertTriangle, Power, Activity, ShieldAlert, Download, Upload,
  Usb, Github, Cloud, Fingerprint,
} from "lucide-react";
import {
  generateTotpSecret, generateTotpQr, enable2FA, disable2FA, setAutoLock,
  checkForUpdates, installUpdate, getRecoveryKey,
  listUsbDrives, enrollUsbKey, removeUsbKey, detectUsbKey,
  backupVault, restoreVault, pickDirectory,
  checkBiometric, toggleBiometric,
  type VaultConfigDto, type UsbDriveInfo,
} from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";
import { GitHubConnect } from "../auth/GitHubConnect";

type TwoFaState = "idle" | "setup" | "enabled";

export function SecurityPage({ config, refresh }: { config: VaultConfigDto | null; refresh: () => Promise<void> }) {
  const [twoFaState, setTwoFaState] = useState<TwoFaState>(
    config?.totp_enabled ? "enabled" : "idle"
  );
  const [totpSecret, setTotpSecret] = useState("");
  const [qrUri, setQrUri] = useState("");
  const [verifyCode, setVerifyCode] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [autoLockMins, setAutoLockMins] = useState(config?.auto_lock_minutes ?? 5);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState<{ version: string; notes?: string } | null>(null);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateError, setUpdateError] = useState("");
  const [recoveryKeyVisible, setRecoveryKeyVisible] = useState(false);
  const [recoveryKey, setRecoveryKey] = useState("");
  const [usbDrives, setUsbDrives] = useState<UsbDriveInfo[]>([]);
  const [usbSelectedDrive, setUsbSelectedDrive] = useState("");
  const [usbLoading, setUsbLoading] = useState(false);
  const [usbError, setUsbError] = useState("");
  const [biometricAvailable, setBiometricAvailable] = useState<boolean | null>(null);
  const [biometricLoading, setBiometricLoading] = useState(false);
  const [biometricPw, setBiometricPw] = useState("");

  useEffect(() => {
    checkBiometric().then(s => setBiometricAvailable(s.available)).catch(() => setBiometricAvailable(false));
  }, []);

  const handleToggleBiometric = async () => {
    if (config?.biometric_enabled) {
      setBiometricLoading(true);
      setError("");
      try {
        await toggleBiometric(false);
        await refresh();
      } catch (e: any) {
        setError("Failed to disable biometric: " + e);
      }
      setBiometricLoading(false);
    } else {
      if (!biometricPw) return;
      setBiometricLoading(true);
      setError("");
      try {
        await toggleBiometric(true, biometricPw);
        setBiometricPw("");
        await refresh();
      } catch (e: any) {
        setError("Failed to enable biometric: " + e);
      }
      setBiometricLoading(false);
    }
  };

  const start2faSetup = async () => {
    setTwoFaState("setup");
    setError("");
    setSuccess("");
    setVerifyCode("");
    try {
      const s = await generateTotpSecret();
      setTotpSecret(s);
      const qr = await generateTotpQr(s);
      setQrUri(qr);
    } catch (e: any) {
      setError("Failed to generate 2FA secret: " + e);
      setTwoFaState("idle");
    }
  };

  const handleEnable2FA = async () => {
    setLoading(true);
    setError("");
    try {
      await enable2FA(totpSecret, verifyCode);
      setTwoFaState("enabled");
      setSuccess("Two-factor authentication enabled successfully!");
      setTotpSecret("");
      setQrUri("");
      setVerifyCode("");
      await refresh();
    } catch (e: any) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleDisable2FA = async () => {
    if (!confirm("Disable two-factor authentication? Your vault will be less secure.")) return;
    setLoading(true);
    setError("");
    try {
      await disable2FA();
      setTwoFaState("idle");
      setSuccess("Two-factor authentication disabled.");
      await refresh();
    } catch (e: any) {
      setError(String(e));
    }
    setLoading(false);
  };

  const cancelSetup = () => {
    setTwoFaState("idle");
    setTotpSecret("");
    setQrUri("");
    setVerifyCode("");
    setError("");
  };

  const handleAutoLock = async (minutes: number) => {
    setAutoLockMins(minutes);
    try {
      await setAutoLock(minutes);
      await refresh();
    } catch (e: any) {
      setError("Failed to set auto-lock: " + e);
    }
  };

  const handleCheckUpdate = async () => {
    setUpdateChecking(true);
    setUpdateError("");
    setUpdateAvailable(null);
    try {
      const result = await checkForUpdates();
      if (result?.available) {
        setUpdateAvailable({ version: result.version || "Unknown", notes: result.notes });
      } else {
        setUpdateError("No updates available. You're running the latest version.");
      }
    } catch (e: any) {
      setUpdateError("Failed to check for updates: " + e);
    }
    setUpdateChecking(false);
  };

  const handleInstallUpdate = async () => {
    setUpdateInstalling(true);
    setUpdateError("");
    try {
      const result = await installUpdate();
      if (!result.success) {
        setUpdateError("Failed to install update: " + (result.error || "Unknown error"));
        setUpdateInstalling(false);
      }
    } catch (e: any) {
      setUpdateError("Failed to install update: " + e);
      setUpdateInstalling(false);
    }
  };

  const handleShowRecoveryKey = async () => {
    try {
      const key = await getRecoveryKey();
      setRecoveryKey(key);
      setRecoveryKeyVisible(true);
    } catch (e: any) {
      setError("Failed to load recovery key: " + e);
    }
  };

  const handleScanUsbDrives = async () => {
    setUsbError("");
    try {
      const drives = await listUsbDrives();
      setUsbDrives(drives);
      if (drives.length === 0) {
        setUsbError("No removable drives detected. Plug in your pendrive and try again.");
      }
    } catch (e: any) {
      setUsbError("Failed to scan USB drives: " + e);
    }
  };

  const handleEnrollUsb = async () => {
    if (!usbSelectedDrive) return;
    setUsbLoading(true);
    setUsbError("");
    try {
      await enrollUsbKey(usbSelectedDrive);
      setSuccess("USB key enrolled successfully! Your pendrive is now a hardware recovery key.");
      await refresh();
    } catch (e: any) {
      setUsbError(String(e));
    }
    setUsbLoading(false);
  };

  const handleRemoveUsbKey = async () => {
    if (!confirm("Remove USB key enrollment? The key file on your pendrive will remain but won't auto-unlock anymore.")) return;
    setUsbLoading(true);
    setUsbError("");
    try {
      await removeUsbKey();
      setSuccess("USB key enrollment removed.");
      await refresh();
    } catch (e: any) {
      setUsbError(String(e));
    }
    setUsbLoading(false);
  };

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <SectionHeader
        eyebrow="FR-AUTH · Argon2id + AES-256-GCM"
        title="Security & 2FA"
        subtitle="Zero-trust master credentials, RFC 6238 TOTP, and 3-tier recovery. Vault sealed with authenticated encryption."
      />

      <div className="grid lg:grid-cols-3 gap-5">
        <div className="glass rounded-2xl p-6 lg:col-span-2">
          <div className="flex items-center gap-3 mb-1">
            <div className="w-9 h-9 rounded-lg grid place-items-center" style={{ background: "color-mix(in oklab, var(--cyan) 15%, transparent)" }}>
              <QrCode className="w-4 h-4 text-[color:var(--primary)]" />
            </div>
            <h3 className="font-semibold">Two-Factor Authentication</h3>
            {twoFaState === "enabled" ? (
              <span className="ml-auto text-xs px-2 py-1 rounded-full bg-[color:var(--success)]/15 text-[color:var(--success)]">Enforced</span>
            ) : (
              <span className="ml-auto text-xs px-2 py-1 rounded-full bg-surface text-[color:var(--muted-foreground)] border border-surface-border">Disabled</span>
            )}
          </div>
          <p className="text-xs text-[color:var(--muted-foreground)] mb-5">Compatible with Google Authenticator, Authy, Microsoft Authenticator & 1Password.</p>

          {error && (
            <div className="mb-4 p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
              {error}
            </div>
          )}
          {success && (
            <div className="mb-4 p-3 rounded-lg bg-[color:var(--success)]/15 border border-[color:var(--success)]/30 text-sm text-[color:var(--success)]">
              {success}
            </div>
          )}

          {twoFaState === "enabled" && (
            <div className="flex items-center gap-4">
              <CheckCircle2 className="w-8 h-8 text-[color:var(--success)]" />
              <div>
                <div className="text-sm font-medium">2FA is active</div>
                <div className="text-xs text-[color:var(--muted-foreground)]">Your vault requires a TOTP code on every unlock.</div>
              </div>
              <button onClick={handleDisable2FA} disabled={loading}
                      className="ml-auto px-3 py-2 rounded-lg text-xs border border-[color:var(--destructive)]/30 text-[color:var(--destructive)] hover:bg-[color:var(--destructive)]/10 disabled:opacity-40">
                {loading ? "Disabling..." : "Disable 2FA"}
              </button>
            </div>
          )}

          {twoFaState === "setup" && (
            <div className="flex gap-6">
              <div className="w-40 h-40 rounded-xl bg-white p-3 shrink-0 grid place-items-center">
                {qrUri ? <img src={qrUri} alt="TOTP QR Code" className="w-full h-full" /> : <div className="text-xs text-gray-400">Generating QR...</div>}
              </div>
              <div className="flex-1 space-y-3">
                <div>
                  <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)]">Secret Key</div>
                  <div className="flex items-center gap-2 mt-1">
                    <code className="flex-1 px-3 py-2 rounded-lg bg-surface border border-surface-border text-xs tracking-wider break-all">{totpSecret}</code>
                    <button onClick={() => navigator.clipboard.writeText(totpSecret)} className="p-2 rounded-lg bg-surface border border-surface-border hover:bg-surface-active"><Copy className="w-4 h-4" /></button>
                  </div>
                </div>
                <div>
                  <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)]">Enter 6-digit code from your authenticator</div>
                  <div className="flex items-center gap-3 mt-1">
                    <input value={verifyCode} onChange={e => setVerifyCode(e.target.value.replace(/\D/g, ""))} placeholder="000000" maxLength={6}
                           className="text-3xl font-semibold tracking-[0.3em] bg-transparent outline-none w-40 placeholder:text-[color:var(--muted-foreground)]/30" />
                  </div>
                </div>
                <div className="flex gap-3">
                  <button onClick={cancelSetup}
                          className="px-4 py-2 rounded-lg text-sm bg-surface border border-surface-border hover:bg-surface-active">
                    Cancel
                  </button>
                  <button onClick={handleEnable2FA} disabled={!verifyCode || verifyCode.length !== 6 || loading}
                          className="px-4 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan disabled:opacity-40"
                          style={{ background: "var(--gradient-brand)" }}>
                    {loading ? "Verifying..." : "Verify & Enable 2FA"} <ArrowRight className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          )}

          {twoFaState === "idle" && (
            <button onClick={start2faSetup}
                    className="px-5 py-3 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan"
                    style={{ background: "var(--gradient-brand)" }}>
              <QrCode className="w-4 h-4" /> Enable Two-Factor Authentication
            </button>
          )}
        </div>

        <div className="glass rounded-2xl p-6">
          <div className="flex items-center gap-3 mb-4">
            <KeyRound className="w-5 h-5 text-[color:var(--violet)]" />
            <h3 className="font-semibold">Recovery Tiers</h3>
          </div>
          <ol className="space-y-3">
            {[
              { n: 1, label: "Security Q&A", meta: config?.security_question ? "Configured" : "Not set", ok: !!config?.security_question },
              { n: 2, label: "Recovery Key", meta: config?.recovery_key ? "Generated \u00b7 View to copy" : "Not generated", ok: !!config?.recovery_key },
              { n: 3, label: "USB Hardware Key", meta: config?.usb_key_enabled ? `Enrolled \u00b7 ${config.usb_key_drive_label}` : "Not enrolled", ok: !!config?.usb_key_enabled },
            ].map(r => (
              <li key={r.n} className="flex items-center gap-3 p-3 rounded-lg bg-surface border border-surface-border">
                <div className={`w-8 h-8 rounded-full grid place-items-center text-xs font-semibold ${r.ok ? "text-[color:var(--primary-foreground)]" : "text-[color:var(--muted-foreground)] bg-surface"}`}
                     style={r.ok ? { background: "var(--gradient-brand)" } : undefined}>
                  {r.n}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm">{r.label}</div>
                  <div className="text-xs text-[color:var(--muted-foreground)]">{r.meta}</div>
                </div>
                {r.ok ? <CheckCircle2 className="w-4 h-4 text-[color:var(--success)]" /> : <AlertTriangle className="w-4 h-4 text-[color:var(--warning)]" />}
              </li>
            ))}
          </ol>
          {config?.recovery_key && !recoveryKeyVisible && (
            <button onClick={handleShowRecoveryKey}
                    className="mt-4 w-full px-3 py-2 rounded-lg text-xs border border-[color:var(--violet)]/30 text-[color:var(--violet)] hover:bg-[color:var(--violet)]/10 transition">
              Show Recovery Key
            </button>
          )}
          {recoveryKeyVisible && (
            <div className="mt-4 space-y-2">
              <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)]">Your Recovery Key</div>
              <div className="flex items-center gap-2">
                <code className="flex-1 px-3 py-2 rounded-lg bg-surface border border-surface-border text-xs break-all font-mono">{recoveryKey}</code>
                <button onClick={() => navigator.clipboard.writeText(recoveryKey)} className="p-2 rounded-lg bg-surface border border-surface-border hover:bg-surface-active shrink-0"><Copy className="w-4 h-4" /></button>
              </div>
              <p className="text-[10px] text-[color:var(--warning)]">Save this key somewhere safe. It can recover your vault if you forget your password.</p>
            </div>
          )}
        </div>
      </div>

      <div className="grid md:grid-cols-2 gap-5">
        <div className="glass rounded-2xl p-6">
          <div className="flex items-center gap-3 mb-4">
            <Power className="w-5 h-5 text-[color:var(--warning)]" />
            <h3 className="font-semibold">Panic Hotkey</h3>
          </div>
          <p className="text-xs text-[color:var(--muted-foreground)] mb-4">Instantly mutes all audio and locks the Windows session. Press Win+Alt+L from anywhere.</p>
          <div className="flex items-center gap-2">
            {["Win", "Alt", "L"].map(k => (
              <kbd key={k} className="px-3 py-2 rounded-lg bg-surface border border-surface-border text-sm font-mono">{k}</kbd>
            ))}
          </div>
        </div>

        <div className="glass rounded-2xl p-6">
          <div className="flex items-center gap-3 mb-4">
            <Activity className="w-5 h-5 text-[color:var(--primary)]" />
            <h3 className="font-semibold">Session Auto-Lock</h3>
          </div>
          <div className="grid grid-cols-5 gap-2">
            {[
              { label: "Immediate", value: 0 },
              { label: "1 min", value: 1 },
              { label: "5 min", value: 5 },
              { label: "15 min", value: 15 },
              { label: "30 min", value: 30 },
            ].map(o => (
              <button key={o.value} onClick={() => handleAutoLock(o.value)}
                      className={`px-2 py-2 rounded-lg text-xs border transition ${
                        autoLockMins === o.value
                          ? "border-[color:var(--primary)]/50 bg-[color:var(--primary)]/10 text-[color:var(--primary)]"
                          : "border-surface-border bg-surface text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]"
                      }`}>
                {o.label}
              </button>
            ))}
          </div>
          <div className="mt-4 pt-4 border-t border-[color:var(--border)] text-xs text-[color:var(--muted-foreground)]">
            Vault stored at <code>%APPDATA%\InnologyBD\OmniLock\vault.enc</code>
          </div>
        </div>
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <Fingerprint className="w-5 h-5 text-[color:var(--primary)]" />
          <h3 className="font-semibold">Biometric Login (Fingerprint)</h3>
          {config?.biometric_enabled ? (
            <span className="ml-auto text-xs px-2 py-1 rounded-full bg-[color:var(--success)]/15 text-[color:var(--success)]">Enabled</span>
          ) : (
            <span className="ml-auto text-xs px-2 py-1 rounded-full bg-surface text-[color:var(--muted-foreground)] border border-surface-border">Disabled</span>
          )}
        </div>
        <p className="text-xs text-[color:var(--muted-foreground)] mb-4">
          Use your fingerprint, face, or PIN to unlock your vault via Windows Hello. No biometric data leaves your device.
        </p>
        {biometricAvailable === false && (
          <div className="mb-3 p-3 rounded-lg bg-[color:var(--warning)]/10 border border-[color:var(--warning)]/30 text-sm text-[color:var(--warning)]">
            Windows Hello is not available on this device. Set up a fingerprint or PIN in Windows Settings &gt; Accounts &gt; Sign-in options.
          </div>
        )}
        {biometricAvailable === true && (
          <div className="space-y-3">
            {!config?.biometric_enabled && (
              <div>
                <label className="text-xs text-[color:var(--muted-foreground)] mb-1 block">Enter master password to enable biometric login</label>
                <input type="password" value={biometricPw} onChange={e => setBiometricPw(e.target.value)}
                       placeholder="Master password"
                       className="w-full px-3 py-2 text-sm rounded-lg bg-surface border border-surface-border focus:border-[color:var(--primary)] focus:outline-none text-[color:var(--foreground)] placeholder:text-[color:var(--muted-foreground)]" />
              </div>
            )}
            <button onClick={handleToggleBiometric} disabled={biometricLoading || (!config?.biometric_enabled && !biometricPw)}
                    className={`px-4 py-2 rounded-lg text-sm font-medium flex items-center gap-2 disabled:opacity-40 transition ${
                      config?.biometric_enabled
                        ? "border border-[color:var(--destructive)]/30 text-[color:var(--destructive)] hover:bg-[color:var(--destructive)]/10"
                        : "text-[color:var(--primary-foreground)] glow-cyan"
                    }`}
                    style={config?.biometric_enabled ? undefined : { background: "var(--gradient-brand)" }}>
              <Fingerprint className="w-4 h-4" />
              {biometricLoading ? "Processing..." : config?.biometric_enabled ? "Disable Biometric Login" : "Enable Biometric Login"}
            </button>
          </div>
        )}
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <Usb className="w-5 h-5 text-[color:var(--primary)]" />
          <h3 className="font-semibold">USB Hardware Key</h3>
          {config?.usb_key_enabled ? (
            <span className="ml-auto text-xs px-2 py-1 rounded-full bg-[color:var(--success)]/15 text-[color:var(--success)]">Enrolled</span>
          ) : (
            <span className="ml-auto text-xs px-2 py-1 rounded-full bg-surface text-[color:var(--muted-foreground)] border border-surface-border">Not enrolled</span>
          )}
        </div>
        <p className="text-xs text-[color:var(--muted-foreground)] mb-4">
          Store your recovery key on a USB pendrive. When plugged in, OmniLock can auto-unlock using the key. The key never touches your hard disk.
        </p>

        {usbError && (
          <div className="mb-3 p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
            {usbError}
          </div>
        )}

        {config?.usb_key_enabled ? (
          <div className="space-y-3">
            <div className="flex items-center gap-3 p-3 rounded-lg bg-[color:var(--success)]/10 border border-[color:var(--success)]/20">
              <CheckCircle2 className="w-5 h-5 text-[color:var(--success)] shrink-0" />
              <div>
                <div className="text-sm font-medium">USB key enrolled</div>
                <div className="text-xs text-[color:var(--muted-foreground)]">Drive: {config.usb_key_drive_label}</div>
              </div>
            </div>
            <div className="flex gap-2">
              <button onClick={async () => {
                setUsbLoading(true);
                setUsbError("");
                try {
                  const key = await detectUsbKey();
                  if (key) {
                    setSuccess("USB key detected and verified! Recovery key can be read from this pendrive.");
                  } else {
                    setUsbError("No OmniLock USB key detected. Make sure your enrolled pendrive is plugged in.");
                  }
                } catch (e: any) {
                  setUsbError("Test failed: " + e);
                }
                setUsbLoading(false);
              }} disabled={usbLoading}
                      className="px-3 py-2 rounded-lg text-xs bg-[color:var(--success)]/15 border border-[color:var(--success)]/30 text-[color:var(--success)] hover:bg-[color:var(--success)]/25 disabled:opacity-40 flex items-center gap-1">
                <Usb className="w-3 h-3" /> {usbLoading ? "Testing..." : "Test USB Key"}
              </button>
              <button onClick={handleRemoveUsbKey} disabled={usbLoading}
                      className="px-3 py-2 rounded-lg text-xs border border-[color:var(--destructive)]/30 text-[color:var(--destructive)] hover:bg-[color:var(--destructive)]/10 disabled:opacity-40">
                {usbLoading ? "Removing..." : "Remove USB Key Enrollment"}
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            <button onClick={handleScanUsbDrives}
                    className="px-4 py-2 rounded-lg text-sm bg-surface border border-surface-border hover:bg-surface-active flex items-center gap-2">
              <Usb className="w-4 h-4" /> Scan for Pendrives
            </button>
            {usbDrives.length > 0 && (
              <>
                <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)]">Select your pendrive</div>
                <div className="space-y-2">
                  {usbDrives.map(d => (
                    <button key={d.letter} onClick={() => setUsbSelectedDrive(d.letter)}
                            className={`w-full flex items-center gap-3 p-3 rounded-lg border transition ${
                              usbSelectedDrive === d.letter
                                ? "border-[color:var(--primary)]/50 bg-[color:var(--primary)]/10"
                                : "border-surface-border bg-surface hover:bg-surface-hover"
                            }`}>
                      <Usb className="w-4 h-4 text-[color:var(--primary)]" />
                      <div className="text-left">
                        <div className="text-sm">{d.label || `Drive ${d.letter}:`}</div>
                        <div className="text-xs text-[color:var(--muted-foreground)]">{d.letter}: drive \u00b7 serial {d.serial}</div>
                      </div>
                    </button>
                  ))}
                </div>
                <button onClick={handleEnrollUsb} disabled={!usbSelectedDrive || usbLoading}
                        className="w-full px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                        style={{ background: "var(--gradient-brand)" }}>
                  {usbLoading ? "Enrolling..." : "Write Key to Pendrive"} <ArrowRight className="w-4 h-4" />
                </button>
              </>
            )}
          </div>
        )}
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <Cloud className="w-5 h-5 text-[color:var(--primary)]" />
          <h3 className="font-semibold">Cloud Sync</h3>
          {config?.cloud_sync_enabled ? (
            <span className="ml-auto text-xs px-2 py-1 rounded-full bg-[color:var(--success)]/15 text-[color:var(--success)]">Active</span>
          ) : (
            <span className="ml-auto text-xs px-2 py-1 rounded-full bg-surface text-[color:var(--muted-foreground)] border border-surface-border">Not connected</span>
          )}
        </div>
        <p className="text-xs text-[color:var(--muted-foreground)] mb-4">
          Backup your encrypted vault to GitHub Gists. Restore your settings on any computer by connecting your GitHub account.
          Your vault is encrypted end-to-end with your master password - GitHub never sees your data.
        </p>
        <GitHubConnect onStatusChange={() => refresh()} />
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <Download className="w-5 h-5 text-[color:var(--success)]" />
          <h3 className="font-semibold">Updates</h3>
        </div>
        <p className="text-xs text-[color:var(--muted-foreground)] mb-4">Check for and install updates from GitHub Releases.</p>
        
        {updateError && (
          <div className="mb-3 p-3 rounded-lg bg-[color:var(--muted)]/20 border border-[color:var(--border)] text-sm text-[color:var(--muted-foreground)]">
            {updateError}
          </div>
        )}
        
        {updateAvailable ? (
          <div className="space-y-3">
            <div className="p-3 rounded-lg bg-[color:var(--success)]/10 border border-[color:var(--success)]/30">
              <div className="text-sm font-medium text-[color:var(--success)]">Update available: v{updateAvailable.version}</div>
              {updateAvailable.notes && (
                <div className="text-xs text-[color:var(--muted-foreground)] mt-1">{updateAvailable.notes}</div>
              )}
            </div>
            <button onClick={handleInstallUpdate} disabled={updateInstalling}
                    className="px-4 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan disabled:opacity-40"
                    style={{ background: "var(--gradient-brand)" }}>
              {updateInstalling ? "Downloading & restarting..." : "Install Update & Restart"}
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        ) : (
          <button onClick={handleCheckUpdate} disabled={updateChecking}
                  className="px-4 py-2 rounded-lg text-sm font-medium bg-surface border border-surface-border hover:bg-surface-active disabled:opacity-40 flex items-center gap-2">
            {updateChecking ? "Checking..." : "Check for Updates"}
            <Download className="w-4 h-4" />
          </button>
        )}
      </div>

      <div className="glass rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <Download className="w-5 h-5 text-[color:var(--primary)]" />
          <h3 className="font-semibold">Backup & Restore</h3>
        </div>
        <p className="text-xs text-[color:var(--muted-foreground)] mb-4">
          Export your encrypted vault to a safe location. If you reinstall OmniLock, import the backup to restore all your settings, locked apps, folders, and recovery keys.
        </p>
        <div className="flex gap-3">
          <button onClick={async () => {
                    const dir = await pickDirectory("Choose Backup Location");
                    if (dir) {
                      try {
                        const msg = await backupVault(dir);
                        setSuccess(msg);
                      } catch (e: any) {
                        setError("Backup failed: " + e);
                      }
                    }
                  }}
                  className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium bg-surface border border-surface-border hover:bg-surface-active flex items-center justify-center gap-2">
            <Download className="w-4 h-4" /> Export Backup
          </button>
          <button onClick={async () => {
                    const dir = await pickDirectory("Select Backup Folder to Restore");
                    if (dir) {
                      try {
                        const msg = await restoreVault(dir);
                        setSuccess(msg);
                      } catch (e: any) {
                        setError("Restore failed: " + e);
                      }
                    }
                  }}
                  className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium bg-surface border border-surface-border hover:bg-surface-active flex items-center justify-center gap-2">
            <Upload className="w-4 h-4" /> Import Backup
          </button>
        </div>
      </div>
    </div>
  );
}
