<#
.SYNOPSIS
    Builds the production release binary and compiles the Inno Setup Windows installer for Screen Recorder Pro.

.DESCRIPTION
    Automates the entire distribution pipeline:
    1. Optionally bumps the version in Cargo.toml and ScreenRecorderPro.iss.
    2. Builds the stripped, optimized release executable.
    3. Compiles the modern Inno Setup wizard installer into installer/ and dist/.

.PARAMETER Version
    Optional new version string (e.g., "0.2.0"). If omitted, uses the current version in Cargo.toml or auto-increments.

.EXAMPLE
    .\build_installer.ps1
    .\build_installer.ps1 -Version "0.2.0"
#>

[CmdletBinding()]
param (
    [string]$Version
)

$ErrorActionPreference = "Stop"
$WorkspaceRoot = $PSScriptRoot

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host "    Screen Recorder Pro - Installer Build Automation    " -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan

# 1. Ensure MinGW / GCC toolchain is in PATH for GNU rustc
if (Test-Path "C:\msys64\mingw64\bin") {
    $env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
}

# 1. Locate ISCC.exe (Inno Setup Compiler)
$IsccPaths = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe"
)

$IsccExe = $null
foreach ($path in $IsccPaths) {
    if (Test-Path $path) {
        $IsccExe = $path
        break
    }
}

if (-not $IsccExe) {
    Write-Error "ISCC.exe (Inno Setup Compiler) was not found. Please install Inno Setup 6."
}

# 2. Handle version in Cargo.toml and ScreenRecorderPro.iss
$CargoTomlPath = Join-Path $WorkspaceRoot "Cargo.toml"
$IssPath = Join-Path $WorkspaceRoot "installer\ScreenRecorderPro.iss"

if ($Version) {
    Write-Host "[1/3] Updating version to v$Version..." -ForegroundColor Yellow
    
    $cargoContent = Get-Content $CargoTomlPath -Raw
    $cargoContent = $cargoContent -replace '(?ms)(\[workspace\.package\].*?version\s*=\s*)"[^"]+"', "`$1`"$Version`""
    Set-Content -Path $CargoTomlPath -Value $cargoContent -NoNewline
    
    $issContent = Get-Content $IssPath -Raw
    $issContent = $issContent -replace '#define MyAppVersion\s*"[^"]+"', "#define MyAppVersion `"$Version`""
    Set-Content -Path $IssPath -Value $issContent -NoNewline
} else {
    $cargoContent = Get-Content $CargoTomlPath -Raw
    if ($cargoContent -match '(?ms)\[workspace\.package\].*?version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        $Version = "0.1.1"
    }
    Write-Host "[1/3] Using current version v$Version..." -ForegroundColor Yellow

    # Ensure ISS matches
    $issContent = Get-Content $IssPath -Raw
    $issContent = $issContent -replace '#define MyAppVersion\s*"[^"]+"', "#define MyAppVersion `"$Version`""
    Set-Content -Path $IssPath -Value $issContent -NoNewline
}

# 3. Stop running instance of rec-app to prevent file locks
Stop-Process -Name rec-app -Force -ErrorAction SilentlyContinue

# 4. Build optimized release binary
Write-Host "[2/3] Compiling release binary (rec-app.exe)..." -ForegroundColor Yellow
Push-Location $WorkspaceRoot
try {
    cargo build --release -p rec-app
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

# 5. Compile Inno Setup Installer
Write-Host "[3/3] Compiling Windows Installer with Inno Setup..." -ForegroundColor Yellow
& $IsccExe $IssPath
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
}

# 6. Mirror output to dist/
$InstallerOut = Join-Path $WorkspaceRoot "installer\ScreenRecorderPro_Setup_v$Version.exe"
$DistDir = Join-Path $WorkspaceRoot "dist"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
Copy-Item $InstallerOut (Join-Path $DistDir "ScreenRecorderPro_Setup_v$Version.exe") -Force

$FileSizeMb = [math]::Round((Get-Item $InstallerOut).Length / 1MB, 2)

Write-Host ""
Write-Host "=======================================================" -ForegroundColor Green
Write-Host " [SUCCESS] Installer generated successfully!           " -ForegroundColor Green
Write-Host "=======================================================" -ForegroundColor Green
Write-Host "  Version:   v$Version" -ForegroundColor White
Write-Host "  Size:      $FileSizeMb MB" -ForegroundColor White
Write-Host "  Location:  $InstallerOut" -ForegroundColor Cyan
Write-Host "  Mirror:    dist\ScreenRecorderPro_Setup_v$Version.exe" -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Green
