$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$tauriDir = Join-Path $root "src-tauri"
$cargoTauri = Join-Path $env:USERPROFILE ".cargo\bin\cargo-tauri.exe"

if (-not (Test-Path $cargoTauri)) {
  throw "cargo-tauri not found at '$cargoTauri'. Install it with: cargo install tauri-cli --locked"
}

$userKey = [Environment]::GetEnvironmentVariable("OPENAI_API_KEY", "User")
if ([string]::IsNullOrWhiteSpace($userKey)) {
  throw "OPENAI_API_KEY is not set in User environment. Set it first: setx OPENAI_API_KEY ""sk-..."""
}

$env:OPENAI_API_KEY = $userKey

Set-Location $tauriDir
& $cargoTauri dev
