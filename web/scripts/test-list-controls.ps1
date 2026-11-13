param([string]$Browser)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$outputRoot = Join-Path $projectRoot '.local'
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
if (-not $Browser) {
    $Browser = @(
        'C:/Program Files/Google/Chrome/Application/chrome.exe',
        'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'
    ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $Browser) { throw 'Chrome or Edge is required. Supply -Browser with its executable path.' }
$bundle = Join-Path $outputRoot 'h3f029-browser.js'
& (Join-Path $projectRoot 'web/node_modules/.bin/esbuild.cmd') (Join-Path $projectRoot 'web/src/components/listControls.browser.tsx') --bundle "--outfile=$bundle"
if ($LASTEXITCODE -ne 0) { throw 'Could not bundle list acceptance fixture.' }
$html = Join-Path $outputRoot 'h3f029-browser.html'
'<!DOCTYPE html><html><head><meta charset="UTF-8"><link rel="stylesheet" href="h3f029-browser.css"></head><body><div id="root"></div><pre id="result">running</pre><script src="h3f029-browser.js"></script></body></html>' | Set-Content -Encoding utf8 $html
$profile = Join-Path $outputRoot 'h3f029-chrome'
$screenshot = Join-Path $outputRoot 'h3f029-layout.png'
$stderr = Join-Path $outputRoot 'h3f029-browser-stderr.log'
$dom = Join-Path $outputRoot 'h3f029-browser-dom.html'
# Chrome logs diagnostics on stderr even when successful; judge the fixture result.
$ErrorActionPreference = 'Continue'
& $Browser --headless --disable-gpu --no-first-run --no-default-browser-check --allow-file-access-from-files "--user-data-dir=$profile" --window-size=1280,720 --force-device-scale-factor=1 --virtual-time-budget=12000 "--screenshot=$screenshot" --dump-dom ([System.Uri]$html).AbsoluteUri 2> $stderr | Set-Content $dom
$ErrorActionPreference = 'Stop'
$match = [regex]::Match([System.IO.File]::ReadAllText($dom), '<pre id="result">(.*?)</pre>', 'Singleline')
if (-not $match.Success -or $match.Groups[1].Value -eq 'running') { throw "Browser fixture did not finish. See $dom and $stderr." }
$result = [System.Net.WebUtility]::HtmlDecode($match.Groups[1].Value) | ConvertFrom-Json
if (-not $result.ok) { throw "$($result.error)`n$($result.stack)" }
Write-Output "List UI acceptance passed: $($result.assertions) assertions, $($result.writes) native writes; screenshot: $screenshot"
