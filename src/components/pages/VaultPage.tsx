import { useState, useEffect } from "react";
import {
  HardDrive, Folder, FileLock2, Lock, Unlock, Plus, X,
} from "lucide-react";
import {
  listDrives, addLockedDrive, removeLockedDrive,
  addLockedFolder, removeLockedFolder, addLockedFile, removeLockedFile,
  type VaultConfigDto,
} from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";
import { Field } from "../shared/Field";

export function VaultPage({ config, refresh }: { config: VaultConfigDto | null; refresh: () => Promise<void> }) {
  const [drives, setDrives] = useState<string[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [addPath, setAddPath] = useState("");
  const [addType, setAddType] = useState<"file" | "folder">("folder");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  useEffect(() => {
    listDrives().then(setDrives).catch(e => setError("Failed to list drives: " + e));
  }, []);

  const lockedDrives = config?.locked_drives || [];
  const lockedFolders = config?.locked_folders || [];
  const lockedFiles = config?.locked_files || [];

  const handleToggleDrive = async (letter: string) => {
    setError(""); setSuccess("");
    try {
      if (lockedDrives.includes(letter)) {
        await removeLockedDrive(letter);
        setSuccess(`Drive ${letter}:\\ unlocked.`);
      } else {
        await addLockedDrive(letter);
        setSuccess(`Drive ${letter}:\\ locked.`);
      }
      await refresh();
    } catch (e: any) {
      setError("Failed: " + e);
      await refresh();
    }
  };

  const handleAddPath = async () => {
    if (!addPath.trim()) return;
    setError(""); setSuccess("");
    try {
      if (addType === "folder") {
        await addLockedFolder(addPath);
      } else {
        await addLockedFile(addPath);
      }
      setSuccess(`Path locked: ${addPath}`);
      setAddPath("");
      setShowAdd(false);
      await refresh();
    } catch (e: any) {
      setError("Failed to lock path: " + e);
    }
  };

  const handleRemovePath = async (path: string, type: "file" | "folder") => {
    setError(""); setSuccess("");
    try {
      if (type === "folder") {
        await removeLockedFolder(path);
      } else {
        await removeLockedFile(path);
      }
      setSuccess(`Path unlocked: ${path}`);
      await refresh();
    } catch (e: any) {
      setError("Failed to unlock path: " + e);
      await refresh();
    }
  };

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <SectionHeader
        eyebrow="FR-FILE · Windows NT DACL"
        title="File, Folder & Drive Vault"
        subtitle="Enforces access control by revoking GENERIC_ALL for EVERYONE, SYSTEM and Administrators via native security descriptors."
      />

      {error && <div className="p-3 rounded-lg bg-[color:var(--destructive)]/15 border border-[color:var(--destructive)]/30 text-sm text-[color:var(--destructive)]">{error}</div>}
      {success && <div className="p-3 rounded-lg bg-[color:var(--success)]/15 border border-[color:var(--success)]/30 text-sm text-[color:var(--success)]">{success}</div>}

      <div>
        <h3 className="text-sm uppercase tracking-widest text-[color:var(--muted-foreground)] mb-3">Drive Volumes</h3>
        <div className="grid md:grid-cols-2 xl:grid-cols-4 gap-4">
          {drives.map(letter => {
            const isLocked = lockedDrives.includes(letter);
            return (
              <div key={letter} className={`glass rounded-2xl p-5 relative overflow-hidden ${isLocked ? "border-[color:var(--primary)]/30" : ""}`}>
                {isLocked && <div className="absolute -top-16 -right-16 w-40 h-40 rounded-full opacity-30 blur-3xl" style={{ background: "var(--gradient-brand)" }} />}
                <div className="flex items-start justify-between relative">
                  <HardDrive className={`w-8 h-8 ${isLocked ? "text-[color:var(--primary)]" : "text-[color:var(--muted-foreground)]"}`} />
                  {isLocked ? <Lock className="w-4 h-4 text-[color:var(--primary)]" /> : <Unlock className="w-4 h-4 text-[color:var(--muted-foreground)]" />}
                </div>
                <div className="mt-4 relative">
                  <div className="text-2xl font-semibold tracking-tight">{letter}:\</div>
                  <div className="text-xs text-[color:var(--muted-foreground)]">{isLocked ? "Locked" : "Unlocked"}</div>
                </div>
                <div className="mt-4 h-1.5 rounded-full bg-white/[0.06] overflow-hidden">
                  <div className="h-full rounded-full" style={{
                    width: isLocked ? "100%" : "0%",
                    background: isLocked ? "var(--gradient-brand)" : "oklch(1 0 0 / 0.2)",
                  }} />
                </div>
                <div className="mt-3 flex items-center justify-between text-xs text-[color:var(--muted-foreground)]">
                  <button onClick={() => handleToggleDrive(letter)}
                          className={`px-3 py-1.5 rounded-lg border transition text-xs ${isLocked ? "border-[color:var(--primary)]/30 text-[color:var(--primary)] hover:bg-[color:var(--primary)]/10" : "border-white/10 hover:bg-white/[0.04]"}`}>
                    {isLocked ? "Unlock" : "Lock"}
                  </button>
                  <span className={isLocked ? "text-[color:var(--primary)]" : ""}>{isLocked ? "DACL Enforced" : "Unprotected"}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="glass rounded-2xl overflow-hidden">
        <div className="p-5 border-b border-[color:var(--border)] flex items-center gap-3">
          <FileLock2 className="w-5 h-5 text-[color:var(--primary)]" />
          <h3 className="font-semibold">Protected Paths</h3>
          <span className="text-xs text-[color:var(--muted-foreground)]">{lockedFolders.length + lockedFiles.length} entries</span>
          <div className="ml-auto flex gap-2">
            <button onClick={() => { setAddType("folder"); setShowAdd(true); }}
                    className="px-3 py-2 rounded-lg text-sm bg-white/[0.04] border border-white/10 flex items-center gap-2">
              <Folder className="w-4 h-4" /> Add Folder
            </button>
            <button onClick={() => { setAddType("file"); setShowAdd(true); }}
                    className="px-3 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan"
                    style={{ background: "var(--gradient-brand)" }}>
              <Plus className="w-4 h-4" /> Lock New Path
            </button>
          </div>
        </div>
        <div className="divide-y divide-white/[0.06]">
          {lockedFolders.map(f => (
            <div key={f} className="px-5 py-4 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg grid place-items-center bg-white/[0.04] border border-white/10">
                <Folder className="w-4 h-4 text-[color:var(--primary)]" />
              </div>
              <div className="flex-1 min-w-0">
                <code className="text-sm">{f}</code>
                <div className="text-xs text-[color:var(--muted-foreground)] mt-0.5">Folder · <span className="text-[color:var(--warning)]">DENIED · EVERYONE</span></div>
              </div>
              <button onClick={() => handleRemovePath(f, "folder")} className="p-1.5 rounded-lg hover:bg-white/[0.06] text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)]">
                <X className="w-4 h-4" />
              </button>
            </div>
          ))}
          {lockedFiles.map(f => (
            <div key={f} className="px-5 py-4 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg grid place-items-center bg-white/[0.04] border border-white/10">
                <FileLock2 className="w-4 h-4 text-[color:var(--violet)]" />
              </div>
              <div className="flex-1 min-w-0">
                <code className="text-sm">{f}</code>
                <div className="text-xs text-[color:var(--muted-foreground)] mt-0.5">File · <span className="text-[color:var(--warning)]">DENIED · SYSTEM</span></div>
              </div>
              <button onClick={() => handleRemovePath(f, "file")} className="p-1.5 rounded-lg hover:bg-white/[0.06] text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)]">
                <X className="w-4 h-4" />
              </button>
            </div>
          ))}
          {lockedFolders.length === 0 && lockedFiles.length === 0 && (
            <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
              No paths locked yet. Click "Lock New Path" to get started.
            </div>
          )}
        </div>
      </div>

      {showAdd && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass rounded-2xl w-full max-w-md p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold">Lock {addType === "folder" ? "Folder" : "File"}</h3>
              <button onClick={() => setShowAdd(false)} className="p-1.5 rounded-lg hover:bg-white/[0.06]"><X className="w-4 h-4" /></button>
            </div>
            <Field label="Path" icon={addType === "folder" ? Folder : FileLock2}>
              <input value={addPath} onChange={e => setAddPath(e.target.value)}
                     placeholder={addType === "folder" ? "D:\\Private\\Financials" : "C:\\Users\\file.txt"}
                     className="flex-1 bg-transparent outline-none text-sm placeholder:text-[color:var(--muted-foreground)]" />
            </Field>
            <div className="flex gap-3 mt-4">
              <button onClick={() => setShowAdd(false)} className="flex-1 px-4 py-2.5 rounded-lg text-sm bg-white/[0.04] border border-white/10 hover:bg-white/[0.08]">
                Cancel
              </button>
              <button onClick={handleAddPath} disabled={!addPath.trim()}
                      className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center justify-center gap-2 glow-cyan disabled:opacity-40"
                      style={{ background: "var(--gradient-brand)" }}>
                <Lock className="w-4 h-4" /> Lock
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
