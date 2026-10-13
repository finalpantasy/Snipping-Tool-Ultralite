# Guest-only preflight. Never launches GUI or accesses the clipboard.
$ErrorActionPreference = 'Stop'
if ($env:USERNAME -ne 'WDAGUtilityAccount' -or $PSScriptRoot -ne 'C:\UL\Input') {
    throw 'This preflight must run from the mapped input folder inside Windows Sandbox.'
}
$exe = Join-Path $PSScriptRoot 'snipping-tool-ul.exe'
$expected = (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'expected-sha256.txt') -Raw).Trim()
if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $expected) { throw 'Executable hash mismatch.' }
$env:SNIPPING_TOOL_UL_CONFIG_DIR = Join-Path $env:LOCALAPPDATA 'SnippingToolUL-IsolatedTest'
function Invoke-SafeCli([string]$Arguments) {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $exe
    $info.Arguments = $Arguments
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $child = New-Object Diagnostics.Process
    $child.StartInfo = $info
    try {
        [void]$child.Start()
        $stdout = $child.StandardOutput.ReadToEndAsync()
        $stderr = $child.StandardError.ReadToEndAsync()
        if (!$child.WaitForExit(15000)) { $child.Kill(); throw 'Owned CLI preflight timed out.' }
        [pscustomobject]@{Code=$child.ExitCode;Output=$stdout.Result;Error=$stderr.Result}
    } finally { $child.Dispose() }
}
$version = Invoke-SafeCli '--version'
if ($version.Code -ne 0) { throw 'Version check failed.' }
if ((Invoke-SafeCli '--help').Code -ne 0) { throw 'Help check failed.' }
$forbiddenFile = Join-Path $env:TEMP ('ul-invalid-region-'+[guid]::NewGuid().ToString('N')+'.png')
$invalid = Invoke-SafeCli ('capture --region 0,0,0,1 --save "'+$forbiddenFile+'"')
if ($invalid.Code -ne 2 -or (Test-Path -LiteralPath $forbiddenFile)) { throw 'Invalid-region guard failed.' }
$result = [ordered]@{version=$version.Output.Trim();sha256=$expected;preflight_passed=$true;native_workflows_tested=$false;uploads_tested=$false;host_clipboard_shared=$false}
$report = Join-Path 'C:\UL\Results' ('preflight-'+[guid]::NewGuid().ToString('N')+'.json')
$result | ConvertTo-Json | Set-Content -LiteralPath $report -Encoding utf8
# The shortcut launches the portable app with test-only preferences when opened manually.
$desktop = [Environment]::GetFolderPath('Desktop')
@'
@echo off
set "SNIPPING_TOOL_UL_CONFIG_DIR=%LOCALAPPDATA%\SnippingToolUL-IsolatedTest"
"C:\UL\Input\snipping-tool-ul.exe"
'@ | Set-Content -LiteralPath (Join-Path $desktop 'Open Snipping Tool UL.cmd') -Encoding ascii
Copy-Item -LiteralPath 'C:\UL\Input\docs\SANDBOX-TESTING.md' -Destination $desktop
