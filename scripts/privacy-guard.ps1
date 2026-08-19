[CmdletBinding()]
param(
    [ValidateSet('Identity', 'Staged', 'Message', 'Range', 'Repository', 'PrePush', 'SelfTest')]
    [string]$Mode = 'Repository',
    [string]$BaseSha,
    [string]$HeadSha = 'HEAD',
    [string]$Revision = 'HEAD',
    [string]$InputPath,
    [string]$RefName,
    [string]$RemoteName,
    [switch]$ScanRepository
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:Findings = [System.Collections.Generic.List[object]]::new()
$script:SafeEmailPattern = '^(?:noreply@github\.com|(?:[0-9]+\+)?[A-Za-z0-9][A-Za-z0-9-]{0,38}(?:\[bot\])?@users\.noreply\.github\.com)$'
$script:SafeNamePattern = '^(?:GitHub|[A-Za-z0-9][A-Za-z0-9._-]{0,62}(?:\[bot\])?)$'
$script:EmailPattern = '(?i)(?<![A-Z0-9._%+\-])[A-Z0-9._%+\-]+@[A-Z][A-Z0-9-]*(?:\.[A-Z0-9-]+)*\.[A-Z]{2,63}(?![A-Z0-9._%+\-])'
$script:WindowsProfilePattern = '(?i)(?:[A-Z]:|\\\\\?\\[A-Z]:)[\\/](?:Users|Documents and Settings)[\\/](?<profile>[^\\/\r\n"''<>|]+)'
$script:PosixProfilePattern = '(?i)/(?:Users|home)/(?<profile>[^/\r\n"''<>|]+)'
$script:InternalAccountPattern = '(?i)(?<![A-Za-z0-9])u[0-9]{5,12}(?![A-Za-z0-9])'

function Invoke-GitCommand {
    param(
        [Parameter(Mandatory)]
        [string[]]$Arguments,
        [switch]$AllowFailure
    )

    $output = @(& git @Arguments 2>$null)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        throw 'Gitの検査コマンドを実行できませんでした。'
    }

    if ($exitCode -ne 0) {
        return @()
    }

    return $output
}

function Add-PrivacyFinding {
    param(
        [Parameter(Mandatory)]
        [string]$Category,
        [Parameter(Mandatory)]
        [string]$Context
    )

    $script:Findings.Add([pscustomobject]@{
        Category = $Category
        Context = $Context
    })
}

function Test-SafeEmail {
    param([Parameter(Mandatory)][string]$Email)
    return $Email -cmatch $script:SafeEmailPattern
}

function Test-SafePublicName {
    param([Parameter(Mandatory)][string]$Name)
    return $Name -cmatch $script:SafeNamePattern
}

function Test-PlaceholderProfile {
    param([Parameter(Mandatory)][string]$Profile)
    return $Profile -match '^(?:example|sample|test)(?:[ ._-][A-Za-z0-9._-]+)*$|^(?:default|public|runneradmin|user|username)$'
}

function Get-LocalSensitiveIdentifiers {
    if ($env:CI -eq 'true') {
        return @()
    }

    $candidates = @(
        $env:USERNAME,
        $env:USERDOMAIN,
        $env:COMPUTERNAME,
        $(if ($env:USERPROFILE) { Split-Path -Leaf $env:USERPROFILE })
    )
    $genericValues = @('actions', 'default', 'desktop', 'github', 'localhost', 'public', 'runner', 'runneradmin', 'user')

    return @($candidates |
        Where-Object { $_ -and $_.Length -ge 4 -and $_.ToLowerInvariant() -notin $genericValues } |
        Sort-Object -Unique)
}

$script:LocalSensitiveIdentifiers = @(Get-LocalSensitiveIdentifiers)

function Get-PrivacyIssueCategories {
    param([AllowEmptyString()][string]$Text)

    $categories = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    if ([string]::IsNullOrEmpty($Text)) {
        return @()
    }

    foreach ($match in [regex]::Matches($Text, $script:EmailPattern)) {
        if (-not (Test-SafeEmail -Email $match.Value)) {
            [void]$categories.Add('許可されていないメールアドレス')
        }
    }

    foreach ($pattern in @($script:WindowsProfilePattern, $script:PosixProfilePattern)) {
        foreach ($match in [regex]::Matches($Text, $pattern)) {
            if (-not (Test-PlaceholderProfile -Profile $match.Groups['profile'].Value.Trim())) {
                [void]$categories.Add('ユーザープロファイルの絶対パス')
            }
        }
    }

    if ($Text -match $script:InternalAccountPattern) {
        [void]$categories.Add('内部アカウント形式の識別子')
    }

    foreach ($identifier in $script:LocalSensitiveIdentifiers) {
        if ($Text.IndexOf($identifier, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
            [void]$categories.Add('ローカル環境由来の識別子')
        }
    }

    return @($categories)
}

function Test-PrivacyText {
    param(
        [AllowEmptyString()][string]$Text,
        [Parameter(Mandatory)][string]$Context
    )

    foreach ($category in @(Get-PrivacyIssueCategories -Text $Text)) {
        Add-PrivacyFinding -Category $category -Context $Context
    }
}

function Test-IdentityValues {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Email,
        [Parameter(Mandatory)][string]$Context
    )

    if (-not (Test-SafePublicName -Name $Name)) {
        Add-PrivacyFinding -Category '公開ハンドル形式ではないGit表示名' -Context $Context
    }
    if (-not (Test-SafeEmail -Email $Email)) {
        Add-PrivacyFinding -Category 'GitHub noreply形式ではないGitメール' -Context $Context
    }
    Test-PrivacyText -Text $Name -Context $Context
    Test-PrivacyText -Text $Email -Context $Context
}

function Test-CurrentIdentity {
    foreach ($identityVariable in @('GIT_AUTHOR_IDENT', 'GIT_COMMITTER_IDENT')) {
        $identity = (@(Invoke-GitCommand -Arguments @('var', $identityVariable)) -join "`n").TrimEnd()
        if ($identity -notmatch '^(?<name>.*) <(?<email>[^>]*)> [0-9]+ [+-][0-9]{4}$') {
            Add-PrivacyFinding -Category '解釈できないGit identity' -Context 'ローカルGit設定'
            continue
        }
        Test-IdentityValues -Name $Matches['name'] -Email $Matches['email'] -Context 'ローカルGit設定'
    }
}

function Resolve-Commit {
    param([Parameter(Mandatory)][string]$Value)
    $resolved = @(Invoke-GitCommand -Arguments @('rev-parse', '--verify', "$Value^{commit}") -AllowFailure)
    if ($resolved.Count -eq 0) {
        return $null
    }
    return $resolved[0].Trim()
}

function Test-DiffLines {
    param(
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]]$Lines,
        [Parameter(Mandatory)][string]$Context
    )

    foreach ($line in $Lines) {
        if ($line.StartsWith('+', [System.StringComparison]::Ordinal) -and
            -not $line.StartsWith('+++', [System.StringComparison]::Ordinal)) {
            Test-PrivacyText -Text $line.Substring(1) -Context $Context
        }
    }
}

function Test-Commit {
    param([Parameter(Mandatory)][string]$Commit)

    $shortCommit = (@(Invoke-GitCommand -Arguments @('rev-parse', '--short=12', $Commit)) -join '').Trim()
    $context = "commit $shortCommit"
    $authorName = (@(Invoke-GitCommand -Arguments @('show', '-s', '--format=%an', $Commit)) -join "`n").TrimEnd()
    $authorEmail = (@(Invoke-GitCommand -Arguments @('show', '-s', '--format=%ae', $Commit)) -join "`n").TrimEnd()
    $committerName = (@(Invoke-GitCommand -Arguments @('show', '-s', '--format=%cn', $Commit)) -join "`n").TrimEnd()
    $committerEmail = (@(Invoke-GitCommand -Arguments @('show', '-s', '--format=%ce', $Commit)) -join "`n").TrimEnd()

    Test-IdentityValues -Name $authorName -Email $authorEmail -Context "$context author"
    Test-IdentityValues -Name $committerName -Email $committerEmail -Context "$context committer"

    $message = @(Invoke-GitCommand -Arguments @('show', '-s', '--format=%B', $Commit)) -join "`n"
    Test-PrivacyText -Text $message -Context "$context message"

    $changedPaths = @(Invoke-GitCommand -Arguments @('diff-tree', '--root', '--no-commit-id', '--name-only', '-r', '--diff-filter=ACMR', $Commit))
    foreach ($path in $changedPaths) {
        Test-PrivacyText -Text $path -Context "$context path"
    }

    $diff = @(Invoke-GitCommand -Arguments @('show', '--format=', '--no-color', '--no-ext-diff', '--unified=0', '--diff-filter=ACMR', $Commit))
    Test-DiffLines -Lines $diff -Context "$context content"
}

function Test-Commits {
    param([Parameter(Mandatory)][string[]]$Commits)

    foreach ($commit in @($Commits | Where-Object { $_ } | Sort-Object -Unique)) {
        Test-Commit -Commit $commit
    }
}

function Get-RangeCommits {
    param(
        [string]$Base,
        [Parameter(Mandatory)][string]$Head
    )

    $headCommit = Resolve-Commit -Value $Head
    if (-not $headCommit) {
        throw '検査対象のcommitを解決できませんでした。'
    }

    $baseCommit = $null
    if ($Base -and $Base -notmatch '^0+$') {
        $baseCommit = Resolve-Commit -Value $Base
    }

    if (-not $baseCommit) {
        $defaultRemoteRef = @(Invoke-GitCommand -Arguments @('rev-parse', '--verify', 'refs/remotes/origin/main') -AllowFailure)
        if ($defaultRemoteRef.Count -gt 0) {
            $mergeBase = @(Invoke-GitCommand -Arguments @('merge-base', $defaultRemoteRef[0].Trim(), $headCommit) -AllowFailure)
            if ($mergeBase.Count -gt 0 -and $mergeBase[0].Trim() -ne $headCommit) {
                $baseCommit = $mergeBase[0].Trim()
            }
        }
    }

    if ($baseCommit) {
        $commits = @(Invoke-GitCommand -Arguments @('rev-list', '--reverse', "$baseCommit..$headCommit"))
    } else {
        $commits = @($headCommit)
    }

    if ($commits.Count -eq 0) {
        return @($headCommit)
    }
    return $commits
}

function Test-TagIdentity {
    param([string]$TagRef)

    if (-not $TagRef -or -not $TagRef.StartsWith('refs/tags/', [System.StringComparison]::Ordinal)) {
        return
    }

    $fields = @(Invoke-GitCommand -Arguments @('for-each-ref', '--format=%(objecttype)%09%(taggername)%09%(taggeremail)', $TagRef) -AllowFailure)
    if ($fields.Count -eq 0) {
        return
    }

    $parts = $fields[0] -split "`t", 3
    if ($parts.Count -eq 3 -and $parts[0] -eq 'tag') {
        Add-PrivacyFinding -Category 'tagger metadataを持つ注釈付きtag' -Context 'annotated tag'
        $email = $parts[2].Trim()
        if ($email.StartsWith('<') -and $email.EndsWith('>')) {
            $email = $email.Substring(1, $email.Length - 2)
        }
        Test-IdentityValues -Name $parts[1] -Email $email -Context 'annotated tag'
        $tagMessage = @(Invoke-GitCommand -Arguments @('for-each-ref', '--format=%(contents)', $TagRef) -AllowFailure) -join "`n"
        Test-PrivacyText -Text $tagMessage -Context 'annotated tag message'
    }
}

function Test-StagedChanges {
    $paths = @(Invoke-GitCommand -Arguments @('diff', '--cached', '--name-only', '--diff-filter=ACMR'))
    foreach ($path in $paths) {
        Test-PrivacyText -Text $path -Context 'staged path'
    }

    $diff = @(Invoke-GitCommand -Arguments @('diff', '--cached', '--no-color', '--no-ext-diff', '--unified=0', '--diff-filter=ACMR'))
    Test-DiffLines -Lines $diff -Context 'staged content'
}

function Test-TrackedRepository {
    $root = (@(Invoke-GitCommand -Arguments @('rev-parse', '--show-toplevel')) -join '').Trim()
    foreach ($path in @(Invoke-GitCommand -Arguments @('ls-files'))) {
        Test-PrivacyText -Text $path -Context 'tracked path'
        $fullPath = Join-Path $root $path
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }

        $bytes = [System.IO.File]::ReadAllBytes($fullPath)
        $utf8Text = [System.Text.Encoding]::UTF8.GetString($bytes)
        Test-PrivacyText -Text $utf8Text -Context 'tracked content'
        if ($bytes -contains 0) {
            $utf16Text = [System.Text.Encoding]::Unicode.GetString($bytes)
            Test-PrivacyText -Text $utf16Text -Context 'tracked binary metadata'
        }
    }
}

function Test-CommitMessageFile {
    if (-not $InputPath -or -not (Test-Path -LiteralPath $InputPath -PathType Leaf)) {
        throw 'commit messageファイルを読み取れませんでした。'
    }
    Test-PrivacyText -Text ([System.IO.File]::ReadAllText($InputPath)) -Context 'commit message'
}

function Test-PrePushInput {
    $inputText = [Console]::In.ReadToEnd()
    foreach ($line in @($inputText -split "\r?\n" | Where-Object { $_ })) {
        $fields = $line -split '\s+'
        if ($fields.Count -ne 4) {
            Add-PrivacyFinding -Category '解釈できないpush ref' -Context 'pre-push'
            continue
        }

        $localRef = $fields[0]
        $localSha = $fields[1]
        $remoteSha = $fields[3]
        if ($localSha -match '^0+$') {
            continue
        }

        Test-PrivacyText -Text $localRef -Context 'push ref'
        Test-TagIdentity -TagRef $localRef
        $localCommit = Resolve-Commit -Value $localSha
        if (-not $localCommit) {
            Add-PrivacyFinding -Category 'commitを指さないpush ref' -Context 'pre-push'
            continue
        }

        $commits = @()
        if ($remoteSha -notmatch '^0+$' -and (Resolve-Commit -Value $remoteSha)) {
            $commits = @(Invoke-GitCommand -Arguments @('rev-list', '--reverse', "$remoteSha..$localCommit"))
        } else {
            $arguments = @('rev-list', '--reverse', $localCommit, '--not', '--remotes')
            if ($RemoteName) {
                $arguments[-1] = "--remotes=$RemoteName"
            }
            $commits = @(Invoke-GitCommand -Arguments $arguments)
        }

        if ($commits.Count -eq 0) {
            $commits = @($localCommit)
        }
        Test-Commits -Commits $commits
    }
}

function Invoke-SelfTest {
    $at = [char]64
    $separator = [char]92
    $safeEmail = '123456+public-handle' + $at + 'users.noreply.github.com'
    $privateEmail = 'private.person' + $at + 'example.test'
    $placeholderPath = 'C:' + $separator + 'Users' + $separator + 'Example User' + $separator + 'clip.mp4'
    $privatePath = 'C:' + $separator + 'Users' + $separator + 'Private Person' + $separator + 'clip.mp4'
    $internalIdentifier = 'u' + '7654321'

    if (@(Get-PrivacyIssueCategories -Text $safeEmail).Count -ne 0) { throw 'privacy guard self-test failed (safe email).' }
    if (@(Get-PrivacyIssueCategories -Text $privateEmail).Count -eq 0) { throw 'privacy guard self-test failed (email).' }
    if (@(Get-PrivacyIssueCategories -Text $placeholderPath).Count -ne 0) { throw 'privacy guard self-test failed (placeholder path).' }
    if (@(Get-PrivacyIssueCategories -Text $privatePath).Count -eq 0) { throw 'privacy guard self-test failed (profile path).' }
    if (@(Get-PrivacyIssueCategories -Text $internalIdentifier).Count -eq 0) { throw 'privacy guard self-test failed (account identifier).' }
    if (-not (Test-SafePublicName -Name 'public-handle')) { throw 'privacy guard self-test failed (safe name).' }
    if (Test-SafePublicName -Name 'Private Person') { throw 'privacy guard self-test failed (private name).' }
    Write-Host 'Privacy guard self-test passed.'
}

switch ($Mode) {
    'Identity' {
        Test-CurrentIdentity
    }
    'Staged' {
        Test-CurrentIdentity
        Test-StagedChanges
    }
    'Message' {
        Test-CurrentIdentity
        Test-CommitMessageFile
    }
    'Range' {
        Test-Commits -Commits @(Get-RangeCommits -Base $BaseSha -Head $HeadSha)
        Test-TagIdentity -TagRef $RefName
        if ($ScanRepository) {
            Test-TrackedRepository
        }
    }
    'Repository' {
        $commit = Resolve-Commit -Value $Revision
        if (-not $commit) { throw '検査対象のcommitを解決できませんでした。' }
        Test-Commit -Commit $commit
        Test-TagIdentity -TagRef $RefName
        Test-TrackedRepository
    }
    'PrePush' {
        Test-CurrentIdentity
        Test-PrePushInput
    }
    'SelfTest' {
        Invoke-SelfTest
    }
}

if ($script:Findings.Count -gt 0) {
    Write-Host 'Privacy guard rejected the change. Detected values are intentionally omitted from this log.'
    foreach ($group in @($script:Findings | Group-Object Category, Context)) {
        Write-Host "- $($group.Name): $($group.Count)"
    }
    exit 1
}

if ($Mode -ne 'SelfTest') {
    Write-Host "Privacy guard passed ($Mode)."
}
