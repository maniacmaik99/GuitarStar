# Stores the update signing key as GitHub Actions secrets, so the release
# workflow can sign updates. Run once by the repository owner:
#   powershell -ExecutionPolicy Bypass -File scripts\set-release-secrets.ps1
# The key was created with:  npx tauri signer generate -w $HOME\.tauri\guitarstar.key
# Keep ~\.tauri\guitarstar.key and its .password file backed up: without them,
# installed copies of GuitarStar cannot receive updates any more.
$ErrorActionPreference = "Stop"
$repo = "maniacmaik99/GuitarStar"
$dir = Join-Path $HOME ".tauri"
$gh = "C:\Program Files\GitHub CLI\gh.exe"
if (-not (Test-Path $gh)) { $gh = "gh" }

Get-Content (Join-Path $dir "guitarstar.key") -Raw | & $gh secret set TAURI_SIGNING_PRIVATE_KEY -R $repo
Get-Content (Join-Path $dir "guitarstar.key.password") -Raw | & $gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD -R $repo
& $gh secret list -R $repo
