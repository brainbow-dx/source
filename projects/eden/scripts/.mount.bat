@echo off

rem The path to vswhere.exe, used to find Visual Studio components.
set "vswhere_path=C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe"

rem Check if vswhere.exe exists. If not, it's a critical error.
if not exist "%vswhere_path%" (
echo Error: vswhere.exe not found at "%vswhere_path%".
echo Please ensure Visual Studio is installed.
exit /b 1
)

rem Find the path to vcvarsall.bat and store it.
rem The "tokens=" captures the entire output line, which is the full path.
for /f "usebackq tokens=" %%i in ("%vswhere_path%" -latest -find "VC\\Auxiliary\\Build\\vcvarsall.bat") do set "vcvars_path=%%i"

rem Check if vcvarsall.bat was found.
if "%vcvars_path%"=="" (
echo Error: Could not find vcvarsall.bat.
echo Make sure the "Desktop development with C++" workload is installed in Visual Studio.
exit /b 1
)

rem Call vcvarsall.bat to set up the environment for x86 builds.
call "%vcvars_path%" x86

rem Execute the command provided as arguments to this batch script.
rem The "%" is a special variable that represents all command-line arguments.
%