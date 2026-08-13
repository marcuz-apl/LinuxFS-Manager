[CmdletBinding()]
param(
    [string]$Tag = "",
    [string]$WinFspDll = "",
    [switch]$SkipBuild
)

$repoRoot = Split-Path -Parent $PSScriptRoot
$versionFile = Join-Path $repoRoot "VERSION"
if ([string]::IsNullOrWhiteSpace($Tag)) {
    $version = (Get-Content -LiteralPath $versionFile -Raw).Trim()
    if ([string]::IsNullOrWhiteSpace($version)) {
        throw "VERSION was empty; pass -Tag explicitly."
    }
    $Tag = "{0}-multilang" -f $version
}
$releaseExe = Join-Path $repoRoot "target\release\LinuxFSManager.exe"
$licenseFile = Join-Path $repoRoot "LICENSE"
$noticeFile = Join-Path $repoRoot "NOTICE.md"
$localesSource = Join-Path $repoRoot "locales"
$distRoot = Join-Path $repoRoot "dist"
$packageDir = Join-Path $distRoot ("LinuxFSManager-win64-{0}" -f $Tag)
$zipPath = Join-Path $distRoot ("LinuxFSManager-win64-{0}.zip" -f $Tag)

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        cargo build --release --workspace
        if ($LASTEXITCODE -ne 0) {
            throw "Release build failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $releaseExe -PathType Leaf)) {
    throw "Release executable was not found at $releaseExe."
}
if (-not (Test-Path -LiteralPath $licenseFile -PathType Leaf)) {
    throw "LICENSE was not found at $licenseFile."
}
if (-not (Test-Path -LiteralPath $noticeFile -PathType Leaf)) {
    throw "NOTICE.md was not found at $noticeFile."
}
if (-not (Test-Path -LiteralPath $localesSource -PathType Container)) {
    throw "Locale directory was not found at $localesSource."
}

$dllCandidates = [System.Collections.Generic.List[string]]::new()
if (-not [string]::IsNullOrWhiteSpace($WinFspDll)) {
    $dllCandidates.Add($WinFspDll)
}

foreach ($registryPath in @(
    "HKLM:\SOFTWARE\WOW6432Node\WinFsp",
    "HKLM:\SOFTWARE\WinFsp"
)) {
    try {
        $installDir = (Get-ItemProperty -LiteralPath $registryPath -Name InstallDir -ErrorAction Stop).InstallDir
        if ($installDir) {
            $dllCandidates.Add((Join-Path $installDir "bin\winfsp-x64.dll"))
        }
    }
    catch {
        # Try the next registered architecture/location.
    }
}

# This fallback is for repeatable local rebuilds after a previously reviewed
# WinFsp runtime has already been placed in a generated dist package.
$dllCandidates.Add((Join-Path $distRoot "LinuxFSManager-win64-drive-auto\winfsp-x64.dll"))

$dllSource = $dllCandidates |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    Select-Object -First 1
if (-not $dllSource) {
    throw "winfsp-x64.dll was not found. Install WinFsp or pass -WinFspDll <path>."
}

New-Item -ItemType Directory -Force -Path $packageDir | Out-Null
Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $packageDir "LinuxFSManager.exe") -Force
Copy-Item -LiteralPath $dllSource -Destination (Join-Path $packageDir "winfsp-x64.dll") -Force
Copy-Item -LiteralPath $licenseFile -Destination (Join-Path $packageDir "LICENSE") -Force
Copy-Item -LiteralPath $noticeFile -Destination (Join-Path $packageDir "NOTICE.md") -Force
Copy-Item -LiteralPath $localesSource -Destination (Join-Path $packageDir "locales") -Recurse -Force
Copy-Item -LiteralPath $dllSource -Destination (Join-Path (Split-Path $releaseExe) "winfsp-x64.dll") -Force

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath (Join-Path $packageDir "LinuxFSManager.exe"), (Join-Path $packageDir "winfsp-x64.dll"), (Join-Path $packageDir "LICENSE"), (Join-Path $packageDir "NOTICE.md"), (Join-Path $packageDir "locales") -DestinationPath $zipPath -CompressionLevel Optimal

$packagedExe = Get-Item -LiteralPath (Join-Path $packageDir "LinuxFSManager.exe")
$packagedDll = Get-Item -LiteralPath (Join-Path $packageDir "winfsp-x64.dll")
$packagedLicense = Get-Item -LiteralPath (Join-Path $packageDir "LICENSE")
$packagedNotice = Get-Item -LiteralPath (Join-Path $packageDir "NOTICE.md")
$packagedLocales = Get-ChildItem -LiteralPath (Join-Path $packageDir "locales") -File
$packagedZip = Get-Item -LiteralPath $zipPath
Write-Output ("Package: {0}" -f $packageDir)
Write-Output ("ZIP: {0}" -f $packagedZip.FullName)
Write-Output ("Executable: {0} bytes" -f $packagedExe.Length)
Write-Output ("WinFsp DLL: {0} bytes ({1})" -f $packagedDll.Length, $dllSource)
Write-Output ("License: {0} bytes" -f $packagedLicense.Length)
Write-Output ("Notices: {0} bytes" -f $packagedNotice.Length)
Write-Output ("Locales: {0} UTF-8 TOML files" -f $packagedLocales.Count)
