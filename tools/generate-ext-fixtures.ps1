param(
    [string]$OutputDirectory = "tests/fixtures-ext/generated"
)

New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$windowsDirectory = (Resolve-Path $OutputDirectory).Path
$wslDirectory = (wsl wslpath -a ($windowsDirectory -replace "\\", "/")).Trim()

foreach ($filesystem in @("ext2", "ext3", "ext4")) {
    $wslOutput = "$wslDirectory/$filesystem.img"
    wsl -e bash -lc "set -e; rm -f '$wslOutput'; dd if=/dev/zero of='$wslOutput' bs=1M count=32 status=none; mkfs.$filesystem -F -q -L LINUXFS_$filesystem '$wslOutput'"
    if ($LASTEXITCODE -ne 0) { throw "failed to generate $filesystem fixture" }
}

Write-Host "Generated Ext2/Ext3/Ext4 fixtures in $windowsDirectory"