param(
  [Parameter(Mandatory = $true)]
  [string]$CertificatePath
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $CertificatePath)) {
  throw "Certificate was not found: $CertificatePath"
}

Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\CurrentUser\Root' | Out-Null
Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\CurrentUser\TrustedPublisher' | Out-Null

[pscustomobject]@{
  CertificatePath = $CertificatePath
  TrustedStores = @('CurrentUser\Root', 'CurrentUser\TrustedPublisher')
}
