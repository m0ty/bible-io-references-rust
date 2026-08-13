param(
    [Parameter(Mandatory = $true)]
    [string]$DartCheckout
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$dartRoot = (Resolve-Path -LiteralPath $DartCheckout).Path
$dartManifest = Join-Path $dartRoot 'pubspec.yaml'
if (-not (Test-Path -LiteralPath $dartManifest -PathType Leaf)) {
    throw "DartCheckout does not contain pubspec.yaml: $dartRoot"
}

Push-Location $repoRoot
try {
    Push-Location $dartRoot
    try {
        & dart pub get
        if ($LASTEXITCODE -ne 0) { throw 'dart pub get failed' }
    }
    finally {
        Pop-Location
    }

    $cargoMetadata = (& cargo metadata --no-deps --format-version 1 --locked) |
        ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $package = $cargoMetadata.packages |
        Where-Object { $_.name -eq 'bible-io-references' } |
        Select-Object -First 1
    $parity = $package.metadata.'dart-parity'
    $actualCommit = (& git -c "safe.directory=$dartRoot" -C $dartRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'unable to read the Dart checkout commit' }

    $versionLine = Get-Content -LiteralPath $dartManifest |
        Where-Object { $_ -match '^version:\s*' } |
        Select-Object -First 1
    $actualVersion = ($versionLine -replace '^version:\s*', '').Trim()
    if ($actualCommit -ne $parity.commit) {
        throw "Dart upstream moved: pinned $($parity.commit), checkout $actualCommit. Review upstream and update the Rust parity baseline."
    }
    if ($actualVersion -ne $parity.version) {
        throw "Dart version differs: pinned $($parity.version), checkout $actualVersion."
    }

    & cargo build --all-features --locked --bin bible-io-references
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

    $packageConfig = Join-Path (Join-Path $dartRoot '.dart_tool') 'package_config.json'
    $parityProgram = Join-Path (Join-Path $repoRoot 'tools') (Join-Path 'parity' 'dart_rust_parity.dart')
    $isWindowsHost = $env:OS -eq 'Windows_NT'
    if (Get-Variable -Name IsWindows -ErrorAction SilentlyContinue) {
        $isWindowsHost = $isWindowsHost -or $IsWindows
    }
    $binaryName = if ($isWindowsHost) {
        'bible-io-references.exe'
    }
    else {
        'bible-io-references'
    }
    $rustBinary = Join-Path $repoRoot (Join-Path 'target' (Join-Path 'debug' $binaryName))
    & dart "--packages=$packageConfig" $parityProgram $rustBinary
    if ($LASTEXITCODE -ne 0) { throw 'Dart/Rust behavioral parity check failed' }

    Write-Host "Dart parity baseline verified at $actualCommit (v$actualVersion)."
}
finally {
    Pop-Location
}
