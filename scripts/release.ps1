# Usage: .\scripts\release.ps1 [patch|minor|major|VERSION] [--push] [--dry-run]
# This prepares Git metadata only. The pushed tag builds on GitHub Actions.
. "$PSScriptRoot/versions.ps1"
$versionSpec = $null
$push = $false
$dryRun = $false
foreach ($argument in $args) {
    switch -Exact ($argument) {
        { $_ -in '--push', '-Push' } { $push = $true; break }
        { $_ -in '--dry-run', '-DryRun', '-n' } { $dryRun = $true; break }
        { $_ -in '--help', '-Help', '-h' } {
            Write-Host 'Usage: .\scripts\release.ps1 [patch|minor|major|VERSION] [--push] [--dry-run]'
            Write-Host 'Defaults to patch. Without --push, commit/tag stay local. Push always requires confirmation.'
            return
        }
        default {
            if ($argument.StartsWith('-')) { throw "Unknown option: $argument" }
            if ($null -ne $versionSpec) { throw 'Supply only one version or bump kind.' }
            $versionSpec = $argument
        }
    }
}
if ($null -eq $versionSpec) { $versionSpec = 'patch' }
if ($versionSpec -notin @('patch', 'minor', 'major')) { Assert-ReleaseVersion $versionSpec }
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $current = Get-ReleaseVersion
    Assert-ReleaseVersion $current
    $parts = $current.Split('.')
    $Version = switch ($versionSpec) {
        'patch' { '{0}.{1}.{2}' -f $parts[0], $parts[1], ([int]$parts[2] + 1) }
        'minor' { '{0}.{1}.0' -f $parts[0], ([int]$parts[1] + 1) }
        'major' { '{0}.0.0' -f ([int]$parts[0] + 1) }
        default { $versionSpec }
    }
    Assert-ReleaseVersion $Version
    $dirty = Invoke-Checked git @('status', '--porcelain', '--untracked-files=all')
    if ($dirty) { throw 'Working tree must be clean, including untracked files.' }
    $branch = Invoke-Checked git @('symbolic-ref', '--quiet', '--short', 'HEAD')
    $tag = "v$Version"
    if (Invoke-Checked git @('tag', '--list', $tag)) { throw "Tag $tag already exists locally." }
    if ($push) {
        $origin = Invoke-Checked git @('remote', 'get-url', 'origin')
        if (Invoke-Checked git @('ls-remote', '--tags', 'origin', "refs/tags/$tag")) { throw "Tag $tag already exists on origin." }
    }
    Test-ReleaseMetadata $current
    if ([version]$Version -le [version]$current) { throw "Version must be newer than $current." }

    Write-Host "Release $current -> $Version on $branch; annotated tag $tag; push: $push. No local build."
    if ($dryRun) { Write-Host 'Dry run: no files, Git objects or remote refs changed.'; return }
    if ((Read-Host 'Prepare the release commit and tag? [y/N]') -notmatch '^[yY]$') {
        Write-Host 'Cancelled. Nothing changed.'
        return
    }

    $path = Join-Path (Get-Location) 'Cargo.toml'
    $content = [IO.File]::ReadAllText($path)
    $content = [regex]::Replace($content, '(?ms)(^\[workspace\.package\]\s*\r?\nversion = ")[^"]+', { param($match) $match.Groups[1].Value + $Version })
    [IO.File]::WriteAllText($path, $content, (New-Object Text.UTF8Encoding $false))
    $path = Join-Path (Get-Location) 'apps/h3forge-desktop/src-tauri/tauri.conf.json'
    $content = [IO.File]::ReadAllText($path).Replace("`"version`": `"$current`"", "`"version`": `"$Version`"")
    [IO.File]::WriteAllText($path, $content, (New-Object Text.UTF8Encoding $false))
    Invoke-Checked npm.cmd @('--prefix', 'web', 'version', $Version, '--no-git-tag-version', '--ignore-scripts', '--cache', '.local/npm-cache')
    # Resolve workspace version changes from the existing offline lock, without compiling.
    Invoke-Checked cargo @('update', '--workspace', '--offline')
    Test-ReleaseMetadata $Version
    $files = @('Cargo.toml', 'Cargo.lock', 'web/package.json', 'web/package-lock.json', 'apps/h3forge-desktop/src-tauri/tauri.conf.json')
    Invoke-Checked git (@('diff', '--check', '--') + $files)
    Invoke-Checked git (@('diff', '--stat', '--') + $files)
    Invoke-Checked git (@('add', '--') + $files)
    Invoke-Checked git @('commit', '-m', "Release $tag")
    Invoke-Checked git @('tag', '-a', $tag, '-m', "Release $tag")

    Write-Host "Created local release commit and annotated tag $tag."
    $pushCommand = "git push --atomic origin HEAD:refs/heads/$branch refs/tags/$tag"
    Write-Host "To push this prepared release manually: $pushCommand"
    if ($push) {
        Write-Host "Pushing $branch and $tag to $origin will BUILD AND PUBLISH a GitHub Release."
        $answer = Read-Host "Type 'push $tag' to confirm, or press Enter to keep everything local"
        if ($answer -ceq "push $tag") {
            Invoke-Checked git @('push', '--atomic', 'origin', "HEAD:refs/heads/$branch", "refs/tags/$tag")
        } else { Write-Host 'Nothing pushed. The release commit and tag remain local.' }
    } else {
        Write-Host 'No --push supplied. The release commit and tag remain local.'
    }
} finally { Pop-Location }
