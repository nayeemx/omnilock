import { useState, useEffect } from "react";
import {
  ShieldCheck, Fingerprint, Activity,
  Lock, Eye, EyeOff, ArrowRight, CheckCircle2, HelpCircle, KeyRound,
  Usb, FileKey, Github, Cloud,
} from "lucide-react";
import {
  unlockSession, getSecurityQuestion, resetPassword,
  recoverWithKey, recoverWithUsbKey,
  githubStartDeviceFlow, githubPollToken, openExternalUrl,
  checkBiometric, authenticateBiometric, biometricLogin,
} from "../../lib/tauri-bridge";
import { Field } from "../shared/Field";

type ResetMode = "select" | "question" | "recovery_key" | "usb_key";

export function LoginScreen({ totpEnabled, biometricEnabled, onUnlock }: { totpEnabled: boolean; biometricEnabled: boolean; onUnlock: () => void }) {
  const [password, setPassword] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [showPw, setShowPw] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [biometricLoading, setBiometricLoading] = useState(false);
  const [biometricAvailable, setBiometricAvailable] = useState<boolean | null>(null);

  useEffect(() => {
    checkBiometric().then(s => setBiometricAvailable(s.available)).catch(() => setBiometricAvailable(false));
  }, []);

  const handleBiometricLogin = async () => {
    setBiometricLoading(true);
    setError("");
    try {
      await authenticateBiometric("Verify identity to unlock OmniLock");
      await biometricLogin();
      await onUnlock();
    } catch (e: any) {
      setError(String(e));
    }
    setBiometricLoading(false);
  };

  const [githubLoading, setGithubLoading] = useState(false);
  const [githubStep, setGithubStep] = useState<"idle" | "code" | "polling">("idle");
  const [githubUserCode, setGithubUserCode] = useState("");
  const [githubCopied, setGithubCopied] = useState(false);

  const [resetMode, setResetMode] = useState<ResetMode | null>(null);
  const [resetQuestion, setResetQuestion] = useState("");
  const [resetAnswer, setResetAnswer] = useState("");
  const [resetRecoveryKey, setResetRecoveryKey] = useState("");
  const [resetNewPw, setResetNewPw] = useState("");
  const [resetConfirmPw, setResetConfirmPw] = useState("");
  const [resetLoading, setResetLoading] = useState(false);
  const [resetError, setResetError] = useState("");
  const [resetSuccess, setResetSuccess] = useState(false);

  const handleResetQuestion = async () => {
    setResetError("");
    setResetLoading(true);
    try {
      const q = await getSecurityQuestion();
      setResetQuestion(q);
      setResetMode("question");
    } catch (e: any) {
      setResetError(e);
    } finally {
      setResetLoading(false);
    }
  };

  const handleResetSubmit = async () => {
    setResetError("");
    if (!resetNewPw || !resetAnswer) return;
    if (resetNewPw !== resetConfirmPw) {
      setResetError("Passwords do not match");
      return;
    }
    if (resetNewPw.length < 8) {
      setResetError("Password must be at least 8 characters");
      return;
    }
    const hasUpper = /[A-Z]/.test(resetNewPw);
    const hasLower = /[a-z]/.test(resetNewPw);
    const hasDigit = /[0-9]/.test(resetNewPw);
    const hasSymbol = /[^A-Za-z0-9]/.test(resetNewPw);
    if (!hasUpper || !hasLower || !hasDigit || !hasSymbol) {
      setResetError("Password must include uppercase, lowercase, numbers, and symbols");
      return;
    }
    setResetLoading(true);
    try {
      await resetPassword(resetNewPw, resetAnswer);
      setResetSuccess(true);
    } catch (e: any) {
      setResetError(e);
    } finally {
      setResetLoading(false);
    }
  };

  const handleRecoveryKeySubmit = async () => {
    setResetError("");
    if (!resetRecoveryKey || !resetNewPw) return;
    if (resetNewPw !== resetConfirmPw) {
      setResetError("Passwords do not match");
      return;
    }
    if (resetNewPw.length < 8) {
      setResetError("Password must be at least 8 characters");
      return;
    }
    const hasUpper = /[A-Z]/.test(resetNewPw);
    const hasLower = /[a-z]/.test(resetNewPw);
    const hasDigit = /[0-9]/.test(resetNewPw);
    const hasSymbol = /[^A-Za-z0-9]/.test(resetNewPw);
    if (!hasUpper || !hasLower || !hasDigit || !hasSymbol) {
      setResetError("Password must include uppercase, lowercase, numbers, and symbols");
      return;
    }
    setResetLoading(true);
    try {
      await recoverWithKey(resetNewPw, resetRecoveryKey);
      setResetSuccess(true);
    } catch (e: any) {
      setResetError(e);
    } finally {
      setResetLoading(false);
    }
  };

  const handleUsbKeySubmit = async () => {
    setResetError("");
    if (!resetNewPw) return;
    if (resetNewPw !== resetConfirmPw) {
      setResetError("Passwords do not match");
      return;
    }
    if (resetNewPw.length < 8) {
      setResetError("Password must be at least 8 characters");
      return;
    }
    const hasUpper = /[A-Z]/.test(resetNewPw);
    const hasLower = /[a-z]/.test(resetNewPw);
    const hasDigit = /[0-9]/.test(resetNewPw);
    const hasSymbol = /[^A-Za-z0-9]/.test(resetNewPw);
    if (!hasUpper || !hasLower || !hasDigit || !hasSymbol) {
      setResetError("Password must include uppercase, lowercase, numbers, and symbols");
      return;
    }
    setResetLoading(true);
    try {
      await recoverWithUsbKey(resetNewPw);
      setResetSuccess(true);
    } catch (e: any) {
      setResetError(e);
    } finally {
      setResetLoading(false);
    }
  };

  const handleResetCancel = () => {
    setResetMode(null);
    setResetQuestion("");
    setResetAnswer("");
    setResetRecoveryKey("");
    setResetNewPw("");
    setResetConfirmPw("");
    setResetError("");
    setResetSuccess(false);
  };

  const handleResetBackToLogin = () => {
    handleResetCancel();
    setPassword("");
  };

  const handleGitHubLogin = async () => {
    setGithubLoading(true);
    setError("");
    try {
      const flow = await githubStartDeviceFlow();
      setGithubUserCode(flow.user_code);
      setGithubStep("code");
      openExternalUrl(flow.verification_uri);

      const result = await githubPollToken(flow.device_code, flow.interval, flow.expires_in);
      if (result.connected) {
        setGithubStep("idle");
        await onUnlock();
      }
    } catch (e: any) {
      setError(String(e));
      setGithubStep("idle");
    } finally {
      setGithubLoading(false);
    }
  };

  const copyGithubCode = () => {
    navigator.clipboard.writeText(githubUserCode);
    setGithubCopied(true);
    setTimeout(() => setGithubCopied(false), 2000);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError("");
    try {
      await unlockSession({ master_password: password, totp_code: totpCode });
      await onUnlock();
    } catch (e: any) {
      setError(e);
    } finally {
      setLoading(false);
    }
  };

  const resetPwValid = resetNewPw.length >= 8 && resetNewPw === resetConfirmPw
    && (resetMode === "usb_key" || resetAnswer.length > 0)
    && (resetMode !== "recovery_key" || resetRecoveryKey.length > 0);

  const renderResetForm = () => {
    if (resetMode === "question") {
      return (
        <div className="glass rounded-2xl p-6 space-y-4">
          <div>
            <label className="text-xs text-[color:var(--muted-foreground)] mb-1 block">Security question</label>
            <div className="text-sm font-medium px-3 py-2.5 rounded-lg border border-[color:var(--border)] bg-[color:var(--muted)]/30">
              {resetQuestion}
            </div>
          </div>
          <Field label="Your answer" icon={HelpCircle}>
            <input value={resetAnswer} onChange={e => setResetAnswer(e.target.value)}
                   placeholder="Enter your answer" autoFocus
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <Field label="New master password" icon={KeyRound}>
            <input type="password" value={resetNewPw} onChange={e => setResetNewPw(e.target.value)}
                   placeholder="Min 8 chars, upper+lower+number+symbol"
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <Field label="Confirm new password" icon={KeyRound}>
            <input type="password" value={resetConfirmPw} onChange={e => setResetConfirmPw(e.target.value)}
                   placeholder="Re-enter new password"
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <div className="flex gap-3 mt-2">
            <button type="button" onClick={handleResetCancel}
                    className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium border border-[color:var(--border)] text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] hover:border-[color:var(--foreground)]/30 transition">
              Cancel
            </button>
            <button type="button" onClick={handleResetSubmit} disabled={resetLoading || !resetPwValid}
                    className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                    style={{ background: "var(--gradient-brand)" }}>
              {resetLoading ? "Resetting..." : "Reset password"}
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      );
    }

    if (resetMode === "recovery_key") {
      return (
        <div className="glass rounded-2xl p-6 space-y-4">
          <Field label="Recovery key" icon={FileKey}>
            <input value={resetRecoveryKey} onChange={e => setResetRecoveryKey(e.target.value)}
                   placeholder="Paste your recovery key" autoFocus
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)] font-mono" />
          </Field>
          <Field label="New master password" icon={KeyRound}>
            <input type="password" value={resetNewPw} onChange={e => setResetNewPw(e.target.value)}
                   placeholder="Min 8 chars, upper+lower+number+symbol"
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <Field label="Confirm new password" icon={KeyRound}>
            <input type="password" value={resetConfirmPw} onChange={e => setResetConfirmPw(e.target.value)}
                   placeholder="Re-enter new password"
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <div className="flex gap-3 mt-2">
            <button type="button" onClick={handleResetCancel}
                    className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium border border-[color:var(--border)] text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] hover:border-[color:var(--foreground)]/30 transition">
              Cancel
            </button>
            <button type="button" onClick={handleRecoveryKeySubmit} disabled={resetLoading || !resetPwValid}
                    className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                    style={{ background: "var(--gradient-brand)" }}>
              {resetLoading ? "Resetting..." : "Reset password"}
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      );
    }

    if (resetMode === "usb_key") {
      return (
        <div className="glass rounded-2xl p-6 space-y-4">
          <div className="p-3 rounded-lg bg-[color:var(--primary)]/10 border border-[color:var(--primary)]/20 text-sm text-[color:var(--primary)]">
            Insert your enrolled USB pendrive and click Reset.
          </div>
          <Field label="New master password" icon={KeyRound}>
            <input type="password" value={resetNewPw} onChange={e => setResetNewPw(e.target.value)}
                   placeholder="Min 8 chars, upper+lower+number+symbol"
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <Field label="Confirm new password" icon={KeyRound}>
            <input type="password" value={resetConfirmPw} onChange={e => setResetConfirmPw(e.target.value)}
                   placeholder="Re-enter new password"
                   className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
          </Field>
          <div className="flex gap-3 mt-2">
            <button type="button" onClick={handleResetCancel}
                    className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium border border-[color:var(--border)] text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)] hover:border-[color:var(--foreground)]/30 transition">
              Cancel
            </button>
            <button type="button" onClick={handleUsbKeySubmit} disabled={resetLoading || !resetPwValid}
                    className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                    style={{ background: "var(--gradient-brand)" }}>
              {resetLoading ? "Resetting..." : "Reset with USB Key"}
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      );
    }

    return (
      <div className="space-y-3">
        <button onClick={handleResetQuestion} disabled={resetLoading}
                className="w-full flex items-center gap-4 p-4 rounded-xl border border-surface-border bg-surface hover:bg-surface-hover transition text-left">
          <div className="w-10 h-10 rounded-lg grid place-items-center shrink-0" style={{ background: "color-mix(in oklab, var(--cyan) 15%, transparent)", color: "var(--cyan)" }}>
            <HelpCircle className="w-5 h-5" />
          </div>
          <div>
            <div className="text-sm font-medium">Security Question</div>
            <div className="text-xs text-[color:var(--muted-foreground)]">Answer your security question to reset</div>
          </div>
          <ArrowRight className="w-4 h-4 text-[color:var(--muted-foreground)] ml-auto" />
        </button>
        <button onClick={() => { setResetError(""); setResetMode("recovery_key"); }} disabled={resetLoading}
                className="w-full flex items-center gap-4 p-4 rounded-xl border border-surface-border bg-surface hover:bg-surface-hover transition text-left">
          <div className="w-10 h-10 rounded-lg grid place-items-center shrink-0" style={{ background: "color-mix(in oklab, var(--violet) 15%, transparent)", color: "var(--violet)" }}>
            <FileKey className="w-5 h-5" />
          </div>
          <div>
            <div className="text-sm font-medium">Recovery Key</div>
            <div className="text-xs text-[color:var(--muted-foreground)]">Use your saved recovery key</div>
          </div>
          <ArrowRight className="w-4 h-4 text-[color:var(--muted-foreground)] ml-auto" />
        </button>
        <button onClick={() => { setResetError(""); setResetMode("usb_key"); }} disabled={resetLoading}
                className="w-full flex items-center gap-4 p-4 rounded-xl border border-surface-border bg-surface hover:bg-surface-hover transition text-left">
          <div className="w-10 h-10 rounded-lg grid place-items-center shrink-0" style={{ background: "color-mix(in oklab, var(--success) 15%, transparent)", color: "var(--success)" }}>
            <Usb className="w-5 h-5" />
          </div>
          <div>
            <div className="text-sm font-medium">USB Hardware Key</div>
            <div className="text-xs text-[color:var(--muted-foreground)]">Plug in your enrolled pendrive</div>
          </div>
          <ArrowRight className="w-4 h-4 text-[color:var(--muted-foreground)] ml-auto" />
        </button>
      </div>
    );
  };

  return (
    <div className="min-h-screen grid lg:grid-cols-2 text-[color:var(--foreground)]">
      <div className="hidden lg:flex flex-col justify-between p-12 border-r border-[color:var(--border)] relative overflow-hidden">
        <div className="absolute inset-0 opacity-60"
             style={{ background: "radial-gradient(ellipse at 30% 20%, oklch(0.35 0.18 210 / 0.5), transparent 60%), radial-gradient(ellipse at 70% 80%, oklch(0.35 0.22 295 / 0.45), transparent 60%)" }} />
        <div className="relative flex items-center gap-3">
          <img src="/icon.png" alt="OmniLock" className="w-11 h-11 rounded-xl object-cover glow-cyan" />
          <div>
            <div className="text-lg font-semibold">OmniLock</div>
            <div className="text-[11px] uppercase tracking-widest text-[color:var(--muted-foreground)]">by InnologyBD</div>
          </div>
        </div>
        <div className="relative space-y-6 max-w-md">
          <h1 className="text-4xl font-semibold tracking-tight leading-tight">
            Zero-trust security for every process, folder and drive.
          </h1>
          <p className="text-[color:var(--muted-foreground)] text-sm leading-relaxed">
            Argon2id key derivation, AES-256-GCM authenticated encryption, RFC 6238 TOTP
            and a dual-process watchdog. Enterprise-grade lockdown for Windows 10 & 11.
          </p>
          <div className="flex flex-col gap-3 pt-2">
            {[
              { icon: ShieldCheck, text: "SHA-256 binary hashing — rename bypass impossible" },
              { icon: Fingerprint, text: "Two-factor enforced on every unlock" },
              { icon: Activity, text: "Guardian daemon with self-healing revival" },
            ].map(({ icon: Icon, text }) => (
              <div key={text} className="flex items-center gap-3 text-sm">
                <div className="w-8 h-8 rounded-lg grid place-items-center"
                     style={{ background: "color-mix(in oklab, var(--cyan) 15%, transparent)", color: "var(--cyan)" }}>
                  <Icon className="w-4 h-4" />
                </div>
                <span className="text-[color:var(--muted-foreground)]">{text}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="relative text-xs text-[color:var(--muted-foreground)]">
          © 2026 InnologyBD · All systems nominal
        </div>
      </div>

      <div className="flex items-center justify-center p-6 sm:p-12">
        <div className="w-full max-w-md">
          <div className="lg:hidden flex items-center gap-3 mb-8">
            <img src="/icon.png" alt="OmniLock" className="w-10 h-10 rounded-xl object-cover" />
            <div className="text-lg font-semibold">OmniLock</div>
          </div>

          {!resetMode && !resetSuccess ? (
            <>
              <div className="mb-8">
                <div className="text-[11px] uppercase tracking-widest text-[color:var(--primary)] mb-2">Welcome back</div>
                <h2 className="text-3xl font-semibold tracking-tight">Sign in to OmniLock</h2>
                <p className="text-sm text-[color:var(--muted-foreground)] mt-2">Enter your master credentials to unlock the vault.</p>
              </div>

              {error && (
                <div className="mb-4 p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
                  {error}
                </div>
              )}

              <form onSubmit={handleSubmit} className="glass rounded-2xl p-6 space-y-4">
                <Field label="Master password" icon={KeyRound}>
                  <input type={showPw ? "text" : "password"} value={password} onChange={e => setPassword(e.target.value)}
                         placeholder="••••••••••••" autoFocus
                         className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
                  <button type="button" onClick={() => setShowPw(!showPw)} className="text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]">
                    {showPw ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                  </button>
                </Field>

                {totpEnabled && (
                  <Field label="Two-factor code" icon={Fingerprint}>
                    <input value={totpCode} onChange={e => setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))} placeholder="6-digit code" maxLength={6}
                           className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)] tracking-[0.3em]" />
                  </Field>
                )}

                <button type="submit" disabled={loading || !password}
                        className="w-full mt-2 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                        style={{ background: "var(--gradient-brand)" }}>
                  <Lock className="w-4 h-4" /> {loading ? "Unlocking..." : "Unlock vault"}
                  <ArrowRight className="w-4 h-4" />
                </button>
              </form>

              {biometricEnabled && biometricAvailable && (
                <div className="relative my-4">
                  <div className="absolute inset-0 flex items-center">
                    <div className="w-full border-t border-surface-border"></div>
                  </div>
                  <div className="relative flex justify-center text-[11px]">
                    <span className="px-3 text-[color:var(--muted-foreground)]" style={{ background: "var(--background)" }}>or</span>
                  </div>
                </div>
              )}

              {biometricEnabled && biometricAvailable && (
                <button onClick={handleBiometricLogin} disabled={biometricLoading}
                        className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium border border-[color:var(--primary)]/30 text-[color:var(--primary)] hover:bg-[color:var(--primary)]/10 transition-colors disabled:opacity-40">
                  <Fingerprint className="w-4 h-4" />
                  {biometricLoading ? "Verifying..." : "Login with Fingerprint"}
                </button>
              )}

              <div className="mt-4 text-center">
                <button onClick={() => { setResetMode("select"); setResetError(""); }} disabled={resetLoading}
                        className="text-xs text-[color:var(--primary)] hover:underline disabled:opacity-50">
                  Forgot password?
                </button>
              </div>

              <div className="relative my-6">
                <div className="absolute inset-0 flex items-center">
                  <div className="w-full border-t border-surface-border"></div>
                </div>
                <div className="relative flex justify-center text-[11px]">
                  <span className="px-3 text-[color:var(--muted-foreground)]" style={{ background: "var(--background)" }}>or</span>
                </div>
              </div>

              {githubStep === "code" ? (
                <div className="glass rounded-2xl p-6 space-y-4">
                  <div className="text-center">
                    <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)] mb-2">
                      Enter this code on GitHub
                    </div>
                    <div className="flex items-center justify-center gap-2">
                      <span className="text-2xl font-mono font-bold tracking-wider text-[color:var(--primary)]">
                        {githubUserCode}
                      </span>
                      <button onClick={copyGithubCode} className="p-1.5 rounded-md bg-surface hover:bg-surface-hover">
                        {githubCopied ? <CheckCircle2 className="w-4 h-4 text-[color:var(--success)]" /> : <Cloud className="w-4 h-4" />}
                      </button>
                    </div>
                  </div>
                  <div className="text-center text-[11px] text-[color:var(--muted-foreground)]">
                    Waiting for authorization...
                  </div>
                  <button onClick={() => { setGithubStep("idle"); setGithubLoading(false); }}
                          className="w-full text-center text-[11px] text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]">
                    Cancel
                  </button>
                </div>
              ) : (
                <button onClick={handleGitHubLogin} disabled={githubLoading}
                        className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium border border-surface-border bg-surface hover:bg-surface-hover transition-colors disabled:opacity-40">
                  <Github className="w-4 h-4" />
                  {githubLoading ? "Connecting..." : "Connect with GitHub"}
                </button>
              )}

              <div className="mt-6 text-center text-[11px] text-[color:var(--muted-foreground)]">
                Protected by Argon2id · AES-256-GCM · TOTP RFC 6238
              </div>
            </>
          ) : (
            <>
              <div className="mb-8">
                <div className="text-[11px] uppercase tracking-widest text-[color:var(--primary)] mb-2">Account recovery</div>
                <h2 className="text-3xl font-semibold tracking-tight">
                  {resetSuccess ? "Password reset" : resetMode === "select" ? "Recovery method" : "Reset your password"}
                </h2>
                <p className="text-sm text-[color:var(--muted-foreground)] mt-2">
                  {resetSuccess
                    ? "Your master password has been reset. Two-factor authentication has been disabled for security. You can now sign in with your new password."
                    : resetMode === "select"
                    ? "Choose how you want to recover your account. Two-factor authentication will be disabled after reset."
                    : resetMode === "usb_key"
                    ? "Insert your enrolled USB pendrive, then set a new master password."
                    : "Provide your credentials to set a new master password."}
                </p>
              </div>

              {!resetSuccess && resetError && (
                <div className="mb-4 p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
                  {resetError}
                </div>
              )}

              {!resetSuccess ? (
                renderResetForm()
              ) : (
                <div className="glass rounded-2xl p-6 text-center space-y-4">
                  <div className="w-14 h-14 rounded-full mx-auto grid place-items-center" style={{ background: "color-mix(in oklab, oklch(0.65 0.18 145) 15%, transparent)" }}>
                    <CheckCircle2 className="w-7 h-7" style={{ color: "oklch(0.65 0.18 145)" }} />
                  </div>
                  <button type="button" onClick={handleResetBackToLogin}
                          className="w-full px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan"
                          style={{ background: "var(--gradient-brand)" }}>
                    Back to sign in
                    <ArrowRight className="w-4 h-4" />
                  </button>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
