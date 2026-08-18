[CmdletBinding()]
param(
    [string]$PackagePath,
    [switch]$LiveInstall
)

$ErrorActionPreference = 'Stop'
$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))

function Get-WindowsSdkTool([string]$Name) {
    $sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $tools = @(Get-ChildItem -LiteralPath $sdkBin -Filter $Name -File -Recurse |
        Where-Object { $_.Directory.Name -ceq 'x64' } |
        Where-Object { $_.Directory.Parent.Name -cmatch '^\d+(\.\d+){1,3}$' } |
        Sort-Object { [Version]($_.Directory.Parent.Name -replace '[^0-9.]', '') } -Descending)
    if ($tools.Count -eq 0) {
        throw "$Name was not found in Windows SDK 10."
    }
    $tools[0].FullName
}

function Get-PeMachine([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $reader = [System.IO.BinaryReader]::new($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            throw "$Path is not a PE file."
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadInt32()
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "$Path has an invalid PE signature."
        }
        $reader.ReadUInt16()
    } finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Assert-BundleMarker([string]$Path, [string]$Expected) {
    $binaryText = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($Path))
    if (-not $binaryText.Contains($Expected, [StringComparison]::Ordinal)) {
        throw "$Path does not contain the expected Tauri bundle marker $Expected."
    }
}

function Assert-File([string]$Root, [string]$RelativePath) {
    $path = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "The MSIX package is missing $RelativePath."
    }
    $path
}

function Remove-TestCertificate([string]$Store, $Certificate, [switch]$CurrentUser) {
    if ($null -eq $Certificate) {
        return
    }
    if ($CurrentUser) {
        & certutil.exe -user -delstore $Store $Certificate.Thumbprint | Out-Null
    } else {
        & certutil.exe -delstore $Store $Certificate.Thumbprint | Out-Null
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to remove the live-test certificate $($Certificate.Thumbprint) from $Store."
    }
}

function Test-LiveInstallation(
    [string]$SourcePackage,
    [xml]$Manifest,
    [System.Xml.XmlNamespaceManager]$Namespaces,
    [string]$OperationRoot
) {
    $principal = [Security.Principal.WindowsPrincipal]::new(
        [Security.Principal.WindowsIdentity]::GetCurrent()
    )
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'The MSIX live-install test requires an elevated PowerShell session to manage LocalMachine package-signing trust.'
    }

    $identity = $Manifest.SelectSingleNode('/f:Package/f:Identity', $Namespaces)
    if ($identity.Name -cne 'Vidmetry.Dev' -or $identity.Publisher -cne 'CN=Vidmetry Development') {
        throw 'Live installation is restricted to the isolated Vidmetry.Dev package identity.'
    }
    if (@(Get-AppxPackage -Name $identity.Name).Count -ne 0) {
        throw 'Vidmetry.Dev is already installed; refusing to replace an existing development package.'
    }

    $testPackage = Join-Path $OperationRoot 'Vidmetry.LiveTest.msix'
    Copy-Item -LiteralPath $SourcePackage -Destination $testPackage
    $rootCertificate = $null
    $trustedRootCertificate = $null
    $certificate = $null
    $trustedCertificate = $null
    $installedPackage = $null
    $launchedProcess = $null
    try {
        $rootCertificate = New-SelfSignedCertificate `
            -Type Custom `
            -Subject "CN=Vidmetry MSIX live test root $([guid]::NewGuid())" `
            -FriendlyName "Vidmetry MSIX live test root" `
            -KeyAlgorithm RSA `
            -KeyLength 2048 `
            -HashAlgorithm SHA256 `
            -KeyUsage CertSign, CRLSign `
            -TextExtension '2.5.29.19={critical}{text}ca=1&pathlength=0' `
            -CertStoreLocation 'Cert:\CurrentUser\My'
        $certificate = New-SelfSignedCertificate `
            -Type CodeSigningCert `
            -Subject $identity.Publisher `
            -FriendlyName "Vidmetry MSIX live test $([guid]::NewGuid())" `
            -Signer $rootCertificate `
            -KeyAlgorithm RSA `
            -KeyLength 2048 `
            -HashAlgorithm SHA256 `
            -CertStoreLocation 'Cert:\CurrentUser\My'
        $rootCertificatePath = Join-Path $OperationRoot 'Vidmetry.LiveTest.Root.cer'
        $certificatePath = Join-Path $OperationRoot 'Vidmetry.LiveTest.cer'
        Export-Certificate -Cert $rootCertificate -FilePath $rootCertificatePath | Out-Null
        Export-Certificate -Cert $certificate -FilePath $certificatePath | Out-Null
        $trustedRootCertificate = Import-Certificate `
            -FilePath $rootCertificatePath `
            -CertStoreLocation 'Cert:\LocalMachine\Root'
        $trustedCertificate = Import-Certificate `
            -FilePath $certificatePath `
            -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople'

        $signTool = Get-WindowsSdkTool 'signtool.exe'
        & $signTool sign /fd SHA256 /sha1 $certificate.Thumbprint $testPackage | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw 'Unable to sign the live-test MSIX package.'
        }
        $signature = Get-AuthenticodeSignature -LiteralPath $testPackage
        if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
            throw "The live-test package signature is not trusted: $($signature.StatusMessage)"
        }

        Add-AppxPackage -Path $testPackage
        $installedPackage = Get-AppxPackage -Name $identity.Name
        if ($null -eq $installedPackage -or $installedPackage.Status -ne 'Ok') {
            throw 'The live-test MSIX package was not registered successfully.'
        }

        $commandType = [type]::GetTypeFromCLSID([guid]'7CD16804-1388-4150-991B-A977AEA22567')
        $command = [Activator]::CreateInstance($commandType)
        if ($null -eq $command) {
            throw 'The packaged File Explorer command could not be activated.'
        }
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($command)

        if (-not ('VidmetryMsixActivation' -as [type])) {
            Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

[ComImport]
[Guid("2E941141-7F97-4756-BA1D-9DECDE894A3D")]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IApplicationActivationManager
{
    int ActivateApplication(
        [MarshalAs(UnmanagedType.LPWStr)] string appUserModelId,
        [MarshalAs(UnmanagedType.LPWStr)] string arguments,
        uint options,
        out uint processId);
}

[ComImport]
[Guid("45BA127D-10A8-46EA-8AB7-56EA9078943C")]
class ApplicationActivationManager { }

public static class VidmetryMsixActivation
{
    public static uint Activate(string appUserModelId, string arguments)
    {
        var manager = (IApplicationActivationManager)new ApplicationActivationManager();
        uint processId;
        int result = manager.ActivateApplication(appUserModelId, arguments, 0, out processId);
        Marshal.ThrowExceptionForHR(result);
        return processId;
    }
}
'@
        }

        $selectionDirectory = Join-Path $OperationRoot 'selection'
        New-Item -ItemType Directory -Path $selectionDirectory | Out-Null
        $applicationUserModelId = "$($installedPackage.PackageFamilyName)!Vidmetry"
        $processId = [VidmetryMsixActivation]::Activate(
            $applicationUserModelId,
            "`"$selectionDirectory`""
        )
        $launchedProcess = Get-Process -Id $processId -ErrorAction Stop
        if (-not $launchedProcess.WaitForInputIdle(10000)) {
            throw 'The packaged Vidmetry process did not reach an interactive state.'
        }
        if ($launchedProcess.ProcessName -cne 'vidmetry') {
            throw "The packaged activation launched an unexpected process: $($launchedProcess.ProcessName)"
        }
    } finally {
        if ($null -ne $launchedProcess -and -not $launchedProcess.HasExited) {
            Stop-Process -Id $launchedProcess.Id -Force
            $launchedProcess.WaitForExit(10000)
        }
        if ($null -ne $installedPackage) {
            Remove-AppxPackage -Package $installedPackage.PackageFullName
        }
        Remove-TestCertificate 'TrustedPeople' $trustedCertificate
        Remove-TestCertificate 'My' $certificate -CurrentUser
        Remove-TestCertificate 'Root' $trustedRootCertificate
        Remove-TestCertificate 'My' $rootCertificate -CurrentUser
    }
    if (@(Get-AppxPackage -Name $identity.Name).Count -ne 0) {
        throw 'The live-test package remained installed after removal.'
    }
}

if ([string]::IsNullOrWhiteSpace($PackagePath)) {
    $version = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
    $PackagePath = Join-Path $projectRoot "src-tauri\target\release\bundle\msix\Vidmetry_$version.0_x64.msix"
} else {
    $PackagePath = [System.IO.Path]::GetFullPath($PackagePath)
}
if (-not (Test-Path -LiteralPath $PackagePath -PathType Leaf)) {
    throw "MSIX package not found: $PackagePath"
}

$operationRoot = Join-Path ([System.IO.Path]::GetTempPath()) "vidmetry-msix-$([guid]::NewGuid())"
$layout = Join-Path $operationRoot 'layout'
New-Item -ItemType Directory -Path $operationRoot | Out-Null
try {
    $makeAppx = Get-WindowsSdkTool 'makeappx.exe'
    & $makeAppx unpack /p $PackagePath /d $layout | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to unpack the MSIX package for verification.'
    }

    $manifestPath = Assert-File $layout 'AppxManifest.xml'
    [xml]$manifest = Get-Content -LiteralPath $manifestPath -Raw
    $namespaces = [System.Xml.XmlNamespaceManager]::new($manifest.NameTable)
    $namespaces.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
    $namespaces.AddNamespace('com', 'http://schemas.microsoft.com/appx/manifest/com/windows10')
    $namespaces.AddNamespace('desktop2', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10/2')
    $namespaces.AddNamespace('desktop4', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10/4')
    $namespaces.AddNamespace('desktop5', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10/5')
    $namespaces.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
    $namespaces.AddNamespace('uap3', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/3')
    $namespaces.AddNamespace('uap10', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10')
    $namespaces.AddNamespace('rescap', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')

    $packageJson = Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
    $identity = $manifest.SelectSingleNode('/f:Package/f:Identity', $namespaces)
    if ($identity.Version -cne "$($packageJson.version).0" -or $identity.ProcessorArchitecture -cne 'x64') {
        throw "Unexpected MSIX identity version or architecture: $($identity.Version) $($identity.ProcessorArchitecture)"
    }

    $application = $manifest.SelectSingleNode('//f:Application[@Id="Vidmetry"]', $namespaces)
    if ($null -eq $application -or
        $application.Executable -cne 'vidmetry.exe' -or
        $application.GetAttribute('RuntimeBehavior', $namespaces.LookupNamespace('uap10')) -cne 'packagedClassicApp' -or
        $application.GetAttribute('TrustLevel', $namespaces.LookupNamespace('uap10')) -cne 'mediumIL') {
        throw 'The MSIX application activation contract is invalid.'
    }
    if ($null -eq $manifest.SelectSingleNode('//rescap:Capability[@Name="runFullTrust"]', $namespaces)) {
        throw 'The MSIX package is missing the runFullTrust capability.'
    }

    $selectionSource = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\src\selection.rs') -Raw
    $extensionBlock = [regex]::Match(
        $selectionSource,
        'const\s+VIDEO_EXTENSIONS\s*:\s*&\[&str\]\s*=\s*&\[(?<extensions>[\s\S]*?)\];'
    )
    $expectedFileTypes = @([regex]::Matches($extensionBlock.Groups['extensions'].Value, '"(?<extension>[a-z0-9]+)"') |
        ForEach-Object { ".$($_.Groups['extension'].Value)" })
    $association = $manifest.SelectSingleNode('//uap3:FileTypeAssociation[@Name="vidmetryvideo"]', $namespaces)
    $actualFileTypes = @($association.SelectNodes('uap:SupportedFileTypes/uap:FileType', $namespaces) |
        ForEach-Object { $_.InnerText })
    if (($expectedFileTypes -join "`n") -cne ($actualFileTypes -join "`n") -or
        $association.Parameters -cne '"%1"' -or
        $association.MultiSelectModel -cne 'Single' -or
        $association.GetAttribute('AllowSilentDefaultTakeOver', $namespaces.LookupNamespace('desktop2')) -cne 'true') {
        throw 'The MSIX video file-association contract is invalid.'
    }

    $comClass = $manifest.SelectSingleNode('//com:Class', $namespaces)
    $directoryVerb = $manifest.SelectSingleNode('//desktop5:ItemType[@Type="Directory"]/desktop5:Verb', $namespaces)
    if ($null -eq $comClass -or $null -eq $directoryVerb -or
        $comClass.Id -cne $directoryVerb.Clsid -or
        $comClass.Path -cne 'VidmetryExplorerCommand.dll') {
        throw 'The MSIX directory context-menu and COM registrations do not match.'
    }

    $applicationPath = Assert-File $layout 'vidmetry.exe'
    $commandPath = Assert-File $layout 'VidmetryExplorerCommand.dll'
    if ((Get-PeMachine $applicationPath) -ne 0x8664 -or (Get-PeMachine $commandPath) -ne 0x8664) {
        throw 'The MSIX application and Explorer command must both be x64 PE files.'
    }
    Assert-BundleMarker $applicationPath '__TAURI_BUNDLE_TYPE_VAR_UNK'
    foreach ($required in @(
        'ffmpeg.exe',
        'ffprobe.exe',
        'LICENSE',
        'THIRD_PARTY_NOTICES.md',
        'FFmpeg\FFMPEG_LICENSE.txt',
        'FFmpeg\FFMPEG_BUILD_INFO.txt',
        'FFmpeg\FFMPEG_CORRESPONDING_SOURCE.txt',
        'ThirdPartyLicenses\RUST_THIRD_PARTY_LICENSES.html',
        'ThirdPartyLicenses\JAVASCRIPT_THIRD_PARTY_LICENSES.txt',
        'LicenseSources\MPL-2.0\SOURCE_INDEX.txt'
    )) {
        [void](Assert-File $layout $required)
    }

    $ffmpegManifest = Get-Content -LiteralPath (Join-Path $projectRoot 'scripts\ffmpeg-sidecars.json') -Raw | ConvertFrom-Json
    $target = $ffmpegManifest.targets.'x86_64-pc-windows-msvc'
    $actualFfmpegHash = (Get-FileHash -LiteralPath (Join-Path $layout 'ffmpeg.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    $actualFfprobeHash = (Get-FileHash -LiteralPath (Join-Path $layout 'ffprobe.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualFfmpegHash -cne $target.ffmpeg.sha256 -or $actualFfprobeHash -cne $target.ffprobe.sha256) {
        throw 'The packaged FFmpeg sidecar hashes do not match the locked runtime manifest.'
    }
    $forbiddenFiles = @(Get-ChildItem -LiteralPath $layout -File -Recurse |
        Where-Object { $_.Extension -in @('.pdb', '.lib', '.exp', '.iobj', '.ipdb') })
    if ($forbiddenFiles.Count -ne 0) {
        throw "The MSIX package contains build-only files: $($forbiddenFiles.Name -join ', ')"
    }

    if ($LiveInstall) {
        Test-LiveInstallation $PackagePath $manifest $namespaces $operationRoot
        Write-Output 'MSIX live install, COM activation, app launch, shutdown, and uninstall passed.'
    }
    Write-Output "MSIX structure, $($actualFileTypes.Count) video associations, x64 payloads, and locked sidecars are valid."
} finally {
    if (Test-Path -LiteralPath $operationRoot) {
        Remove-Item -LiteralPath $operationRoot -Recurse -Force
    }
}
