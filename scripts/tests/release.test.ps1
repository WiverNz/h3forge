# Focused Windows PowerShell 5.1 regression. Git is entirely mocked.
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$fixture = Join-Path $root ('.local/release-cli-test-' + [guid]::NewGuid().ToString('N'))
$paths = @(
    'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'web/package.json', 'web/package-lock.json',
    'apps/h3forge-desktop/src-tauri/Cargo.toml', 'apps/h3forge-desktop/src-tauri/tauri.conf.json',
    'crates/h3forge-map/Cargo.toml', 'crates/h3forge-resources/Cargo.toml',
    'crates/h3forge-save/Cargo.toml', 'scripts/release.ps1', 'scripts/versions.ps1'
)
function Reset-Fixture {
    foreach ($relative in $paths) {
        $destination = Join-Path $fixture $relative
        New-Item -ItemType Directory -Force (Split-Path $destination -Parent) | Out-Null
        Copy-Item -LiteralPath (Join-Path $root $relative) -Destination $destination
    }
    foreach ($crate in @('crates/h3forge-map', 'crates/h3forge-resources', 'crates/h3forge-save')) {
        New-Item -ItemType Directory -Force "$fixture/$crate/src" | Out-Null
        Set-Content "$fixture/$crate/src/lib.rs" ''
    }
    New-Item -ItemType Directory -Force "$fixture/apps/h3forge-desktop/src-tauri/src" | Out-Null
    Set-Content "$fixture/apps/h3forge-desktop/src-tauri/src/main.rs" 'fn main() {}'
    $global:H3ForgeReleaseTest = @{ calls = @(); answers = @(); prompts = @(); dirty = $false }
}
function git {
    $global:LASTEXITCODE = 0
    $global:H3ForgeReleaseTest.calls += ,@($args)
    switch ($args[0]) {
        'status' { if ($global:H3ForgeReleaseTest.dirty) { ' M changed.txt' }; return }
        'symbolic-ref' { return 'fixture-branch' }
        'remote' { return 'https://example.invalid/no-push' }
        { $_ -in 'tag', 'ls-remote', 'diff', 'add', 'commit', 'push' } { return }
        default { throw "Unexpected mocked Git call: $args" }
    }
}
function Read-Host {
    param([string]$Prompt)
    $state = $global:H3ForgeReleaseTest
    $state.prompts += $Prompt
    if ($state.answers.Count -eq 0) { throw "Unexpected prompt: $Prompt" }
    $answer = $state.answers[0]
    $state.answers = @($state.answers | Select-Object -Skip 1)
    return $answer
}
function Assert-True($condition, [string]$message) {
    if (!$condition) { throw $message }
}
Push-Location $root
try {
    $tokens = $null; $errors = $null
    $null = [Management.Automation.Language.Parser]::ParseFile("$root/scripts/release.ps1", [ref]$tokens, [ref]$errors)
    Assert-True ($errors.Count -eq 0) 'Release script syntax errors'
    . "$root/scripts/versions.ps1"
    $current = Get-ReleaseVersion
    $parts = $current.Split('.')
    $patch = '{0}.{1}.{2}' -f $parts[0], $parts[1], ([int]$parts[2] + 1)
    $minor = '{0}.{1}.0' -f $parts[0], ([int]$parts[1] + 1)
    $major = '{0}.0.0' -f ([int]$parts[0] + 1)
    $cases = @(
        @{ arguments = @(); version = $patch; answers = @('y'); pushed = $false },
        @{ arguments = @('--push'); version = $patch; answers = @('y', "push v$patch"); pushed = $true },
        @{ arguments = @($minor, '--push'); version = $minor; answers = @('y', ''); pushed = $false },
        @{ arguments = @('-Push', 'minor'); version = $minor; answers = @('y', "push v$minor"); pushed = $true }
    )
    foreach ($case in $cases) {
        Reset-Fixture
        $global:H3ForgeReleaseTest.answers = $case.answers
        $arguments = $case.arguments
        & "$fixture/scripts/release.ps1" @arguments
        Push-Location $fixture
        try { Test-ReleaseMetadata $case.version } finally { Pop-Location }
        $calls = $global:H3ForgeReleaseTest.calls
        Assert-True (@($calls | Where-Object { $_[0] -eq 'commit' }).Count -eq 1) 'Expected one mocked commit'
        $tags = @($calls | Where-Object { $_[0] -eq 'tag' -and $_[1] -eq '-a' })
        Assert-True ($tags.Count -eq 1 -and $tags[0][2] -eq "v$($case.version)") 'Wrong annotated tag'
        $pushes = @($calls | Where-Object { $_[0] -eq 'push' })
        Assert-True ($pushes.Count -eq [int]$case.pushed) 'Push opt-in/confirmation failure'
        if ($case.pushed) {
            Assert-True (($pushes[0] -join ' ') -eq "push --atomic origin HEAD:refs/heads/fixture-branch refs/tags/v$($case.version)") 'Wrong push refs'
        }
        Assert-True ($global:H3ForgeReleaseTest.answers.Count -eq 0) 'Expected confirmation was not requested'
    }
    foreach ($arguments in @(@('major', '--dry-run'), @('patch', '-n'), @('--push', '--dry-run'))) {
        Reset-Fixture
        & "$fixture/scripts/release.ps1" @arguments
        Assert-True ((Get-Content "$fixture/Cargo.toml" -Raw) -ceq (Get-Content "$root/Cargo.toml" -Raw)) 'Dry run changed metadata'
        Assert-True (@($global:H3ForgeReleaseTest.calls | Where-Object { $_[0] -in 'add', 'commit', 'push' -or ($_[0] -eq 'tag' -and $_[1] -eq '-a') }).Count -eq 0) 'Dry run mutated Git'
    }
    Reset-Fixture
    $global:H3ForgeReleaseTest.answers = @('')
    & "$fixture/scripts/release.ps1"
    Assert-True ((Get-Content "$fixture/Cargo.toml" -Raw) -ceq (Get-Content "$root/Cargo.toml" -Raw)) 'Cancellation changed metadata'
    foreach ($arguments in @(@('--unknown'), @('1.2.3', '2.0.0'), @('01.2.3'))) {
        Reset-Fixture
        $rejected = $false
        try { & "$fixture/scripts/release.ps1" @arguments } catch { $rejected = $true }
        Assert-True $rejected 'Invalid arguments accepted'
        Assert-True ($global:H3ForgeReleaseTest.calls.Count -eq 0) 'Invalid arguments reached Git'
    }
    Reset-Fixture
    $global:H3ForgeReleaseTest.dirty = $true
    $rejected = $false
    try { & "$fixture/scripts/release.ps1" --push } catch { $rejected = $_.Exception.Message -like '*Working tree must be clean*' }
    Assert-True $rejected 'Dirty tree was not rejected'
    Write-Host 'PASS: CLI defaults, bump kinds, explicit version, push opt-in/confirmation, cancellation, dry run, invalid arguments and dirty tree. All Git calls mocked.'
} finally { Pop-Location }
