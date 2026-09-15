Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Join-Path ([IO.Path]::GetTempPath()) ("paramex-dist-test-" + [Guid]::NewGuid())
$repo = Join-Path $root "repo"
$script = Join-Path $repo "packaging\windows\build-local-dist.ps1"
$dist = Join-Path $repo "target\dist\ParamEx"

try {
    New-Item -ItemType Directory -Path (Split-Path $script), (Split-Path $dist), (Join-Path $repo "target\release") -Force | Out-Null
    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $repo "target\release\paramex-gui.exe") -Value "stale"
    Set-Content -LiteralPath (Join-Path $dist "sentinel") -Value "keep"
    Set-Content -LiteralPath (Join-Path (Split-Path $dist) "ParamEx-v0.4.0-windows-x64.zip") -Value "old"
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "build-local-dist.ps1") -Destination $script

    function cargo { $global:LASTEXITCODE = 17 }
    try {
        & $script
        throw "expected the release failure to stop packaging"
    }
    catch {
        if ($_.Exception.Message -notmatch "Release build failed with exit code 17") {
            throw
        }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $dist "sentinel"))) {
        throw "failed builds must not replace the existing distribution"
    }
    if (-not (Test-Path -LiteralPath (Join-Path (Split-Path $dist) "ParamEx-v0.4.0-windows-x64.zip"))) {
        throw "failed builds must not wipe leftover files beside the distribution"
    }

    function cargo {
        $global:LASTEXITCODE = 0
        Set-Content -LiteralPath (Join-Path $repo "target\release\paramex-gui.exe") -Value "fresh"
    }
    & $script
    if (-not (Test-Path -LiteralPath (Join-Path $dist "ParamEx.exe"))) {
        throw "successful builds must write ParamEx.exe"
    }
    if (Test-Path -LiteralPath (Join-Path $dist "LICENSE")) {
        throw "the portable distribution must contain only ParamEx.exe"
    }
    $extras = @(Get-ChildItem -LiteralPath (Split-Path $dist) -Force | Where-Object { $_.Name -ne "ParamEx" })
    if ($extras.Count -ne 0) {
        throw "target/dist must contain only ParamEx after a successful package"
    }
    Write-Host "build-local-dist failure guard passed"
}
finally {
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
