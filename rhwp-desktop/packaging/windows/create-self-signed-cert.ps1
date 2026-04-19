param(
  [string]$OutputDirectory = (Join-Path $env:LOCALAPPDATA 'rhwp-signing'),
  [string]$Subject = 'CN=rhwp contributors (Self-Signed)',
  [string]$FriendlyName = 'rhwp self-signed code signing',
  [string]$PfxPassword
)

$ErrorActionPreference = 'Stop'

function New-RandomPassword {
  $chars = @()
  $chars += 48..57
  $chars += 65..90
  $chars += 97..122
  $chars += 35,36,37,42,43,45,46,58,61,64,95
  return -join (1..48 | ForEach-Object { [char]($chars | Get-Random) })
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

if ([string]::IsNullOrWhiteSpace($PfxPassword)) {
  $PfxPassword = New-RandomPassword
}

$securePassword = ConvertTo-SecureString -String $PfxPassword -AsPlainText -Force
$existingCert = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object {
    $_.Subject -eq $Subject -and
    $_.FriendlyName -eq $FriendlyName -and
    $_.HasPrivateKey -and
    $_.NotAfter -gt (Get-Date).AddDays(30)
  } |
  Sort-Object NotAfter -Descending |
  Select-Object -First 1

if (-not $existingCert) {
  $existingCert = New-SelfSignedCertificate `
    -CertStoreLocation 'Cert:\CurrentUser\My' `
    -Type CodeSigningCert `
    -Subject $Subject `
    -FriendlyName $FriendlyName `
    -KeyAlgorithm RSA `
    -KeyLength 4096 `
    -HashAlgorithm SHA256 `
    -KeyExportPolicy Exportable `
    -NotAfter (Get-Date).AddYears(3)
}

$cerPath = Join-Path $OutputDirectory 'rhwp-self-signed-code-signing.cer'
$pfxPath = Join-Path $OutputDirectory 'rhwp-self-signed-code-signing.pfx'

Export-Certificate -Cert $existingCert -FilePath $cerPath -Force | Out-Null
Export-PfxCertificate -Cert $existingCert -FilePath $pfxPath -Password $securePassword -Force | Out-Null

[pscustomobject]@{
  Subject = $existingCert.Subject
  Thumbprint = $existingCert.Thumbprint
  CertificatePath = $cerPath
  PfxPath = $pfxPath
  PfxPassword = $PfxPassword
  NotAfter = $existingCert.NotAfter.ToString('o')
}
