import { useState } from "react";
import {
  Shield, Eye, EyeOff, ArrowRight,
} from "lucide-react";
import { setupVault, unlockSession } from "../../lib/tauri-bridge";
import { Field } from "../shared/Field";
import { securityQuestions, type SetupStep } from "../types";

export function SetupWizard({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState<SetupStep>(1);
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPw, setShowPw] = useState(false);
  const [question, setQuestion] = useState(securityQuestions[0]);
  const [answer, setAnswer] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const pwValid = password.length >= 8 && /[A-Z]/.test(password) && /[a-z]/.test(password) && /[0-9]/.test(password) && /[^A-Za-z0-9]/.test(password);
  const pwMatch = password === confirmPassword && confirmPassword.length > 0;

  const pwChecks = [
    { label: "At least 8 characters", ok: password.length >= 8 },
    { label: "Uppercase letter", ok: /[A-Z]/.test(password) },
    { label: "Lowercase letter", ok: /[a-z]/.test(password) },
    { label: "Number", ok: /[0-9]/.test(password) },
    { label: "Special character", ok: /[^A-Za-z0-9]/.test(password) },
  ];

  const handleFinish = async () => {
    setLoading(true);
    setError("");
    try {
      await setupVault({
        master_password: password,
        security_question: question,
        security_answer: answer,
        totp_secret: "",
      });
      await unlockSession({ master_password: password, totp_code: "" });
      onComplete();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-6" style={{ background: "var(--background)" }}>
      <div className="w-full max-w-lg">
        <div className="flex items-center gap-3 mb-8">
          <div className="w-11 h-11 rounded-xl grid place-items-center glow-cyan" style={{ background: "var(--gradient-brand)" }}>
            <Shield className="w-6 h-6 text-primary-foreground" strokeWidth={2.5} />
          </div>
          <div>
            <div className="text-lg font-semibold tracking-tight">OmniLock</div>
            <div className="text-[11px] text-[color:var(--muted-foreground)] tracking-wider uppercase">by InnologyBD</div>
          </div>
        </div>

        <div className="mb-6">
          <div className="text-[11px] uppercase tracking-widest text-[color:var(--primary)] mb-2">Step {step} of 2</div>
          <h2 className="text-3xl font-semibold tracking-tight">
            {step === 1 && "Create master password"}
            {step === 2 && "Security question"}
          </h2>
          <p className="text-sm text-[color:var(--muted-foreground)] mt-2">
            {step === 1 && "Choose a strong password to protect your vault."}
            {step === 2 && "Set up a recovery question in case you forget your password."}
          </p>
        </div>

        {error && (
          <div className="mb-4 p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">
            {error}
          </div>
        )}

        <div className="glass rounded-2xl p-6">
          {step === 1 && (
            <div className="space-y-4">
              <Field label="Master password" icon={Shield}>
                <input type={showPw ? "text" : "password"} value={password} onChange={e => setPassword(e.target.value)}
                       placeholder="Min 8 chars, mixed case, numbers, symbols"
                       className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
                <button type="button" onClick={() => setShowPw(!showPw)} className="text-[color:var(--muted-foreground)] hover:text-[color:var(--foreground)]">
                  {showPw ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </Field>
              {password && (
                <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
                  {pwChecks.map(c => (
                    <div key={c.label} className={`flex items-center gap-1.5 text-[11px] ${c.ok ? "text-[color:var(--success)]" : "text-[color:var(--muted-foreground)]"}`}>
                      <span className={`w-3.5 h-3.5 rounded-full grid place-items-center text-[9px] font-bold ${c.ok ? "bg-[color:var(--success)]/20" : "bg-white/[0.05] border border-white/10"}`}>
                        {c.ok ? "\u2713" : ""}
                      </span>
                      {c.label}
                    </div>
                  ))}
                </div>
              )}
              <Field label="Confirm password" icon={Shield}>
                <input type={showPw ? "text" : "password"} value={confirmPassword} onChange={e => setConfirmPassword(e.target.value)}
                       placeholder="Repeat your password"
                       className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
              </Field>
              {confirmPassword && (
                <div className={`text-xs ${pwMatch ? "text-[color:var(--success)]" : "text-[color:var(--destructive)]"}`}>
                  {pwMatch ? "Passwords match" : "Passwords do not match"}
                </div>
              )}
              <button onClick={() => setStep(2)} disabled={!pwValid || !pwMatch}
                      className="w-full px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40 disabled:cursor-not-allowed"
                      style={{ background: "var(--gradient-brand)" }}>
                Continue <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          )}

          {step === 2 && (
            <div className="space-y-4">
              <label className="block">
                <div className="text-[10px] uppercase tracking-widest text-[color:var(--muted-foreground)] mb-1.5">Security question</div>
                <select value={question} onChange={e => setQuestion(e.target.value)}
                        className="w-full px-3 py-2.5 rounded-lg bg-white/[0.03] border border-white/10 text-sm outline-none focus:border-[color:var(--primary)]/50 transition-colors">
                  {securityQuestions.map(q => <option key={q} value={q} className="bg-[#1a1a2e]">{q}</option>)}
                </select>
              </label>
              <Field label="Answer" icon={Shield}>
                <input value={answer} onChange={e => setAnswer(e.target.value)} placeholder="Your answer"
                       className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
              </Field>
              <div className="flex gap-3">
                <button onClick={() => setStep(1)} className="flex-1 px-4 py-2.5 rounded-lg text-sm bg-white/[0.04] border border-white/10 hover:bg-white/[0.08]">
                  Back
                </button>
                <button onClick={handleFinish} disabled={!answer.trim() || loading}
                        className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                        style={{ background: "var(--gradient-brand)" }}>
                  {loading ? "Setting up..." : "Finish Setup"} <ArrowRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="mt-6 text-center text-[11px] text-[color:var(--muted-foreground)]">
          Protected by Argon2id · AES-256-GCM · TOTP RFC 6238
        </div>
      </div>
    </div>
  );
}
