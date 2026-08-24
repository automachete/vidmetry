[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,

    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($Tag -notmatch '^v(?<Version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$') {
    throw "Release tag must use the form v1.2.3 or v1.2.3-prerelease: $Tag"
}

$version = $Matches.Version
$root = [System.IO.Path]::GetFullPath($RepositoryRoot)
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

function Set-SingleVersion {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RelativePath,
        [Parameter(Mandatory = $true)]
        [string]$Pattern,
        [Parameter(Mandatory = $true)]
        [string]$Label
    )

    $path = Join-Path $root $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Release version target is missing: $RelativePath"
    }

    $content = [System.IO.File]::ReadAllText($path)
    $expression = [regex]::new($Pattern)
    $matches = $expression.Matches($content)
    if ($matches.Count -ne 1) {
        throw "$Label must have exactly one release-version field in $RelativePath; found $($matches.Count)."
    }

    $updated = $expression.Replace(
        $content,
        { param($match) "$($match.Groups['Prefix'].Value)$version$($match.Groups['Suffix'].Value)" },
        1
    )
    if ($updated -cne $content) {
        [System.IO.File]::WriteAllText($path, $updated, $utf8NoBom)
    }
}

Set-SingleVersion 'package.json' '(?m)^(?<Prefix>  "version":\s*")[^"]+(?<Suffix>",\r?$)' 'npm package'
Set-SingleVersion 'package-lock.json' '(?m)^(?<Prefix>  "version":\s*")[^"]+(?<Suffix>",\r?$)' 'npm lock root'
Set-SingleVersion 'package-lock.json' '(?s)(?<Prefix>"packages"\s*:\s*\{\s*""\s*:\s*\{\s*"name"\s*:\s*"vidmetry"\s*,\s*"version"\s*:\s*")[^"]+(?<Suffix>")' 'npm lock workspace'
Set-SingleVersion 'src-tauri/Cargo.toml' '(?ms)(?<Prefix>^\[package\]\r?\nname\s*=\s*"vidmetry"\r?\nversion\s*=\s*")[^"]+(?<Suffix>")' 'Cargo package'
Set-SingleVersion 'src-tauri/Cargo.lock' '(?ms)(?<Prefix>^\[\[package\]\]\r?\nname\s*=\s*"vidmetry"\r?\nversion\s*=\s*")[^"]+(?<Suffix>")' 'Cargo lock package'
Set-SingleVersion 'src-tauri/tauri.conf.json' '(?m)^(?<Prefix>  "version":\s*")[^"]+(?<Suffix>",\r?$)' 'Tauri application'
Set-SingleVersion 'docs/SDD.md' '(?m)^(?<Prefix>\| Product version \| )[0-9A-Za-z.-]+(?<Suffix> \|\r?$)' 'SDD product version'
Set-SingleVersion 'docs/SDD.md' '(?m)^(?<Prefix>### 2\.2 Non-goals for )[0-9A-Za-z.-]+(?<Suffix>\r?$)' 'SDD non-goals version'
Set-SingleVersion 'docs/SDD.md' '(?m)^(?<Prefix>## 14\. Acceptance criteria for )[0-9A-Za-z.-]+(?<Suffix>\r?$)' 'SDD acceptance heading version'
Set-SingleVersion 'docs/SDD.md' '(?m)^(?<Prefix>The )[0-9A-Za-z.-]+(?<Suffix> implementation satisfies AC-001)' 'SDD acceptance summary version'
Set-SingleVersion 'docs/VERIFICATION.md' '(?m)^(?<Prefix># Vidmetry )[0-9A-Za-z.-]+(?<Suffix> Verification Record\r?$)' 'verification title version'
Set-SingleVersion 'docs/VERIFICATION.md' '(?<Prefix>generated from the verified )[0-9A-Za-z.-]+(?<Suffix> source tree)' 'verification source-tree version'
Set-SingleVersion 'docs/VERIFICATION.md' '(?<Prefix>`Vidmetry_)[0-9A-Za-z.-]+(?<Suffix>\.0_x64\.msix`)' 'verification MSIX artifact version'

& (Join-Path $PSScriptRoot 'check-release-version.ps1') -Tag $Tag -RepositoryRoot $root
Write-Output "Synchronized release version $version from tag $Tag."
