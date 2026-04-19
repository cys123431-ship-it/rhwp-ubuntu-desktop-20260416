param(
  [string]$ReleaseDirectory = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\release\windows')).Path ''),
  [string]$CertificateDirectory = (Join-Path $env:LOCALAPPDATA 'rhwp-signing'),
  [string]$PfxPassword,
  [switch]$TrustCurrentUser
)

$ErrorActionPreference = 'Stop'

function Find-SignTool {
  $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $kitsRoot = 'C:\Program Files (x86)\Windows Kits\10\bin'
  $candidate = Get-ChildItem -LiteralPath $kitsRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1

  if (-not $candidate) {
    throw 'signtool.exe was not found.'
  }

  return $candidate.FullName
}

if (-not (Test-Path -LiteralPath $ReleaseDirectory)) {
  throw "Release directory was not found: $ReleaseDirectory"
}

$createScript = Join-Path $PSScriptRoot 'create-self-signed-cert.ps1'
$signScript = Join-Path $PSScriptRoot 'sign.ps1'
$trustScript = Join-Path $PSScriptRoot 'trust-self-signed-cert.ps1'
$signTool = Find-SignTool

$certInfo = & $createScript -OutputDirectory $CertificateDirectory -PfxPassword $PfxPassword
$env:WINDOWS_CERTIFICATE_PATH = $certInfo.PfxPath
$env:WINDOWS_CERTIFICATE_PASSWORD = $certInfo.PfxPassword

if ($TrustCurrentUser) {
  & $trustScript -CertificatePath $certInfo.CertificatePath | Out-Null
}

$artifacts = Get-ChildItem -LiteralPath $ReleaseDirectory -File |
  Where-Object { $_.Extension -in @('.exe', '.msi') } |
  Sort-Object Name

if (-not $artifacts) {
  throw "No Windows installers were found in $ReleaseDirectory"
}

foreach ($artifact in $artifacts) {
  & $signScript -BinaryPath $artifact.FullName
  $signature = Get-AuthenticodeSignature -FilePath $artifact.FullName
  if ($signature.Status -eq 'NotSigned') {
    throw "Artifact is not signed: $($artifact.Name)"
  }
  if (-not $signature.SignerCertificate) {
    throw "No signer certificate was attached to: $($artifact.Name)"
  }
  if ($signature.SignerCertificate.Thumbprint -ne $certInfo.Thumbprint) {
    throw "Unexpected signer thumbprint on: $($artifact.Name)"
  }

  if ($TrustCurrentUser) {
    & $signTool verify /pa $artifact.FullName | Out-Null
    if ($LASTEXITCODE -ne 0) {
      throw "signtool verification failed for $($artifact.Name)"
    }
  }
}

$publicCertPath = Join-Path $ReleaseDirectory 'rhwp-self-signed-code-signing.cer'
Copy-Item -LiteralPath $certInfo.CertificatePath -Destination $publicCertPath -Force

$hashTargets = $artifacts.FullName + $publicCertPath
foreach ($target in $hashTargets) {
  $hash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash
  $hashFile = "$target.sha256"
  Set-Content -LiteralPath $hashFile -NoNewline -Value "$hash *$([System.IO.Path]::GetFileName($target))"
}

[pscustomobject]@{
  ReleaseDirectory = $ReleaseDirectory
  CertificatePath = $publicCertPath
  Thumbprint = $certInfo.Thumbprint
  NotAfter = $certInfo.NotAfter
  SignedArtifacts = $artifacts.Name
}
