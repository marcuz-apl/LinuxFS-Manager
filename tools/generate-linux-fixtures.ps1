[CmdletBinding()]
param(
    [string]$OutputDirectory = ""
)

$repoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repoRoot "tests\fixtures-linux\generated"
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$fullOutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if ($fullOutputDirectory -notmatch '^(?<drive>[A-Za-z]):\\(?<path>.*)$') {
    throw "The fixture output directory must be on a Windows drive that WSL can access."
}
$wslOutputDirectory = "/mnt/$($Matches.drive.ToLowerInvariant())/$($Matches.path.Replace('\', '/'))"

$fullRepoRoot = [IO.Path]::GetFullPath($repoRoot)
if ($fullRepoRoot -notmatch '^(?<repoDrive>[A-Za-z]):\\(?<repoPath>.*)$') {
    throw "The repository root must be on a Windows drive that WSL can access."
}
$wslScriptPath = "/mnt/$($Matches.repoDrive.ToLowerInvariant())/$($Matches.repoPath.Replace('\', '/'))/tools/generate-linux-fixtures.sh"
& wsl.exe -e bash $wslScriptPath $wslOutputDirectory
if ($LASTEXITCODE -ne 0) {
    throw "Linux fixture generation failed with exit code $LASTEXITCODE."
}

Write-Output "Generated fixtures in $OutputDirectory"
