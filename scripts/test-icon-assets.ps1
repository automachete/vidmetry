$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$svgPath = Join-Path $projectRoot 'assets\app-icon.svg'
$stylesPath = Join-Path $projectRoot 'src\styles.css'

function Assert-AchromaticHexColors([string]$path) {
    $content = Get-Content -LiteralPath $path -Raw
    $matches = [regex]::Matches($content, '#(?<hex>[0-9a-fA-F]{6})(?![0-9a-fA-F])')
    foreach ($match in $matches) {
        $hex = $match.Groups['hex'].Value
        $red = [Convert]::ToInt32($hex.Substring(0, 2), 16)
        $green = [Convert]::ToInt32($hex.Substring(2, 2), 16)
        $blue = [Convert]::ToInt32($hex.Substring(4, 2), 16)
        if ($red -ne $green -or $green -ne $blue) {
            throw "Chromatic fixed color #$hex remains in $path"
        }
    }
}

Add-Type -AssemblyName System.Drawing
$drawingAssembly = [System.Drawing.Bitmap].Assembly.Location
$drawingRoot = Split-Path -Parent $drawingAssembly
$drawingReferences = @(
    $drawingAssembly,
    (Join-Path $drawingRoot 'System.Private.Windows.GdiPlus.dll'),
    (Join-Path $drawingRoot 'System.Private.Windows.Core.dll'),
    (Join-Path $drawingRoot 'System.Drawing.Primitives.dll')
)
Add-Type -ReferencedAssemblies $drawingReferences -TypeDefinition @'
using System;
using System.Drawing;

public static class VidmetryIconAudit
{
    public static string FindChromaticPixel(string path)
    {
        using (var bitmap = new Bitmap(path))
        {
            for (var y = 0; y < bitmap.Height; y++)
            {
                for (var x = 0; x < bitmap.Width; x++)
                {
                    var pixel = bitmap.GetPixel(x, y);
                    if (pixel.A > 8 && (pixel.R != pixel.G || pixel.G != pixel.B))
                    {
                        return String.Format("{0},{1}=#{2:X2}{3:X2}{4:X2}", x, y, pixel.R, pixel.G, pixel.B);
                    }
                }
            }
        }
        return null;
    }

    public static string FindChromaticIcoPixel(string path)
    {
        using (var icon = new Icon(path))
        using (var bitmap = icon.ToBitmap())
        {
            for (var y = 0; y < bitmap.Height; y++)
            {
                for (var x = 0; x < bitmap.Width; x++)
                {
                    var pixel = bitmap.GetPixel(x, y);
                    if (pixel.A > 8 && (pixel.R != pixel.G || pixel.G != pixel.B))
                    {
                        return String.Format("{0},{1}=#{2:X2}{3:X2}{4:X2}", x, y, pixel.R, pixel.G, pixel.B);
                    }
                }
            }
        }
        return null;
    }
}
'@

Assert-AchromaticHexColors $svgPath

$iconRoot = Join-Path $projectRoot 'src-tauri\icons'
Get-ChildItem -LiteralPath $iconRoot -Filter '*.png' -Recurse | ForEach-Object {
    $chromaticPixel = [VidmetryIconAudit]::FindChromaticPixel($_.FullName)
    if ($null -ne $chromaticPixel) {
        throw "Chromatic pixel remains in $($_.FullName): $chromaticPixel"
    }
}

$icoPath = Join-Path $iconRoot 'icon.ico'
$icoPixel = [VidmetryIconAudit]::FindChromaticIcoPixel($icoPath)
if ($null -ne $icoPixel) {
    throw "Chromatic pixel remains in ${icoPath}: $icoPixel"
}

$styles = Get-Content -LiteralPath $stylesPath -Raw
$legacyTints = @(
    '#' + (('8D', 'E0', 'C7') -join ''),
    '#' + (('96', 'A0', '98') -join ''),
    '#' + (('DD', 'FF', 'F4') -join ''),
    '#' + (('1E', '25', '23') -join '')
)
foreach ($legacyTint in $legacyTints) {
    if ($styles.Contains($legacyTint, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Legacy fixed tint $legacyTint remains in $stylesPath"
    }
}

Write-Output 'Achromatic icon assets verified.'
