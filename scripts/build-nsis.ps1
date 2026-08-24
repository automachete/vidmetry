[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$bundleDirectory = Join-Path $projectRoot 'src-tauri\target\release\bundle\nsis'
$fixedPackageName = 'Vidmetry_x64-setup.exe'
$fixedPackagePath = Join-Path $bundleDirectory $fixedPackageName
$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$versionedPackageName = "Vidmetry_$($version)_x64-setup.exe"

if (Test-Path -LiteralPath $bundleDirectory -PathType Container) {
    foreach ($stalePackage in Get-ChildItem -LiteralPath $bundleDirectory -Filter 'Vidmetry*_x64-setup.exe' -File) {
        Remove-Item -LiteralPath $stalePackage.FullName -Force
    }
}

& npm run tauri bundle -- --bundles nsis
if ($LASTEXITCODE -ne 0) {
    throw 'Tauri failed to build the NSIS package.'
}

$versionedPackagePath = Join-Path $bundleDirectory $versionedPackageName
if (-not (Test-Path -LiteralPath $versionedPackagePath -PathType Leaf)) {
    $generated = @(Get-ChildItem -LiteralPath $bundleDirectory -Filter '*-setup.exe' -File)
    throw "Expected Tauri to generate $versionedPackageName but found: $($generated.Name -join ', ')"
}

Move-Item -LiteralPath $versionedPackagePath -Destination $fixedPackagePath
if (-not (Test-Path -LiteralPath $fixedPackagePath -PathType Leaf)) {
    throw "NSIS package normalization failed: $fixedPackagePath"
}
if (@(Get-ChildItem -LiteralPath $bundleDirectory -Filter '*-setup.exe' -File).Count -ne 1) {
    throw 'The NSIS output directory must contain only the fixed-name setup package.'
}

Write-Output "NSIS package created: $fixedPackagePath"
