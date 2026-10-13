[CmdletBinding(SupportsShouldProcess)]
param(
    [string]$Executable = (Join-Path $PSScriptRoot '..\snipping-tool-ul.exe'),
    [switch]$Apply
)
$ErrorActionPreference = 'Stop'
$resolvedExe = (Resolve-Path -LiteralPath $Executable).ProviderPath
if ([IO.Path]::GetExtension($resolvedExe) -ne '.exe') { throw 'Select the Snipping Tool UL executable.' }
$programs = [Environment]::GetFolderPath('Programs')
$shortcutPath = Join-Path $programs 'Snipping Tool UL.lnk'
$shell = New-Object -ComObject WScript.Shell
try {
    $shortcut = $shell.CreateShortcut($shortcutPath)
    if ((Test-Path -LiteralPath $shortcutPath) -and $shortcut.TargetPath -ne $resolvedExe) {
        throw 'A shortcut with this name points to another executable. It will not be overwritten.'
    }
    if (!$Apply) {
        Write-Output "Preview only: create Start shortcut '$shortcutPath' for '$resolvedExe'."
        Write-Output 'Use -Apply after choosing a permanent portable folder. Start search ranking depends on Windows.'
        return
    }
    if ($PSCmdlet.ShouldProcess($shortcutPath, 'Register Snipping Tool UL in Start')) {
        $shortcut.TargetPath = $resolvedExe
        $shortcut.WorkingDirectory = Split-Path -LiteralPath $resolvedExe
        $shortcut.Arguments = ''
        $shortcut.Description = 'Snipping Tool UL - Ultralight Windows screenshot capture and annotation'
        $shortcut.IconLocation = "$resolvedExe,0"
        $shortcut.Save()
        if (!(Test-Path -LiteralPath $shortcutPath)) { throw 'Shortcut registration could not be verified.' }
        Write-Output 'Snipping Tool UL Start shortcut registered.'
    }
} finally {
    if ($null -ne $shortcut) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shortcut) }
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)
}
