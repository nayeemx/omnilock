Add-Type -AssemblyName System.Drawing

function New-IconPng {
    param([string]$path, [int]$size, [string]$color)
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'HighQuality'
    $pen = New-Object System.Drawing.Pen($color, [Math]::Max(2, [int]($size/12)))
    $brush = New-Object System.Drawing.SolidBrush($color)
    $w = $size * 0.6
    $h = $size * 0.45
    $x = ($size - $w)/2
    $y = $size * 0.35
    $rect = New-Object System.Drawing.RectangleF($x, $y, $w, $h)
    $g.FillRectangle($brush, $rect)
    $g.DrawRectangle($pen, $rect)
    $shackleW = $w * 0.55
    $shackleH = $size * 0.3
    $shackleX = ($size - $shackleW)/2
    $shackleY = $size * 0.08
    $g.DrawArc($pen, $shackleX, $shackleY, $shackleW, $shackleH, 180, 180)
    $g.Dispose()
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

$sizes = @(16, 24, 32, 48, 256)
New-Item -ItemType Directory -Force -Path "D:\projects\code\omnilock\src-tauri\icons" | Out-Null
foreach ($s in $sizes) {
    New-IconPng "D:\projects\code\omnilock\src-tauri\icons\lock_${s}.png" $s '#e63e3e'
}
Write-Host "Generated lock icons"
foreach ($s in $sizes) {
    New-IconPng "D:\projects\code\omnilock\src-tauri\icons\unlock_${s}.png" $s '#22c55e'
}
Write-Host "Generated unlock icons"
Get-ChildItem "D:\projects\code\omnilock\src-tauri\icons" | ForEach-Object { "$($_.Name) - $($_.Length) bytes" }
