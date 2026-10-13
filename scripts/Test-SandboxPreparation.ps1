$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot ('..\target\sandbox-kit-test-'+[guid]::NewGuid().ToString('N'))
$package = Join-Path $root 'package & fixture'
$output = Join-Path $root 'output & fixture'
New-Item -ItemType Directory -Path $package | Out-Null
'Inert fixture. Never executed.' | Set-Content -LiteralPath (Join-Path $package 'snipping-tool-ul.exe') -Encoding ascii
& (Join-Path $PSScriptRoot 'Prepare-SandboxTest.ps1') -Package $package -Output $output | Out-Null
[xml]$config = Get-Content -LiteralPath (Join-Path $output 'Snipping-Tool-UL.wsb') -Raw
foreach ($field in @('Networking','ClipboardRedirection','AudioInput','VideoInput','PrinterRedirection')) {
    if ($config.Configuration.$field -ne 'Disable') { throw "Unexpected sharing: $field" }
}
$mapped = @($config.Configuration.MappedFolders.MappedFolder)
if ($mapped.Count -ne 2 -or $mapped[0].ReadOnly -ne 'true' -or $mapped[1].ReadOnly -ne 'false') { throw 'Mapping permissions failed.' }
if ($mapped[0].HostFolder -ne [IO.Path]::GetFullPath((Join-Path $output 'Input')) -or $mapped[1].HostFolder -ne [IO.Path]::GetFullPath((Join-Path $output 'Results'))) { throw 'Unexpected host folder mapping.' }
$repeatBlocked = $false
try { & (Join-Path $PSScriptRoot 'Prepare-SandboxTest.ps1') -Package $package -Output $output | Out-Null }
catch { $repeatBlocked = $_.Exception.Message -like 'Choose a new output folder*' }
if (!$repeatBlocked) { throw 'Existing evidence was not protected.' }
$guardBlocked = $false
try { & (Join-Path $PSScriptRoot 'Start-SandboxCheck.ps1') }
catch { $guardBlocked = $_.Exception.Message -like 'This preflight must run*' }
if (!$guardBlocked) { throw 'Guest preflight did not reject the host.' }
foreach ($name in @('Prepare-SandboxTest.ps1','Start-SandboxCheck.ps1')) {
    $tokens=$null; $parseErrors=$null
    [void][Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $name),[ref]$tokens,[ref]$parseErrors)
    if ($parseErrors.Count -ne 0) { throw "Script syntax error: $name" }
}
Write-Output 'Sandbox preparation passed: isolated mappings, disabled sharing/network, escaped paths, overwrite refusal, host guard and script syntax. No guest, GUI or executable fixture was launched.'
