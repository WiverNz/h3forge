# One release build, shared by local verification and the tag workflow.
. "$PSScriptRoot/versions.ps1"
Push-Location (Split-Path $PSScriptRoot -Parent)
$oldFlags = $env:CARGO_ENCODED_RUSTFLAGS
$oldDebug = $env:CARGO_PROFILE_RELEASE_DEBUG
$oldStrip = $env:CARGO_PROFILE_RELEASE_STRIP
try {
    Test-ReleaseMetadata (Get-ReleaseVersion)
    $root = (Get-Location).Path
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $sysroot = Invoke-Checked rustc @('--print', 'sysroot')
    $env:CARGO_ENCODED_RUSTFLAGS = @(
        "--remap-path-prefix=$root=H3Forge",
        "--remap-path-prefix=$($root.Replace('\', '/'))=H3Forge",
        "--remap-path-prefix=$cargoHome=cargo",
        "--remap-path-prefix=$($cargoHome.Replace('\', '/'))=cargo",
        "--remap-path-prefix=$sysroot=rust",
        "--remap-path-prefix=$($sysroot.Replace('\', '/'))=rust",
        '-C', 'target-feature=+crt-static'
    ) -join [char]31
    $env:CARGO_PROFILE_RELEASE_DEBUG = '0'
    $env:CARGO_PROFILE_RELEASE_STRIP = 'symbols'
    Invoke-Checked cargo @('fetch', '--locked', '--target', 'x86_64-pc-windows-msvc')
    # Tauri's beforeBuildCommand runs the React typecheck and Vite production build.
    Invoke-Checked npm.cmd @('--prefix', 'web', 'run', 'desktop:build', '--', '--target', 'x86_64-pc-windows-msvc', '--', '--locked')
    New-Item -ItemType Directory -Force .local/release | Out-Null
    Copy-Item target/x86_64-pc-windows-msvc/release/H3Forge.exe .local/release/H3Forge.exe
    $hash = (Get-FileHash .local/release/H3Forge.exe -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $root '.local/release/H3Forge.exe.sha256'), "$hash  H3Forge.exe`n", (New-Object Text.UTF8Encoding $false))
    Write-Host "Portable artifacts: .local/release/H3Forge.exe and H3Forge.exe.sha256"
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $oldFlags
    $env:CARGO_PROFILE_RELEASE_DEBUG = $oldDebug
    $env:CARGO_PROFILE_RELEASE_STRIP = $oldStrip
    Pop-Location
}
