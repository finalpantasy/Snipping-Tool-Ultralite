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
$comAnchor='New-Object -ComObject WScript.Shell'
if(!$scriptText.Contains($comAnchor)){throw 'Shortcut isolation anchor changed; refusing test.'}
$scriptText=$scriptText.Replace($comAnchor,'$script:mockShell')
$comCleanupLines=@(
    '    if($replacementLink){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($replacementLink)}',
    '    if($oldLink){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($oldLink)}',
    '    if($verified){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($verified)}',
    '    if($link){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($link)}',
    '    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)'
)
foreach($cleanupLine in $comCleanupLines){
    if(!$scriptText.Contains($cleanupLine)){throw 'Shortcut cleanup isolation anchor changed; refusing test.'}
    $scriptText=$scriptText.Replace($cleanupLine,'')
}
$script:mockShortcuts=@{}
function New-MockShortcut([string]$path){
    $stored=$script:mockShortcuts[$path]
    if($null -eq $stored){$stored=[pscustomobject]@{TargetPath='';WorkingDirectory='';Description='';IconLocation=''}}
    $shortcut=[pscustomobject]@{
        Path=$path
        TargetPath=[string]$stored.TargetPath
        WorkingDirectory=[string]$stored.WorkingDirectory
        Description=[string]$stored.Description
        IconLocation=[string]$stored.IconLocation
    }
    Add-Member -InputObject $shortcut -MemberType ScriptMethod -Name Save -Value {
        $script:mockShortcuts[$this.Path]=[pscustomobject]@{
            TargetPath=$this.TargetPath
            WorkingDirectory=$this.WorkingDirectory
            Description=$this.Description
            IconLocation=$this.IconLocation
        }
        if(!(Test-Path -LiteralPath $this.Path)){[IO.File]::WriteAllText($this.Path,'synthetic shortcut')}
    }
    return $shortcut
}
$script:mockShell=[pscustomobject]@{}
Add-Member -InputObject $script:mockShell -MemberType ScriptMethod -Name CreateShortcut -Value {
    param([string]$path)
    New-MockShortcut $path
}
$block=[ScriptBlock]::Create($scriptText)
$originalLocal=$env:LOCALAPPDATA
function Get-AppxPackage {param([string]$Name); if($Name -ne 'Microsoft.ScreenSketch'){throw 'Unexpected query'}; if($script:fakeMicrosoft){[pscustomobject]@{Name='Microsoft.ScreenSketch';PackageFullName='Microsoft.ScreenSketch.Test'}}}
function Remove-AppxPackage {param([string]$Package,[string]$ErrorAction); if($Package -ne 'Microsoft.ScreenSketch.Test'){throw 'Unexpected removal'}; $script:removals++; if($script:failRemoval){throw 'Simulated removal failure'}; $script:fakeMicrosoft=$false}
# Fail closed if the installer accesses any registry key other than the mock.
function Get-ItemProperty {
    param([string]$LiteralPath,[string]$Name,[string]$ErrorAction)
    if($LiteralPath -ne 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -or $Name -ne 'Snipping Tool UL'){throw 'Unexpected registry read'}
    if($null -ne $script:fakeStartup){[pscustomobject]@{'Snipping Tool UL'=$script:fakeStartup}}
    if($script:changeStartupAfterRead){
        $script:fakeStartup=$script:startupAfterRead
        $script:changeStartupAfterRead=$false
    }
}
function Set-ItemProperty {
    param([string]$LiteralPath,[string]$Name,[string]$Value,[string]$Type,[string]$ErrorAction)
    if($LiteralPath -ne 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -or $Name -ne 'Snipping Tool UL' -or $Type -ne 'String'){throw 'Unexpected registry write'}
    $script:startupWrites++
    $script:fakeStartup=$Value
}
function Reset-Scenario([string]$name){
    $env:LOCALAPPDATA=Join-Path $sandbox $name
    $script:fakePrograms=Join-Path $env:LOCALAPPDATA 'Start'
    [void](New-Item -ItemType Directory -Path $script:fakePrograms -Force)
    $script:mockShortcuts=@{}
    $script:fakeMicrosoft=$true; $script:removals=0; $script:failRemoval=$false
    $script:fakeStartup=$null; $script:startupWrites=0
    $script:changeStartupAfterRead=$false; $script:startupAfterRead=$null
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
    $conflictShortcut=Join-Path $script:fakePrograms 'Snipping Tool.lnk'
    [IO.File]::WriteAllText($conflictShortcut,'synthetic shortcut')
    $script:mockShortcuts[$conflictShortcut]=[pscustomobject]@{TargetPath=(Join-Path $env:WINDIR 'notepad.exe');WorkingDirectory='';Description='';IconLocation=''}
    $caught=$false;try{& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | Out-Null}catch{$caught=$true}
    Assert ($caught -and $script:fakeMicrosoft -and $script:removals -eq 0) 'Shortcut conflict did not preserve Microsoft'

    Reset-Scenario 'RemovalFailure'; $script:failRemoval=$true
    $caught=$false;try{& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | Out-Null}catch{$caught=$true}
    Assert ($caught -and $script:fakeMicrosoft) 'Failure falsely reported successful removal'
    Assert (Test-Path -LiteralPath (Join-Path $env:LOCALAPPDATA 'Programs\SnippingToolUL\0.0.1\snipping-tool-ul.exe')) 'UL unavailable after removal failure'
    Assert (!(Test-Path -LiteralPath (Join-Path $env:LOCALAPPDATA 'Programs\SnippingToolUL\integration-state.json'))) 'Failure wrote a success state'
    Reset-Scenario 'StartupUpgrade'
    $old=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply | ConvertFrom-Json
    $script:fakeStartup='"'+$old.executable+'" --background'
    $new=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | ConvertFrom-Json
    Assert ($script:startupWrites -eq 1 -and $script:fakeStartup -eq ('"'+$new.executable+'" --background')) 'Upgrade left startup pointing at an older release'

    Reset-Scenario 'StartupDisabled'
    & $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | Out-Null
    Assert ($script:startupWrites -eq 0 -and $null -eq $script:fakeStartup) 'Install opted into startup'

    Reset-Scenario 'StartupCustom'
    $custom='"'+(Join-Path $sandbox 'portable\snipping-tool-ul.exe')+'" --background'
    $script:fakeStartup=$custom
    & $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | Out-Null
    Assert ($script:startupWrites -eq 0 -and $script:fakeStartup -eq $custom) 'Install replaced a portable startup command'

    Reset-Scenario 'StartupSibling'
    $custom='"'+(Join-Path $env:LOCALAPPDATA 'Programs\SnippingToolUL-other\0.0.1\snipping-tool-ul.exe')+'" --background'
    $script:fakeStartup=$custom
    & $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | Out-Null
    Assert ($script:startupWrites -eq 0 -and $script:fakeStartup -eq $custom) 'Install confused a sibling folder with an owned release'

    foreach($newPreference in @($null,'"portable.exe" --background')){
        Reset-Scenario ('StartupChanged'+[Guid]::NewGuid().ToString('N'))
        $old=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply | ConvertFrom-Json
        $script:fakeStartup='"'+$old.executable+'" --background'
        $script:changeStartupAfterRead=$true; $script:startupAfterRead=$newPreference
        & $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | Out-Null
        Assert ($script:startupWrites -eq 0 -and $script:fakeStartup -eq $newPreference) 'Install overwrote a newer startup preference'
    }

    Reset-Scenario 'TakeoverUpdate'
    & $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | Out-Null
    # Simulate Microsoft being restored later: a default update cannot remove it.
    $script:fakeMicrosoft=$true
    $updated=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | ConvertFrom-Json
    Assert ($updated.replacesWindows -and $script:removals -eq 1 -and $script:fakeMicrosoft) 'Default update changed takeover intent or removed Microsoft again'
    Assert (!(Test-Path -LiteralPath (Join-Path $script:fakePrograms 'Snipping Tool UL.lnk'))) 'Default update duplicated the takeover shortcut'
    $takeoverShortcut=Join-Path $script:fakePrograms 'Snipping Tool.lnk'
    Assert ($script:mockShortcuts[$takeoverShortcut].TargetPath -eq $updated.executable) 'Takeover shortcut targets an older release'

    foreach($unownedTarget in @('', 'shell:AppsFolder\Microsoft.WindowsNotepad_8wekyb3d8bbwe!App')){
        Reset-Scenario ('UnownedShortcut'+[Guid]::NewGuid().ToString('N'))
        $unownedShortcut=Join-Path $script:fakePrograms 'Snipping Tool.lnk'
        [IO.File]::WriteAllText($unownedShortcut,'synthetic shortcut')
        $script:mockShortcuts[$unownedShortcut]=[pscustomobject]@{TargetPath=$unownedTarget;WorkingDirectory='';Description='';IconLocation=''}
        $sideBySide=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply | ConvertFrom-Json
        Assert (!$sideBySide.replacesWindows -and $script:mockShortcuts[$unownedShortcut].TargetPath -eq $unownedTarget) 'Unowned shortcut target was treated as installer-owned'
        Assert (Test-Path -LiteralPath (Join-Path $script:fakePrograms 'Snipping Tool UL.lnk')) 'Unowned shortcut blocked the side-by-side install'
    }

    Reset-Scenario 'EmptyTakeoverConflict'
    $emptyShortcut=Join-Path $script:fakePrograms 'Snipping Tool.lnk'
    [IO.File]::WriteAllText($emptyShortcut,'synthetic empty-target shortcut')
    $script:mockShortcuts[$emptyShortcut]=[pscustomobject]@{TargetPath='';WorkingDirectory='';Description='';IconLocation=''}
    $caught=$false;try{& $block -Executable $fixture -NoticeFolder $project -Version '0.0.1' -Apply -ReplaceWindowsSnippingTool | Out-Null}catch{$caught=$true}
    Assert ($caught -and $script:fakeMicrosoft -and $script:removals -eq 0 -and $script:mockShortcuts[$emptyShortcut].TargetPath -eq '') 'Explicit takeover did not preserve an unowned empty-target shortcut'

    foreach($custom in @('"relative\snipping-tool-ul.exe" --background',('"C:\portable'+[string][char]0+'\snipping-tool-ul.exe" --background'))){
        Reset-Scenario ('StartupMalformed'+[Guid]::NewGuid().ToString('N'))
        $script:fakeStartup=$custom
        $installed=& $block -Executable $fixture -NoticeFolder $project -Version '0.0.2' -Apply | ConvertFrom-Json
        Assert ($script:startupWrites -eq 0 -and $script:fakeStartup -eq $custom -and !$installed.replacesWindows) 'Malformed or relative startup command blocked install or changed the custom value'
    }

    Write-Output 'Installer integration regressions passed. Filesystem isolated; Appx, startup registry, and shortcut COM are mocked; no real Start registration or uninstall.'
}finally{$env:LOCALAPPDATA=$originalLocal}
