[CmdletBinding()]
param([switch]$IncludeExamples, [string[]]$Examples=@())
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).ProviderPath
$previousFlags = $env:CARGO_ENCODED_RUSTFLAGS
try {
    $flags = @()
    foreach ($path in @($env:USERPROFILE, $env:CARGO_HOME)) {
        if ($path) {
            $flags += "--remap-path-prefix=$path=/build/user"
            $flags += "--remap-path-prefix=$($path.Replace('\','/'))=/build/user"
        }
    }
    $flags += "--remap-path-prefix=$projectRoot=/build/workspace"
    $flags += "--remap-path-prefix=$($projectRoot.Replace('\','/'))=/build/workspace"
    $env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
    Push-Location -LiteralPath $projectRoot
    try {
        $buildArgs = @('build','--release','--locked','--bins','-j','4')
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) { throw 'Portable release build failed.' }
        # Keep example-only dependency features out of the shipped binary.
        if ($IncludeExamples -or $Examples.Count -gt 0) {
            $exampleArgs = @('build','--release','--locked','-j','4')
            if ($IncludeExamples) { $exampleArgs += '--examples' }
            foreach ($example in $Examples) { $exampleArgs += @('--example', $example) }
            & cargo @exampleArgs
            if ($LASTEXITCODE -ne 0) { throw 'Portable example build failed.' }
        }
    } finally { Pop-Location }
} finally { $env:CARGO_ENCODED_RUSTFLAGS = $previousFlags }
