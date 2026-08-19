[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
if ($LASTEXITCODE -ne 0 -or -not $repositoryRoot) {
    throw 'Gitリポジトリを検出できませんでした。'
}

& git -C $repositoryRoot config --local user.useConfigOnly true
if ($LASTEXITCODE -ne 0) { throw 'user.useConfigOnlyを設定できませんでした。' }

& git -C $repositoryRoot config --local core.hooksPath .githooks
if ($LASTEXITCODE -ne 0) { throw 'Git hooks pathを設定できませんでした。' }

& pwsh -NoProfile -File (Join-Path $repositoryRoot 'scripts/privacy-guard.ps1') -Mode Identity
if ($LASTEXITCODE -ne 0) {
    throw 'Git identityを公開ハンドルとGitHub noreplyメールへ修正してください。'
}

Write-Host 'Privacy guard hooks are enabled for this checkout.'
