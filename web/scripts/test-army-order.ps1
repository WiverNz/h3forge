param([string]$Browser)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$outputRoot = Join-Path $projectRoot '.local'
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
if (-not $Browser) {
    $Browser = @('C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe') |
        Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $Browser) { throw 'Chrome or Edge is required.' }
& (Join-Path $projectRoot 'web/node_modules/.bin/esbuild.cmd') (Join-Path $projectRoot 'web/src/components/armyOrder.browser.tsx') --bundle "--outfile=$outputRoot/army-order.js"
if ($LASTEXITCODE -ne 0) { throw 'Could not bundle army regression.' }
$html = Join-Path $outputRoot 'army-order.html'
'<!DOCTYPE html><html><head><meta charset="UTF-8"><link rel="stylesheet" href="army-order.css"></head><body><div id="root"></div><pre id="result">running</pre><script src="army-order.js"></script></body></html>' | Set-Content -Encoding utf8 $html
$dom = Join-Path $outputRoot 'army-order-dom.html'
$ErrorActionPreference = 'Continue'
& $Browser --headless --disable-gpu --no-first-run --no-default-browser-check --allow-file-access-from-files "--user-data-dir=$outputRoot/army-order-chrome" --window-size=1280,720 --force-device-scale-factor=1 --virtual-time-budget=12000 "--screenshot=$outputRoot/army-order.png" --dump-dom ([System.Uri]$html).AbsoluteUri 2> "$outputRoot/army-order-stderr.log" | Set-Content $dom
$ErrorActionPreference = 'Stop'
$match = [regex]::Match([System.IO.File]::ReadAllText($dom), '<pre id="result">(.*?)</pre>', 'Singleline')
if (-not $match.Success -or $match.Groups[1].Value -eq 'running') { throw "Browser regression did not finish. See $dom." }
$result = [System.Net.WebUtility]::HtmlDecode($match.Groups[1].Value) | ConvertFrom-Json
if (-not $result.ok) { throw "$($result.error)`n$($result.stack)" }
Write-Output "Army regression passed: $($result.assertions) assertions, $($result.edits) slot edits."
