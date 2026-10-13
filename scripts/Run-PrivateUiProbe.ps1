[CmdletBinding()]
param([string]$Executable=(Join-Path $PSScriptRoot '..\target\debug\examples\private_ui_probe.exe'),[string]$Report=(Join-Path $PSScriptRoot '..\target\private-ui-probe.json'))
$ErrorActionPreference='Stop'
$exePath=(Resolve-Path -LiteralPath $Executable).ProviderPath
$reportPath=[IO.Path]::GetFullPath($Report)
if(Test-Path -LiteralPath $reportPath){throw 'Choose a new report path; existing reports are preserved.'}
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class ULPrivateDesktop {
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] public struct Startup {
 public uint cb; public string reserved; public string desktop; public string title;
 public uint x,y,width,height,xChars,yChars,attribute,flags; public ushort show,reserved2;
 public IntPtr extra,input,output,error;
 }
 [StructLayout(LayoutKind.Sequential)] public struct ProcessInfo {public IntPtr process,thread;public uint processId,threadId;}
 [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern IntPtr CreateDesktopW(string name,IntPtr device,IntPtr mode,uint flags,uint access,IntPtr security);
 [DllImport("user32.dll",SetLastError=true)] public static extern bool CloseDesktop(IntPtr desktop);
 [DllImport("user32.dll",SetLastError=true)] public static extern IntPtr OpenInputDesktop(uint flags,bool inherit,uint access);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern bool GetUserObjectInformationW(IntPtr obj,int index,StringBuilder value,uint bytes,out uint needed);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern bool CreateProcessW(string app,StringBuilder command,IntPtr processSecurity,IntPtr threadSecurity,bool inherit,uint flags,IntPtr environment,string cwd,ref Startup startup,out ProcessInfo info);
 [DllImport("kernel32.dll")] public static extern uint WaitForSingleObject(IntPtr handle,uint timeout);
 [DllImport("kernel32.dll")] public static extern bool GetExitCodeProcess(IntPtr handle,out uint code);
 [DllImport("kernel32.dll")] public static extern bool TerminateProcess(IntPtr handle,uint code);
 [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
 public static string InputName(){IntPtr desktop=OpenInputDesktop(0,false,1);if(desktop==IntPtr.Zero)throw new Exception("Cannot inspect input desktop.");try{StringBuilder name=new StringBuilder(256);uint length;if(!GetUserObjectInformationW(desktop,2,name,512,out length))throw new Exception("Cannot inspect desktop name.");return name.ToString();}finally{CloseDesktop(desktop);}}
}
'@
$inputBefore=[ULPrivateDesktop]::InputName()
$name='SnippingToolUL.Test.'+[Guid]::NewGuid().ToString('N')
$desktop=[ULPrivateDesktop]::CreateDesktopW($name,[IntPtr]::Zero,[IntPtr]::Zero,0,0x00ff,[IntPtr]::Zero)
if($desktop -eq [IntPtr]::Zero){throw "Private desktop creation failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"}
$info=New-Object ULPrivateDesktop+ProcessInfo
try {
 $startup=New-Object ULPrivateDesktop+Startup
 $startup.cb=[Runtime.InteropServices.Marshal]::SizeOf($startup)
 $startup.desktop='WinSta0\'+$name
 $startup.flags=0x81 # hidden initial window, no feedback cursor
 $startup.show=0
 $command=New-Object Text.StringBuilder
 [void]$command.Append('"'+$exePath+'" "'+$reportPath+'"')
 if(![ULPrivateDesktop]::CreateProcessW($exePath,$command,[IntPtr]::Zero,[IntPtr]::Zero,$false,0x08000000,[IntPtr]::Zero,(Split-Path -LiteralPath $exePath),[ref]$startup,[ref]$info)){throw "Private child launch failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"}
 if([ULPrivateDesktop]::WaitForSingleObject($info.process,30000) -ne 0){[void][ULPrivateDesktop]::TerminateProcess($info.process,3);[void][ULPrivateDesktop]::WaitForSingleObject($info.process,5000);throw 'Owned private probe timed out and was stopped.'}
 $exitCode=0;[void][ULPrivateDesktop]::GetExitCodeProcess($info.process,[ref]$exitCode)
 if($exitCode -ne 0){throw "Private probe failed with exit $exitCode."}
 if(!(Test-Path -LiteralPath $reportPath)){throw 'Probe did not produce a report.'}
 if([ULPrivateDesktop]::InputName() -ne $inputBefore){throw 'Input desktop changed during the probe; this harness never switches it.'}
 $result=Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
 if($result.desktop -ne $name){throw 'Probe ran on an unexpected desktop.'}
 Write-Output 'Private UI probe completed; input desktop unchanged. No capture, clipboard, hotkey, tray or notification operations.'
} finally {
 if($info.thread -ne [IntPtr]::Zero){[void][ULPrivateDesktop]::CloseHandle($info.thread)}
 if($info.process -ne [IntPtr]::Zero){[void][ULPrivateDesktop]::CloseHandle($info.process)}
 [void][ULPrivateDesktop]::CloseDesktop($desktop)
}
