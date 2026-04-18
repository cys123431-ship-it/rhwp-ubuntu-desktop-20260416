param(
  [Parameter(Mandatory = $true)]
  [string]$BinaryPath
)

$ErrorActionPreference = "Stop"

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

$signTool = (Get-Command signtool.exe -ErrorAction Stop).Source

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
