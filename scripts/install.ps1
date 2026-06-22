[CmdletBinding()]
param(
  [string]$Version = $env:TASKMANAGER_VERSION,
  [string]$Repo = $(if ($env:TASKMANAGER_REPO) { $env:TASKMANAGER_REPO } else { "smileQiny/taskmanager" }),
  [string]$DownloadDir = $env:TASKMANAGER_DOWNLOAD_DIR,
  [switch]$DryRun,
  [switch]$Silent
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Version)) {
  $Version = "latest"
}

if ($Version -eq "latest") {
  $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
} else {
  $apiUrl = "https://api.github.com/repos/$Repo/releases/tags/$Version"
}

Write-Host "Fetching release metadata from $apiUrl"
$headers = @{
  Accept = "application/vnd.github+json"
  "User-Agent" = "taskmanager-installer"
}

$release = Invoke-RestMethod -Uri $apiUrl -Headers $headers
$asset = $release.assets |
  Where-Object { $_.name -match 'x64.*setup\.exe$|setup\.exe$|\.exe$' } |
  Select-Object -First 1

if (-not $asset) {
  Write-Error "No Windows installer asset found in release $($release.tag_name). Available assets: $($release.assets.name -join ', ')"
}

if ([string]::IsNullOrWhiteSpace($DownloadDir)) {
  $DownloadDir = Join-Path ([System.IO.Path]::GetTempPath()) "taskmanager-installer"
}

New-Item -ItemType Directory -Force -Path $DownloadDir | Out-Null
$installerPath = Join-Path $DownloadDir $asset.name

Write-Host "Selected $($asset.name) from $($release.tag_name)"
if ($DryRun -or $env:TASKMANAGER_DRY_RUN -eq "1") {
  Write-Host "Dry run: would download $($asset.browser_download_url)"
  exit 0
}

Write-Host "Downloading to $installerPath"
Invoke-WebRequest -Uri $asset.browser_download_url -Headers @{ "User-Agent" = "taskmanager-installer" } -OutFile $installerPath

$arguments = @()
if ($Silent) {
  $arguments += "/S"
}

Write-Host "Starting installer..."
$process = Start-Process -FilePath $installerPath -ArgumentList $arguments -Wait -PassThru

if ($process.ExitCode -ne 0) {
  throw "Installer exited with code $($process.ExitCode)"
}

Write-Host "Task Manager installer completed."
