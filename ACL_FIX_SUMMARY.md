# ACL Recursive Unlock Fix - Summary

## Problem
When unlocking a folder in OmniLock, child files inside the folder remained inaccessible because:
1. The recursive unlock function used `std::fs::read_dir()` which fails with access denied on locked folders
2. Even with `SeTakeOwnershipPrivilege`, Rust's `read_dir` cannot enumerate files when the DACL denies access
3. This left child files with the restricted ACL (Admins+SYSTEM only) even after folder unlock

## Solution (v0.0.33)

### 1. Win32 API Enumeration
Replaced `read_dir` with Win32 `FindFirstFileW`/`FindNextFileW` which:
- Works even when DACL is restricted
- Properly respects `SeTakeOwnershipPrivilege`
- Can enumerate files before accessing them

### 2. New Force Unlock Command
Added `force_unlock` Tauri command that:
- Enables both `SeTakeOwnershipPrivilege` and `SeRestorePrivilege`
- Takes ownership first
- Resets DACL
- Restores original owner
- Bypasses all ACL restrictions

### 3. Enhanced Logging
Added detailed logging to track:
- Folder unlock start/completion
- Recursive unlock warnings
- Force unlock operations

## Files Modified

| File | Changes |
|------|---------|
| `src-tauri/src/file_locker.rs` | Added `unlock_files_recursive()`, `force_unlock()`, enhanced logging |
| `service/src/acl.rs` | Added `remove_files_recursive()` with Win32 API |
| `src-tauri/src/lib.rs` | Added `cmd_force_unlock` Tauri command |
| `src/lib/tauri-bridge.ts` | Added `forceUnlock()` TypeScript function |
| `AGENTS.md` | Updated version and documentation |
| `test_acl_recursive_unlock.ps1` | Added test script |

## Testing

Run the test script to verify the fix:
```powershell
powershell -ExecutionPolicy Bypass -File test_acl_recursive_unlock.ps1
```

Then manually test:
1. Build and install the application
2. Lock a folder with files inside
3. Verify files are inaccessible
4. Unlock the folder
5. Verify all files are now accessible

## Build Commands

```bash
# Frontend
npm run build

# Backend (Rust)
cd src-tauri && cargo check
cd .. && npm run tauri build
```

## Known Limitations

1. **Admin Required**: Both `SeTakeOwnershipPrivilege` and `SeRestorePrivilege` require admin rights
2. **Service vs App**: The service-side fix may still have issues if running without elevated privileges
3. **Network Files**: UNC paths may have different behavior

## Next Steps

1. Build production installer with `npm run tauri build`
2. Sign with auto-updater key
3. Update `latest.json` with new signature
4. Test end-to-end on Windows
5. Consider adding UI button for "Force Unlock" in rescue mode
