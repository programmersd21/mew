$ErrorActionPreference = "Stop"
$InstallDir = Join-Path $HOME ".local\bin"
$ThemeDir = Join-Path $env:APPDATA "mew"
$ThemeFile = Join-Path $ThemeDir "theme.toml"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "mew: cargo not found. installing rustup..."
    $RustupInit = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $RustupInit -UseBasicParsing
    & $RustupInit -y --default-toolchain stable
    $CargoBin = Join-Path $HOME ".cargo\bin"
    $env:Path = "$CargoBin;$env:Path"
}

Write-Host "mew: compiling release binary..."
cargo build --release --locked

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Write-Host "mew: installing binary to $InstallDir..."
Copy-Item "target\release\mew.exe" (Join-Path $InstallDir "mew.exe") -Force

$PathParts = $env:Path -split ";"
if ($PathParts -notcontains $InstallDir) {
    Write-Host "mew: adding $InstallDir to user PATH..."
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    }
    $env:Path = "$env:Path;$InstallDir"
}

if (-not (Test-Path $ThemeFile)) {
    Write-Host "mew: provisioning default theme at $ThemeFile..."
    New-Item -ItemType Directory -Force -Path $ThemeDir | Out-Null
    # the annotated template is the single source of truth — never a stale copy
    $Template = & "$InstallDir\mew.exe" theme --raw
    [System.IO.File]::WriteAllText($ThemeFile, ($Template -join "`n") + "`n", (New-Object System.Text.UTF8Encoding($false)))
}

Write-Host "mew: installation complete. run 'mew hook powershell' to integrate with your prompt."
