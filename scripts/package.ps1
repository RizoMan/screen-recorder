$ErrorActionPreference = "Stop"

Write-Host "Screen Recorder Pro - Automated Packaging Pipeline" -ForegroundColor Cyan
Write-Host "Building release binary..." -ForegroundColor Yellow

cargo build --release --workspace

if ($LASTEXITCODE -ne 0) {
    throw "Release build failed."
}

Write-Host "Build complete: target/release/rec-app.exe" -ForegroundColor Green

# Check if ISCC (Inno Setup Compiler) is installed
$isccPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    (Get-Command iscc.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)
)

$iscc = $isccPaths | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1

if ($iscc) {
    Write-Host "Compiling Windows installer using $iscc..." -ForegroundColor Yellow
    & $iscc "packaging/windows/installer.iss"
    Write-Host "Installer generated in dist/ directory." -ForegroundColor Green
} else {
    Write-Host "Notice: Inno Setup compiler (ISCC.exe) not found on PATH. Packaged binary is available in target/release/rec-app.exe" -ForegroundColor DarkYellow
}
