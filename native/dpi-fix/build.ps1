# Builds the 32-bit dinput8.dll and copies it to where the app embeds it.
# Needs: rustup target add i686-pc-windows-msvc
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cargo build --release
Copy-Item "target\i686-pc-windows-msvc\release\dinput8.dll" "..\..\src-tauri\core\assets\dinput8.dll"
Write-Host "dinput8.dll -> src-tauri\core\assets"
