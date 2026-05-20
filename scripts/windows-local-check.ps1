param(
  [switch]$SkipWeb,
  [switch]$ForceRust
)

$ErrorActionPreference = "Continue"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$failures = 0
$blockers = 0

function Invoke-CheckedStep {
  param(
    [string]$Name,
    [scriptblock]$Block
  )

  Write-Host "==> $Name"
  & $Block
  if ($LASTEXITCODE -ne 0) {
    Write-Host "FAIL: $Name (exit $LASTEXITCODE)" -ForegroundColor Red
    $script:failures += 1
  } else {
    Write-Host "PASS: $Name" -ForegroundColor Green
  }
}

Push-Location $root
try {
  Write-Host "RHWP Windows local check"
  Write-Host "Root: $root"

  $hasNonAsciiPath = $false
  foreach ($ch in $root.ToString().ToCharArray()) {
    if ([int]$ch -gt 127) {
      $hasNonAsciiPath = $true
      break
    }
  }

  $rustup = Get-Command rustup -ErrorAction SilentlyContinue
  $cargo = Get-Command cargo -ErrorAction SilentlyContinue
  $link = Get-Command link.exe -ErrorAction SilentlyContinue

  if ($rustup) {
    $activeToolchain = (& rustup show active-toolchain 2>$null)
    Write-Host "Rust toolchain: $activeToolchain"
  } else {
    Write-Host "BLOCKED: rustup not found" -ForegroundColor Yellow
    $blockers += 1
  }

  if (-not $cargo) {
    Write-Host "BLOCKED: cargo not found" -ForegroundColor Yellow
    $blockers += 1
  }

  if ($hasNonAsciiPath -and $activeToolchain -match "windows-gnu") {
    Write-Host "BLOCKED: GNU Rust is running under a non-ASCII path. Use an ASCII checkout path or MSVC." -ForegroundColor Yellow
    $blockers += 1
  }

  if (-not $link) {
    Write-Host "BLOCKED: link.exe not found. Install Visual Studio Build Tools C++ build tools for MSVC verification." -ForegroundColor Yellow
    $blockers += 1
  } else {
    Write-Host "MSVC linker: $($link.Source)"
  }

  if (-not $SkipWeb) {
    Invoke-CheckedStep "rhwp-studio web build" {
      Push-Location (Join-Path $root "rhwp-studio")
      try {
        cmd /c npm run build
      } finally {
        Pop-Location
      }
    }
  }

  if ($ForceRust -or $blockers -eq 0) {
    $rustPrefix = @()
    if ($link) {
      $rustPrefix = @("+stable-x86_64-pc-windows-msvc")
    }

    Invoke-CheckedStep "cargo test" {
      cargo @rustPrefix test
    }
    Invoke-CheckedStep "cargo clippy" {
      cargo @rustPrefix clippy -- -D warnings
    }
  } else {
    Write-Host "SKIP: Rust verification skipped because toolchain blockers were detected. Use -ForceRust to run anyway." -ForegroundColor Yellow
  }
} finally {
  Pop-Location
}

if ($failures -gt 0) {
  exit 1
}

if ($blockers -gt 0 -and -not $ForceRust) {
  exit 2
}

exit 0
