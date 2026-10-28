param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("patch", "minor", "major")]
    [string]$Bump = "patch"
)

$ErrorActionPreference = "Stop"

Write-Host "Screen Recorder Pro - Semantic Version Bumper" -ForegroundColor Cyan
$CargoToml = "Cargo.toml"

if (-not (Test-Path $CargoToml)) {
    throw "Cargo.toml not found in current directory."
}

$content = Get-Content $CargoToml -Raw
if ($content -match 'version = "(\d+)\.(\d+)\.(\d+)"') {
    [int]$major = $Matches[1]
    [int]$minor = $Matches[2]
    [int]$patch = $Matches[3]

    switch ($Bump) {
        "major" { $major++; $minor = 0; $patch = 0 }
        "minor" { $minor++; $patch = 0 }
        "patch" { $patch++ }
    }

    $newVersion = "$major.$minor.$patch"
    Write-Host "Bumping version to: $newVersion" -ForegroundColor Green

    $newContent = $content -replace 'version = "\d+\.\d+\.\d+"', "version = `"$newVersion`""
    Set-Content -Path $CargoToml -Value $newContent -NoNewline

    # Update Inno Setup script
    $IssPath = "packaging/windows/installer.iss"
    if (Test-Path $IssPath) {
        $issContent = Get-Content $IssPath -Raw
        $issNew = $issContent -replace '#define MyAppVersion "\d+\.\d+\.\d+"', "#define MyAppVersion `"$newVersion`""
        Set-Content -Path $IssPath -Value $issNew -NoNewline
        Write-Host "Updated $IssPath with version $newVersion" -ForegroundColor Green
    }
} else {
    throw "Version string not found in $CargoToml"
}
