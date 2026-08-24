[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,

    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
if ($Tag -notmatch '^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
    throw "Release tag must use the form v1.2.3 or v1.2.3-prerelease: $Tag"
}

$branch = (& git -C $projectRoot branch --show-current).Trim()
if ($LASTEXITCODE -ne 0 -or $branch -cne 'main') {
    throw 'Release tags must be created from main.'
}
if (@(& git -C $projectRoot status --porcelain).Count -ne 0) {
    throw 'The working tree must be clean before preparing a release tag.'
}

& git -C $projectRoot fetch origin main --tags
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to refresh origin/main and release tags.'
}
$head = (& git -C $projectRoot rev-parse HEAD).Trim()
$remoteMain = (& git -C $projectRoot rev-parse origin/main).Trim()
if ($head -cne $remoteMain) {
    throw 'Local main must exactly match origin/main before preparing a release tag.'
}
& git -C $projectRoot rev-parse --verify --quiet "refs/tags/$Tag" *> $null
if ($LASTEXITCODE -eq 0) {
    throw "The local tag already exists: $Tag"
}
$remoteTag = @(& git -C $projectRoot ls-remote --tags origin "refs/tags/$Tag")
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to inspect remote release tags.'
}
if ($remoteTag.Count -ne 0) {
    throw "The remote tag already exists: $Tag"
}

& (Join-Path $PSScriptRoot 'set-release-version.ps1') -Tag $Tag -RepositoryRoot $projectRoot

$versionFiles = @(
    'package.json',
    'package-lock.json',
    'src-tauri/Cargo.toml',
    'src-tauri/Cargo.lock',
    'src-tauri/tauri.conf.json',
    'docs/SDD.md',
    'docs/VERIFICATION.md'
)
& git -C $projectRoot add -- $versionFiles
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to stage synchronized release-version files.'
}

& git -C $projectRoot diff --cached --quiet
if ($LASTEXITCODE -eq 1) {
    & git -C $projectRoot commit -m "chore: release $Tag"
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to commit synchronized release-version files.'
    }
} elseif ($LASTEXITCODE -ne 0) {
    throw 'Unable to inspect staged release-version changes.'
}

& git -C $projectRoot tag $Tag
if ($LASTEXITCODE -ne 0) {
    throw "Unable to create lightweight tag $Tag."
}

& git -C $projectRoot push --atomic origin 'HEAD:refs/heads/main' "refs/tags/${Tag}:refs/tags/${Tag}"
if ($LASTEXITCODE -ne 0) {
    throw 'Atomic main/tag push failed; local commit and tag were retained for inspection or retry.'
}

Write-Output "Created and atomically pushed release tag $Tag with synchronized source and documentation versions."
