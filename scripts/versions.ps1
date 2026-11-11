# Shared release metadata checks. Compatible with Windows PowerShell 5.1.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-Checked {
    param([string]$Command, [string[]]$Arguments)
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command failed with exit code $LASTEXITCODE" }
}

function Get-ReleaseVersion {
    $manifest = Get-Content Cargo.toml -Raw
    $match = [regex]::Match($manifest, '(?ms)^\[workspace\.package\]\s*\r?\nversion = "([^"]+)"')
    if (!$match.Success) { throw 'Workspace version not found.' }
    return $match.Groups[1].Value
}

function Assert-ReleaseVersion {
    param([string]$Version)
    # Stable three-part versions only, also bounded for Windows version resources.
    if ($Version -cnotmatch '^(0|[1-9][0-9]{0,4})\.(0|[1-9][0-9]{0,4})\.(0|[1-9][0-9]{0,4})$') {
        throw 'Use a stable version such as 0.2.0 (no v prefix or prerelease suffix).'
    }
    foreach ($part in $Version.Split('.')) {
        if ([int]$part -gt 65535) { throw 'Version components must not exceed 65535.' }
    }
}

function Test-ReleaseMetadata {
    param([string]$Version)
    Assert-ReleaseVersion $Version
    $package = Get-Content web/package.json -Raw | ConvertFrom-Json
    # Windows PowerShell 5.1 cannot ConvertFrom-Json the lockfile's empty root key.
    $lockVersions = Invoke-Checked node @('-p', "const p = require('./web/package-lock.json'); [p.version, p.packages[''].version].join('\n')")
    $tauri = Get-Content apps/h3forge-desktop/src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    foreach ($actual in (@((Get-ReleaseVersion), $package.version, $tauri.version) + @($lockVersions))) {
        if ($actual -cne $Version) { throw "Metadata version '$actual' does not match '$Version'." }
    }
    $metadata = (Invoke-Checked cargo @('metadata', '--locked', '--offline', '--no-deps', '--format-version', '1')) | ConvertFrom-Json
    $cargoLock = Get-Content Cargo.lock -Raw
    foreach ($package in $metadata.packages) {
        if ($package.id -in $metadata.workspace_members) {
            $pattern = '(?m)^name = "' + [regex]::Escape($package.name) + '"\r?\nversion = "' + [regex]::Escape($Version) + '"\r?$'
            if ($package.version -cne $Version -or $cargoLock -cnotmatch $pattern) {
                throw "Workspace member $($package.name) or its lockfile entry has a different version."
            }
        }
    }
    if ($tauri.bundle.active -or $tauri.bundle.resources.Count -ne 0 -or $tauri.bundle.PSObject.Properties['externalBin']) {
        throw 'Portable releases must not enable installers, resources or sidecars.'
    }
}
