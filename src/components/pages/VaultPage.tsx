import { useState, useEffect } from "react";
import {
  HardDrive, Folder, FileLock2, Lock, Unlock, Plus, X, Loader2, FolderOpen, FileSearch, Search,
} from "lucide-react";
import {
  listDrives, addLockedDrive, removeLockedDrive,
  addLockedFolder, removeLockedFolder, addLockedFile, removeLockedFile, showWidget,
  pickFolder, pickFile,
  type VaultConfigDto,
} from "../../lib/tauri-bridge";
import { SectionHeader } from "../shared/SectionHeader";

export function VaultPage({ config, refresh }: { config: VaultConfigDto | null; refresh: () => Promise<void> }) {
  const [drives, setDrives] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [lockingDrive, setLockingDrive] = useState<string | null>(null);
  const [lockingPath, setLockingPath] = useState<string | null>(null);
  const [pathSearch, setPathSearch] = useState("");

  useEffect(() => {
    listDrives().then(setDrives).catch(e => setError("Failed to list drives: " + e));
  }, []);

  const lockedDrives = config?.locked_drives || [];
  const lockedFolders = config?.locked_folders || [];
  const lockedFiles = config?.locked_files || [];
  const filteredFolders = lockedFolders.filter(f => f.toLowerCase().includes(pathSearch.toLowerCase()));
  const filteredFiles = lockedFiles.filter(f => f.toLowerCase().includes(pathSearch.toLowerCase()));

  const clearMessages = () => { setError(""); setSuccess(""); };

  const handleToggleDrive = async (letter: string) => {
    clearMessages();
    setLockingDrive(letter);
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
    setLockingDrive(null);
  };

  const handlePickFolder = async () => {
    clearMessages();
    try {
      const path = await pickFolder();
      if (!path) return;
      setLockingPath(path);
      await addLockedFolder(path);
      setSuccess(`Folder locked: ${path}`);
      await refresh();
    } catch (e: any) {
      setError("Failed to lock folder: " + e);
    }
    setLockingPath(null);
  };

  const handlePickFile = async () => {
    clearMessages();
    try {
      const path = await pickFile();
      if (!path) return;
      setLockingPath(path);
      await addLockedFile(path);
      setSuccess(`File locked: ${path}`);
      await refresh();
    } catch (e: any) {
      setError("Failed to lock file: " + e);
    }
    setLockingPath(null);
  };

  const handleRemovePath = async (path: string, type: "file" | "folder") => {
    clearMessages();
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
                <div className="mt-4 h-1.5 rounded-full bg-surface overflow-hidden">
                  <div className="h-full rounded-full" style={{
                    width: isLocked ? "100%" : "0%",
                    background: isLocked ? "var(--gradient-brand)" : "oklch(1 0 0 / 0.2)",
                  }} />
                </div>
                <div className="mt-3 flex items-center justify-between text-xs text-[color:var(--muted-foreground)]">
                  <button onClick={() => handleToggleDrive(letter)} disabled={lockingDrive === letter}
                          className={`px-3 py-1.5 rounded-lg border transition text-xs flex items-center gap-1.5 ${isLocked ? "border-[color:var(--primary)]/30 text-[color:var(--primary)] hover:bg-[color:var(--primary)]/10" : "border-surface-border hover:bg-surface"} disabled:opacity-40`}>
                    {lockingDrive === letter ? <Loader2 className="w-3 h-3 animate-spin" /> : null}
                    {isLocked ? "Unlock" : "Lock"}
                  </button>
                  {isLocked && (
                    <button onClick={() => showWidget("drive", letter, `${letter}:\\`)}
                            className="px-3 py-1.5 rounded-lg text-xs border border-[color:var(--success)]/30 text-[color:var(--success)] hover:bg-[color:var(--success)]/10">
                      Password Unlock
                    </button>
                  )}
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
          <div className="ml-auto flex items-center gap-2">
            {(lockedFolders.length + lockedFiles.length) > 0 && (
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-surface border border-surface-border">
                <Search className="w-3.5 h-3.5 text-[color:var(--muted-foreground)]" />
                <input placeholder="Search paths..." value={pathSearch} onChange={e => setPathSearch(e.target.value)}
                       className="bg-transparent outline-none text-xs w-32 placeholder:text-[color:var(--muted-foreground)]" />
              </div>
            )}
            <button onClick={handlePickFolder} disabled={lockingPath !== null}
                    className="px-3 py-2 rounded-lg text-sm bg-surface border border-surface-border flex items-center gap-2 hover:bg-surface-active disabled:opacity-40">
              {lockingPath ? <Loader2 className="w-4 h-4 animate-spin" /> : <FolderOpen className="w-4 h-4" />}
              {lockingPath ? "Locking..." : "Add Folder"}
            </button>
            <button onClick={handlePickFile} disabled={lockingPath !== null}
                    className="px-3 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] flex items-center gap-2 glow-cyan disabled:opacity-40"
                    style={{ background: "var(--gradient-brand)" }}>
              {lockingPath ? <Loader2 className="w-4 h-4 animate-spin" /> : <FileSearch className="w-4 h-4" />}
              {lockingPath ? "Locking..." : "Lock File"}
            </button>
          </div>
        </div>
        <div className="divide-y divide-surface-border">
          {filteredFolders.map(f => (
            <div key={f} className="px-5 py-4 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg grid place-items-center bg-surface border border-surface-border">
                <Folder className="w-4 h-4 text-[color:var(--primary)]" />
              </div>
              <div className="flex-1 min-w-0">
                <code className="text-sm">{f}</code>
                <div className="text-xs text-[color:var(--muted-foreground)] mt-0.5">Folder · <span className="text-[color:var(--warning)]">DENIED · EVERYONE</span></div>
              </div>
              <button onClick={() => handleRemovePath(f, "folder")} className="p-1.5 rounded-lg hover:bg-surface-hover text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)]">
                <X className="w-4 h-4" />
              </button>
              <button onClick={() => showWidget("folder", f, f.split("\\").pop() || f)}
                      className="px-3 py-1.5 rounded-lg text-xs border border-[color:var(--success)]/30 text-[color:var(--success)] hover:bg-[color:var(--success)]/10">
                Unlock
              </button>
            </div>
          ))}
          {filteredFiles.map(f => (
            <div key={f} className="px-5 py-4 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg grid place-items-center bg-surface border border-surface-border">
                <FileLock2 className="w-4 h-4 text-[color:var(--violet)]" />
              </div>
              <div className="flex-1 min-w-0">
                <code className="text-sm">{f}</code>
                <div className="text-xs text-[color:var(--muted-foreground)] mt-0.5">File · <span className="text-[color:var(--warning)]">DENIED · SYSTEM</span></div>
              </div>
              <button onClick={() => handleRemovePath(f, "file")} className="p-1.5 rounded-lg hover:bg-surface-hover text-[color:var(--muted-foreground)] hover:text-[color:var(--destructive)]">
                <X className="w-4 h-4" />
              </button>
              <button onClick={() => showWidget("file", f, f.split("\\").pop() || f)}
                      className="px-3 py-1.5 rounded-lg text-xs border border-[color:var(--success)]/30 text-[color:var(--success)] hover:bg-[color:var(--success)]/10">
                Unlock
              </button>
            </div>
          ))}
          {lockedFolders.length === 0 && lockedFiles.length === 0 && (
            <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
              No paths locked yet. Click "Add Folder" or "Lock File" to get started.
            </div>
          )}
          {pathSearch && filteredFolders.length === 0 && filteredFiles.length === 0 && lockedFolders.length + lockedFiles.length > 0 && (
            <div className="p-8 text-center text-[color:var(--muted-foreground)] text-sm">
              No paths match "{pathSearch}".
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
