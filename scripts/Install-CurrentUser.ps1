[CmdletBinding()]
param(
    [string]$Executable=(Join-Path $PSScriptRoot '..\snipping-tool-ul.exe'),
    [string]$Version='0.3.26',
    [string]$NoticeFolder='',
    [switch]$ReplaceWindowsSnippingTool,
    [switch]$Apply,
    [switch]$Interactive
)
$ErrorActionPreference='Stop'
if($Version -notmatch '^\d+\.\d+\.\d+$'){throw 'Invalid release version.'}
$source=(Resolve-Path -LiteralPath $Executable).ProviderPath
if(!$NoticeFolder){$NoticeFolder=Split-Path -LiteralPath $source}
if([IO.Path]::GetFileName($source) -ne 'snipping-tool-ul.exe'){throw 'Select the Snipping Tool UL executable.'}
$root=[IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\SnippingToolUL'))
$destination=Join-Path $root $Version
$target=Join-Path $destination 'snipping-tool-ul.exe'
$programs=[Environment]::GetFolderPath('Programs')
if(!$Apply){
    [ordered]@{previewOnly=$true;version=$Version;installFolder=$destination;replaceWindows=[bool]$ReplaceWindowsSnippingTool;shortcutName=$(if($ReplaceWindowsSnippingTool){'Snipping Tool'}else{'Snipping Tool UL'});currentUserOnly=$true;startupEnabled=$false} | ConvertTo-Json -Compress
    return
}
if($Interactive){
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing
    $form=New-Object Windows.Forms.Form
    $form.Text='Install Snipping Tool UL'; $form.ClientSize=New-Object Drawing.Size(510,240)
    $form.StartPosition='CenterScreen'; $form.FormBorderStyle='FixedDialog'; $form.MaximizeBox=$false; $form.MinimizeBox=$false
    $form.Font=New-Object Drawing.Font('Segoe UI',10)
    $form.BackColor=[Drawing.Color]::FromArgb(36,35,41); $form.ForeColor=[Drawing.Color]::FromArgb(237,234,243)
    $label=New-Object Windows.Forms.Label; $label.SetBounds(22,22,465,48)
    $label.Text='Install for your Windows account. No administrator access or automatic startup is needed.'
    $check=New-Object Windows.Forms.CheckBox; $check.SetBounds(22,82,465,30)
    $check.Text='Use instead of Windows Snipping Tool'; $check.Checked=[bool]$ReplaceWindowsSnippingTool
    $detail=New-Object Windows.Forms.Label; $detail.SetBounds(22,120,465,54)
    $detail.Text='When checked: removes Microsoft Snipping Tool for your account and names UL "Snipping Tool" in Start. Restore Microsoft through the Store. Win+Shift+S is not remapped.'
    $install=New-Object Windows.Forms.Button; $install.SetBounds(268,192,100,30); $install.Text='Install'; $install.DialogResult=[Windows.Forms.DialogResult]::OK
    $cancel=New-Object Windows.Forms.Button; $cancel.SetBounds(382,192,100,30); $cancel.Text='Cancel'; $cancel.DialogResult=[Windows.Forms.DialogResult]::Cancel
    $form.Controls.AddRange(@($label,$check,$detail,$install,$cancel)); $form.AcceptButton=$install; $form.CancelButton=$cancel
    try{if($form.ShowDialog() -ne [Windows.Forms.DialogResult]::OK){Write-Output 'Installation cancelled.'; return}; $ReplaceWindowsSnippingTool=[bool]$check.Checked}finally{$form.Dispose()}
}
# Carry an existing opt-in forward when moving between versioned folders.
# Never enable startup for a new install or replace a portable/custom command.
$startupKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$startupName='Snipping Tool UL'
$startupValue=Get-ItemProperty -LiteralPath $startupKey -Name $startupName -ErrorAction SilentlyContinue
$startupCommand=if($null -ne $startupValue){$startupValue.$startupName}else{$null}
$updateStartup=$false
function Get-NormalizedFilesystemPath([string]$value){
    if([string]::IsNullOrWhiteSpace($value)){return $null}
    try{
        if(![IO.Path]::IsPathRooted($value)){return $null}
        return [IO.Path]::GetFullPath($value)
    }catch{return $null}
}
function Test-OwnedInstallExecutable([string]$value){
    $normalized=Get-NormalizedFilesystemPath $value
    if([string]::IsNullOrEmpty($normalized)){return $false}
    $ownedPrefix=$root+[IO.Path]::DirectorySeparatorChar
    return $normalized.StartsWith($ownedPrefix,[StringComparison]::OrdinalIgnoreCase) -and [IO.Path]::GetFileName($normalized) -eq 'snipping-tool-ul.exe'
}
if($startupCommand -is [string] -and $startupCommand -match '^"([^"]+)" --background$'){
    $startupExe=Get-NormalizedFilesystemPath $Matches[1]
    if($startupExe -and $startupExe.StartsWith($root+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){
        $relativeStartup=$startupExe.Substring($root.Length+1)
        $updateStartup=$relativeStartup -match '^\d+\.\d+\.\d+\\snipping-tool-ul\.exe$'
    }
}
$shortcutName=if($ReplaceWindowsSnippingTool){'Snipping Tool.lnk'}else{'Snipping Tool UL.lnk'}
$shortcutPath=Join-Path $programs $shortcutName
$hash=(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
$noticeFiles=@('LICENSE','THIRD_PARTY_NOTICES.md','assets\OpenSans-OFL.txt','assets\Lucide-LICENSE.txt','docs\DEPENDENCY_NOTICES.md')
foreach($name in $noticeFiles){if(!(Test-Path -LiteralPath (Join-Path $NoticeFolder $name))){throw 'License notices are missing. Install from the full package.'}}
$shell=New-Object -ComObject WScript.Shell
try{
    $replacementMode=[bool]$ReplaceWindowsSnippingTool
    if(!$replacementMode){
        $replacementPath=Join-Path $programs 'Snipping Tool.lnk'
        if(Test-Path -LiteralPath $replacementPath){
            $replacementLink=$shell.CreateShortcut($replacementPath)
            if(Test-OwnedInstallExecutable $replacementLink.TargetPath){
                $replacementMode=$true
                $shortcutPath=$replacementPath
            }
        }
    }
    $link=$shell.CreateShortcut($shortcutPath)
    if(Test-Path -LiteralPath $shortcutPath){
        if(!(Test-OwnedInstallExecutable $link.TargetPath)){throw 'An unrelated Start shortcut has this name. It was preserved.'}
    }
    if(Test-Path -LiteralPath $target){
        if((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $hash){throw 'This release folder contains a different executable. Choose a new release version.'}
    }else{
        [void](New-Item -ItemType Directory -Path $destination -Force)
        Copy-Item -LiteralPath $source -Destination $target
    }
    if((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $hash){throw 'Installed executable verification failed.'}
    foreach($name in $noticeFiles){
        $noticeSource=Join-Path $NoticeFolder $name; $noticeTarget=Join-Path $destination $name
        $noticeHash=(Get-FileHash -LiteralPath $noticeSource -Algorithm SHA256).Hash
        if(Test-Path -LiteralPath $noticeTarget){if((Get-FileHash -LiteralPath $noticeTarget -Algorithm SHA256).Hash -ne $noticeHash){throw 'Existing license notice differs; it was preserved.'}}
        else{[void](New-Item -ItemType Directory -Path (Split-Path -LiteralPath $noticeTarget) -Force); Copy-Item -LiteralPath $noticeSource -Destination $noticeTarget}
    }
    $link.TargetPath=$target; $link.WorkingDirectory=$destination; $link.Description='Snipping Tool UL - Ultralight screenshot capture and annotation'; $link.IconLocation="$target,0"; $link.Save()
    $verified=$shell.CreateShortcut($shortcutPath)
    if($verified.TargetPath -ne $target -or !(Test-Path -LiteralPath $shortcutPath)){throw 'Start shortcut verification failed.'}
    # The replacement is already usable before the Windows app is removed.
    $removed=$replacementMode
    if($ReplaceWindowsSnippingTool){
        $packages=@(Get-AppxPackage -Name Microsoft.ScreenSketch)
        foreach($package in $packages){
            if($package.Name -ne 'Microsoft.ScreenSketch'){throw 'Unexpected app package; nothing else may be removed.'}
            Remove-AppxPackage -Package $package.PackageFullName -ErrorAction Stop
        }
        if(@(Get-AppxPackage -Name Microsoft.ScreenSketch).Count -ne 0){throw 'Microsoft Snipping Tool is still registered. UL is installed, but takeover is incomplete.'}
        $removed=$true
    }
    if($replacementMode){
        $oldShortcut=Join-Path $programs 'Snipping Tool UL.lnk'
        if(Test-Path -LiteralPath $oldShortcut){
            $oldLink=$shell.CreateShortcut($oldShortcut)
            if(Test-OwnedInstallExecutable $oldLink.TargetPath){
                Remove-Item -LiteralPath $oldShortcut
            }
        }
    }
    if($updateStartup){
        # Honor an opt-out or custom target chosen while installation ran.
        $currentStartup=Get-ItemProperty -LiteralPath $startupKey -Name $startupName -ErrorAction SilentlyContinue
        if($null -ne $currentStartup -and $currentStartup.$startupName -eq $startupCommand){
            $newStartupCommand='"'+$target+'" --background'
            Set-ItemProperty -LiteralPath $startupKey -Name $startupName -Value $newStartupCommand -Type String -ErrorAction Stop
            $verifiedStartup=Get-ItemProperty -LiteralPath $startupKey -Name $startupName -ErrorAction Stop
            if($verifiedStartup.$startupName -ne $newStartupCommand){throw 'Startup update verification failed.'}
        }
    }
    $state=[ordered]@{version=$Version;executable=$target;sha256=$hash;shortcut=$shortcutPath;replacesWindows=$removed;restoreMicrosoft='https://apps.microsoft.com/detail/9mz95kl8mr0l'}
    $state | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'integration-state.json') -Encoding UTF8
    $state | ConvertTo-Json -Compress
}finally{
    if($replacementLink){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($replacementLink)}
    if($oldLink){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($oldLink)}
    if($verified){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($verified)}
    if($link){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($link)}
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)
}
