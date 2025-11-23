def main [...command_and_args: string] {
    let vswhere_path = "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe"
    if not ($vswhere_path | path exists) {
        error make { msg: "vswhere.exe not found. Cannot locate Visual Studio." }
        return
    }

    let vcvars_path = (^$vswhere_path -latest -find "VC\\Auxiliary\\Build\\vcvarsall.bat" | str trim)
    if ($vcvars_path | is-empty) {
        error make { msg: "Could not find vcvarsall.bat. Is the C++ workload installed?" }
        return
    }
    
    (^cmd /c $vcvars_path x86 & ...($command_and_args))
}
