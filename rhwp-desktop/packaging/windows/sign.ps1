param(
  [Parameter(Mandatory = $true)]
  [string]$BinaryPath
)

$ErrorActionPreference = "Stop"

function Find-SignTool {
  $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $kitsRoot = 'C:\Program Files (x86)\Windows Kits\10\bin'
  if (-not (Test-Path -LiteralPath $kitsRoot)) {
    throw 'signtool.exe was not found in PATH or Windows Kits.'
  }

  $candidate = Get-ChildItem -LiteralPath $kitsRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1

  if (-not $candidate) {
    throw 'signtool.exe was not found in Windows Kits.'
  }

  return $candidate.FullName
}

if (-not (Test-Path -LiteralPath $BinaryPath)) {
  throw "Binary to sign was not found: $BinaryPath"
}

$certPath = $env:WINDOWS_CERTIFICATE_PATH
$certPassword = $env:WINDOWS_CERTIFICATE_PASSWORD

if ([string]::IsNullOrWhiteSpace($certPath) -or [string]::IsNullOrWhiteSpace($certPassword)) {
  Write-Host "Skipping Windows signing because certificate environment variables are not set."
  exit 0
}

if (-not (Test-Path -LiteralPath $certPath)) {
  throw "Windows signing certificate was not found: $certPath"
}

$timestampUrl = if ([string]::IsNullOrWhiteSpace($env:WINDOWS_TIMESTAMP_URL)) {
  "http://timestamp.digicert.com"
} else {
  $env:WINDOWS_TIMESTAMP_URL
}

$signTool = Find-SignTool

& $signTool sign `
  /fd SHA256 `
  /tr $timestampUrl `
  /td SHA256 `
  /f $certPath `
  /p $certPassword `
  $BinaryPath

if ($LASTEXITCODE -ne 0) {
  throw "signtool failed with exit code $LASTEXITCODE"
}
