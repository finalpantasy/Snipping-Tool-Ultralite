[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Package,
    [Parameter(Mandatory)][string]$Output
)
$ErrorActionPreference = 'Stop'
$packagePath = (Resolve-Path -LiteralPath $Package).ProviderPath
$outputPath = [IO.Path]::GetFullPath($Output)
$sourceExe = Join-Path $packagePath 'snipping-tool-ul.exe'
if (!(Test-Path -LiteralPath $sourceExe)) { throw 'A verified portable package is required.' }
if (Test-Path -LiteralPath $outputPath) { throw 'Choose a new output folder; existing evidence is preserved.' }
$inputPath = Join-Path $outputPath 'Input'
$resultsPath = Join-Path $outputPath 'Results'
New-Item -ItemType Directory -Path $inputPath,$resultsPath | Out-Null
Copy-Item -LiteralPath $sourceExe -Destination $inputPath
foreach ($name in @('LICENSE','THIRD_PARTY_NOTICES.md','assets','docs')) {
    $source = Join-Path $packagePath $name
    if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination $inputPath -Recurse }
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Start-SandboxCheck.ps1') -Destination $inputPath
(Get-FileHash -LiteralPath $sourceExe -Algorithm SHA256).Hash | Set-Content -LiteralPath (Join-Path $inputPath 'expected-sha256.txt') -Encoding ascii
$escapedInput = [Security.SecurityElement]::Escape($inputPath)
$escapedResults = [Security.SecurityElement]::Escape($resultsPath)
@"
<Configuration>
  <Networking>Disable</Networking>
  <ClipboardRedirection>Disable</ClipboardRedirection>
  <AudioInput>Disable</AudioInput>
  <VideoInput>Disable</VideoInput>
  <PrinterRedirection>Disable</PrinterRedirection>
  <MemoryInMB>4096</MemoryInMB>
  <MappedFolders>
    <MappedFolder><HostFolder>$escapedInput</HostFolder><SandboxFolder>C:\UL\Input</SandboxFolder><ReadOnly>true</ReadOnly></MappedFolder>
    <MappedFolder><HostFolder>$escapedResults</HostFolder><SandboxFolder>C:\UL\Results</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder>
  </MappedFolders>
  <LogonCommand><Command>powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\UL\Input\Start-SandboxCheck.ps1</Command></LogonCommand>
</Configuration>
"@ | Set-Content -LiteralPath (Join-Path $outputPath 'Snipping-Tool-UL.wsb') -Encoding utf8
Write-Output 'Prepared offline Sandbox kit. Nothing was launched or enabled. Host clipboard, microphone, camera and printers are not shared.'
