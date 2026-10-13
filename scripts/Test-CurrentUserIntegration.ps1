# Headless sandbox checks. Appx commands are mocked; real Start folders are never used.
$ErrorActionPreference='Stop'
$project=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).ProviderPath
$sandbox=Join-Path $project ('target\Integration.Test.'+[Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $sandbox)
$fixture=Join-Path $sandbox 'snipping-tool-ul.exe'
[IO.File]::WriteAllBytes($fixture,[Text.Encoding]::ASCII.GetBytes('synthetic executable; never executed'))
$scriptText=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'Install-CurrentUser.ps1') -Raw
$programLine='$programs=[Environment]::GetFolderPath(''Programs'')'
if(!$scriptText.Contains($programLine)){throw 'Sandbox isolation anchor changed; refusing test.'}
$scriptText=$scriptText.Replace($programLine,'$programs=$script:fakePrograms')
$block=[ScriptBlock]::Create($scriptText)
$originalLocal=$env:LOCALAPPDATA
function Get-AppxPackage {param([string]$Name); if($Name -ne 'Microsoft.ScreenSketch'){throw 'Unexpected query'}; if($script:fakeMicrosoft){[pscustomobject]@{Name='Microsoft.ScreenSketch';PackageFullName='Microsoft.ScreenSketch.Test'}}}
function Remove-AppxPackage {param([string]$Package,[string]$ErrorAction); if($Package -ne 'Microsoft.ScreenSketch.Test'){throw 'Unexpected removal'}; $script:removals++; if($script:failRemoval){throw 'Simulated removal failure'}; $script:fakeMicrosoft=$false}
function Reset-Scenario([string]$name){
    $env:LOCALAPPDATA=Join-Path $sandbox $name
    $script:fakePrograms=Join-Path $env:LOCALAPPDATA 'Start'
    [void](New-Item -ItemType Directory -Path $script:fakePrograms -Force)
    $script:fakeMicrosoft=$true; $script:removals=0; $script:failRemoval=$false
}
function Assert([bool]$condition,[string]$message){if(!$condition){throw $message}}
try{
    Reset-Scenario 'Preview'
    $plan=& $block -Executable $fixture -Version '0.0.1' -ReplaceWindowsSnippingTool | ConvertFrom-Json
    Assert $plan.previewOnly 'Default install must be a preview'
    Assert ($script:removals -eq 0) 'Preview called removal'
    Assert (!(Test-Path -LiteralPath (Join-Path $env:LOCALAPPDATA 'Programs'))) 'Preview wrote files'

    Reset-Scenario 'Alongside'
    $result=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply | ConvertFrom-Json
    Assert (!$result.replacesWindows -and $script:fakeMicrosoft -and $script:removals -eq 0) 'Normal install removed Microsoft'
    Assert (Test-Path -LiteralPath (Join-Path $script:fakePrograms 'Snipping Tool UL.lnk')) 'Normal shortcut missing'
    Assert (Test-Path -LiteralPath (Join-Path (Split-Path -LiteralPath $result.executable) 'LICENSE')) 'Installed license missing'
    $result=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | ConvertFrom-Json
    Assert ($result.replacesWindows -and !(Test-Path -LiteralPath (Join-Path $script:fakePrograms 'Snipping Tool UL.lnk'))) 'Takeover left a duplicate owned shortcut'

    Reset-Scenario 'Takeover'
    $result=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | ConvertFrom-Json
    Assert ($result.replacesWindows -and !$script:fakeMicrosoft -and $script:removals -eq 1) 'Takeover not scoped to exact package'
    $again=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | ConvertFrom-Json
    Assert ($script:removals -eq 1) 'Idempotent takeover removed again'

    Reset-Scenario 'Conflict'
    $shell=New-Object -ComObject WScript.Shell
    try{$link=$shell.CreateShortcut((Join-Path $script:fakePrograms 'Snipping Tool.lnk'));$link.TargetPath=Join-Path $env:WINDIR 'notepad.exe';$link.Save()}finally{[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($link);[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)}
    $caught=$false;try{& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | Out-Null}catch{$caught=$true}
    Assert ($caught -and $script:fakeMicrosoft -and $script:removals -eq 0) 'Shortcut conflict did not preserve Microsoft'

    Reset-Scenario 'RemovalFailure'; $script:failRemoval=$true
    $caught=$false;try{& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | Out-Null}catch{$caught=$true}
    Assert ($caught -and $script:fakeMicrosoft) 'Failure falsely reported successful removal'
    Assert (Test-Path -LiteralPath (Join-Path $env:LOCALAPPDATA 'Programs\SnippingToolUL\0.0.1\snipping-tool-ul.exe')) 'UL unavailable after removal failure'
    Assert (!(Test-Path -LiteralPath (Join-Path $env:LOCALAPPDATA 'Programs\SnippingToolUL\integration-state.json'))) 'Failure wrote a success state'
    Write-Output 'Five sandbox scenarios passed: preview, alongside, takeover/idempotence, shortcut conflict, removal failure. Appx mocked; no real Start registration or uninstall.'
}finally{$env:LOCALAPPDATA=$originalLocal}
