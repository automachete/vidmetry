$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$operationRoot = Join-Path ([System.IO.Path]::GetTempPath()) "vidmetry-release-version-$([guid]::NewGuid())"
$remoteRoot = Join-Path ([System.IO.Path]::GetTempPath()) "vidmetry-release-remote-$([guid]::NewGuid()).git"
$relativeFiles = @(
    'package.json',
    'package-lock.json',
    'src-tauri/Cargo.toml',
    'src-tauri/Cargo.lock',
    'src-tauri/tauri.conf.json',
    'docs/SDD.md',
    'docs/VERIFICATION.md'
)

try {
    foreach ($relativeFile in $relativeFiles) {
        $source = Join-Path $projectRoot $relativeFile
        $destination = Join-Path $operationRoot $relativeFile
        $destinationDirectory = Split-Path -Parent $destination
        New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $destination
    }

    & (Join-Path $PSScriptRoot 'set-release-version.ps1') -Tag 'v9.8.7' -RepositoryRoot $operationRoot
    & (Join-Path $PSScriptRoot 'check-release-version.ps1') -Tag 'v9.8.7' -RepositoryRoot $operationRoot

    $firstHashes = @{}
    foreach ($relativeFile in $relativeFiles) {
        $firstHashes[$relativeFile] = (Get-FileHash -LiteralPath (Join-Path $operationRoot $relativeFile) -Algorithm SHA256).Hash
    }
    & (Join-Path $PSScriptRoot 'set-release-version.ps1') -Tag 'v9.8.7' -RepositoryRoot $operationRoot
    foreach ($relativeFile in $relativeFiles) {
        $secondHash = (Get-FileHash -LiteralPath (Join-Path $operationRoot $relativeFile) -Algorithm SHA256).Hash
        if ($secondHash -cne $firstHashes[$relativeFile]) {
            throw "Release version synchronization is not idempotent: $relativeFile"
        }
    }

    try {
        & (Join-Path $PSScriptRoot 'set-release-version.ps1') -Tag 'release-9.8.7' -RepositoryRoot $operationRoot
        throw 'An invalid release tag was accepted.'
    } catch {
        if ($_.Exception.Message -ceq 'An invalid release tag was accepted.') {
            throw
        }
    }

    $emptyHooks = Join-Path $operationRoot '.empty-hooks'
    New-Item -ItemType Directory -Path $emptyHooks | Out-Null
    & git init --bare $remoteRoot | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Unable to create the release-tag test remote.' }
    & git -C $operationRoot init -b main | Out-Null
    & git -C $operationRoot config user.name 'Vidmetry release test'
    & git -C $operationRoot config user.email 'release-test@users.noreply.github.com'
    & git -C $operationRoot config core.hooksPath $emptyHooks
    & git -C $operationRoot add -- $relativeFiles
    & git -C $operationRoot commit -m 'test: initialize release source' | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Unable to create the release-tag test commit.' }
    & git -C $operationRoot remote add origin $remoteRoot
    & git -C $operationRoot push -u origin main | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Unable to initialize the release-tag test remote main branch.' }

    & (Join-Path $PSScriptRoot 'create-release-tag.ps1') -Tag 'v9.8.8' -RepositoryRoot $operationRoot
    if ($LASTEXITCODE -ne 0) { throw 'Release-tag creation failed against the local test remote.' }
    $localHead = (& git -C $operationRoot rev-parse HEAD).Trim()
    $remoteHead = (& git -C $operationRoot rev-parse origin/main).Trim()
    $tagHead = (& git -C $operationRoot rev-parse 'refs/tags/v9.8.8').Trim()
    $tagObjectType = (& git -C $operationRoot cat-file -t 'refs/tags/v9.8.8').Trim()
    if ($localHead -cne $remoteHead -or $localHead -cne $tagHead -or $tagObjectType -cne 'commit') {
        throw 'The release commit, remote main, and lightweight tag must resolve to the same commit.'
    }
    & (Join-Path $PSScriptRoot 'check-release-version.ps1') -Tag 'v9.8.8' -RepositoryRoot $operationRoot
    if (@(& git -C $operationRoot status --porcelain).Count -ne 0) {
        throw 'Release-tag creation left the working tree dirty.'
    }

    Write-Output 'Release version synchronization and atomic lightweight-tag contracts passed.'
} finally {
    if (Test-Path -LiteralPath $operationRoot -PathType Container) {
        Remove-Item -LiteralPath $operationRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $remoteRoot -PathType Container) {
        Remove-Item -LiteralPath $remoteRoot -Recurse -Force
    }
}
