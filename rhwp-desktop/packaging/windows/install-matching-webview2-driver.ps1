Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$clientId = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$runtimeKeys = @(
  "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$clientId",
  "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\EdgeUpdate\Clients\$clientId",
  "Registry::HKEY_CURRENT_USER\Software\Microsoft\EdgeUpdate\Clients\$clientId"
)
$runtimeKey = $runtimeKeys | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $runtimeKey) {
  throw 'Microsoft Edge WebView2 Runtime registry key was not found.'
}

$runtimeVersion = (Get-ItemProperty $runtimeKey).pv
if ([string]::IsNullOrWhiteSpace($runtimeVersion)) {
  throw 'Microsoft Edge WebView2 Runtime version was not found.'
}

$driverRoot = Join-Path $env:RUNNER_TEMP "msedgedriver-$runtimeVersion"
$driverArchive = "$driverRoot.zip"
$driverUrl = "https://msedgedriver.microsoft.com/$runtimeVersion/edgedriver_win64.zip"

Remove-Item $driverRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $driverArchive -Force -ErrorAction SilentlyContinue
Invoke-WebRequest -Uri $driverUrl -OutFile $driverArchive
Expand-Archive -Path $driverArchive -DestinationPath $driverRoot -Force

$driverPath = Get-ChildItem $driverRoot -Recurse -Filter msedgedriver.exe |
  Select-Object -First 1 -ExpandProperty FullName
if ([string]::IsNullOrWhiteSpace($driverPath) -or -not (Test-Path $driverPath)) {
  throw "Microsoft Edge WebDriver was not found after extracting $driverArchive."
}

Write-Host "WebView2 Runtime: $runtimeVersion"
& $driverPath --version
"RHWP_E2E_NATIVE_DRIVER=$driverPath" |
  Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
