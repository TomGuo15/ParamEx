Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")).Path
$SourceExe = Join-Path $RepoRoot "target\release\paramex-gui.exe"
$DistRoot = Join-Path $RepoRoot "target\dist"
$DistDir = Join-Path $DistRoot "ParamEx"
$DestExe = Join-Path $DistDir "ParamEx.exe"
# Stage every copy before touching the previous distribution.
$StagingDir = Join-Path $DistRoot (".ParamEx-staging-" + [Guid]::NewGuid().ToString("N"))
$StagingExe = Join-Path $StagingDir "ParamEx.exe"

function Assert-InRepo {
    param([Parameter(Mandatory = $true)][string]$Path)

    $full = [IO.Path]::GetFullPath($Path)
    $rootWithSlash = $RepoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $full.StartsWith($rootWithSlash, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to touch path outside repository: $full"
    }
}

Assert-InRepo -Path $DistRoot
Assert-InRepo -Path $DistDir
Assert-InRepo -Path $StagingDir

Push-Location $RepoRoot
try {
    # cargo reports progress on stderr. Under Windows PowerShell 5.1 a caller
    # that redirects stderr (`2>&1`) would otherwise turn that progress into a
    # terminating error, so relax the preference for the build call only and
    # rely on the exit code.
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        # Drop a leftover binary so this run always links a new EXE. Stop if
        # the file is locked: otherwise cargo can report Fresh and the script
        # would package the old binary.
        if (Test-Path -LiteralPath $SourceExe) {
            Remove-Item -LiteralPath $SourceExe -Force -ErrorAction Stop
        }
        cargo build --release -p paramex-gui
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Release build failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $SourceExe)) {
    throw "Expected Rust release binary was not created: $SourceExe"
}

$source = Get-Item -LiteralPath $SourceExe

try {
    New-Item -ItemType Directory -Path $StagingDir -Force | Out-Null
    Copy-Item -LiteralPath $SourceExe -Destination $StagingExe -Force
    $packagedAt = Get-Date

    $staged = Get-Item -LiteralPath $StagingExe
    if ($source.Length -ne $staged.Length) {
        throw "Copied EXE size mismatch: source=$($source.Length), dest=$($staged.Length)"
    }
    $stagedBytes = $staged.Length
    $staged.LastWriteTime = $packagedAt

    if (Test-Path -LiteralPath $DistDir) {
        Remove-Item -LiteralPath $DistDir -Recurse -Force
    }
    Move-Item -LiteralPath $StagingDir -Destination $DistDir
    # A successful package owns target/dist: drop old zips, versioned copies,
    # notes, and staging leftovers so only ParamEx/ParamEx.exe remains.
    $keep = [IO.Path]::GetFullPath($DistDir)
    Get-ChildItem -LiteralPath $DistRoot -Force | Where-Object {
        [IO.Path]::GetFullPath($_.FullName) -ne $keep
    } | ForEach-Object {
        Remove-Item -LiteralPath $_.FullName -Recurse -Force
    }
    Write-Host ("Wrote {0} ({1:N2} MiB)" -f $DestExe, ($stagedBytes / 1MB))
}
finally {
    if (Test-Path -LiteralPath $StagingDir) {
        Remove-Item -LiteralPath $StagingDir -Recurse -Force
    }
}
