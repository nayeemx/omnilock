# ACL Recursive Unlock Test Script
# This script tests the fix for the file-unlock child ACL inheritance gap

Write-Host "=== OmniLock ACL Recursive Unlock Test ===" -ForegroundColor Cyan
Write-Host ""

# Test parameters
$testFolder = "D:\OmniLockTest_Folder"
$testFile1 = "$testFolder\test1.txt"
$testFile2 = "$testFolder\test2.txt"
$testSubFolder = "$testFolder\subfolder"
$testFile3 = "$testSubFolder\test3.txt"

# Cleanup previous test
if (Test-Path $testFolder) {
    Write-Host "Cleaning up previous test data..." -ForegroundColor Yellow
    Remove-Item $testFolder -Recurse -Force
}

# Create test structure
Write-Host "Creating test folder structure..." -ForegroundColor Green
New-Item -ItemType Directory -Path $testFolder -Force | Out-Null
New-Item -ItemType Directory -Path $testSubFolder -Force | Out-Null
New-Item -ItemType File -Path $testFile1 -Force | Out-Null
New-Item -ItemType File -Path $testFile2 -Force | Out-Null
New-Item -ItemType File -Path $testFile3 -Force | Out-Null

# Add some content to test files
"Test content 1" | Out-File $testFile1 -Encoding utf8
"Test content 2" | Out-File $testFile2 -Encoding utf8
"Test content 3" | Out-File $testFile3 -Encoding utf8

Write-Host "Test structure created:" -ForegroundColor Green
Write-Host "  $testFolder\"
Write-Host "    test1.txt"
Write-Host "    test2.txt"
Write-Host "    subfolder\"
Write-Host "      test3.txt"
Write-Host ""

# Show current ACLs
Write-Host "=== Current ACLs (Before Lock) ===" -ForegroundColor Cyan
Get-Acl $testFolder | Format-List
Get-Acl $testFile1 | Format-List
Get-Acl $testFile2 | Format-List
Get-Acl $testSubFolder | Format-List
Get-Acl $testFile3 | Format-List
Write-Host ""

# Note: To test the actual OmniLock unlock, you would need to:
# 1. Build the application with the fix
# 2. Lock the folder using OmniLock
# 3. Try to access the files (should fail)
# 4. Unlock the folder using OmniLock
# 5. Verify files are accessible again

Write-Host "=== Test Instructions ===" -ForegroundColor Yellow
Write-Host "1. Build OmniLock: npm run tauri build" -ForegroundColor White
Write-Host "2. Install the application" -ForegroundColor White
Write-Host "3. Create a vault and unlock" -ForegroundColor White
Write-Host "4. Lock the folder: $testFolder" -ForegroundColor White
Write-Host "5. Try to access files inside (should get Access Denied)" -ForegroundColor White
Write-Host "6. Unlock the folder using OmniLock" -ForegroundColor White
Write-Host "7. Verify files are now accessible:" -ForegroundColor White
Write-Host "   - $testFile1" -ForegroundColor White
Write-Host "   - $testFile2" -ForegroundColor White
Write-Host "   - $testFile3" -ForegroundColor White
Write-Host ""

Write-Host "=== Cleanup ===" -ForegroundColor Cyan
Write-Host "To clean up test data, run:" -ForegroundColor White
Write-Host "Remove-Item '$testFolder' -Recurse -Force" -ForegroundColor Gray
