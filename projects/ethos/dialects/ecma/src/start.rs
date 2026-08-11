
//---
#[cfg(feature="ffi")]
pub mod ffi {
    use std::ffi::CStr;
    
    use crate::runtime::EcmaRuntimeError;
    
    use crate::runtime::ffi::CEcmaRuntimeState;
    use crate::runtime::ffi::JS_RUNTIME_MANAGER;
    
    //---
    #[repr(C)]
    #[derive(Debug)]
    pub struct CExecuteModuleOptions {
        pub main_module_specifier: *const std::ffi::c_char,
    }

    /// TODO
    #[repr(C)]
    #[derive(Debug)]
    pub enum CStartResult {
        Ok = 0,
        Err = 1,
        BindingErr = 2,
        EcmaRuntimeErr = 3,
        FailedCreateAsyncRuntime = 4,
        FailedFetchingWorkDirErr = 5,
        DataDirInvalidErr = 6,
        LogDirInvalidErr = 7,
        MainModuleInvalidErr = 8,
        MainModuleUninitializedErr = 9,
        FailedModuleExecErr = 10,
        FailedEventLoopErr = 11,
    }

    /// TODO: Return a CEcmaRuntimeStartResult (repr(C)) for state.
    #[export_name = "aby__start"]
    pub unsafe extern "C" fn c_start(options: CExecuteModuleOptions) -> CStartResult {
        let Some(ecma_runtime) = JS_RUNTIME_MANAGER.get() else {
            crate::runtime::ffi::set_state(CEcmaRuntimeState::Panic);
            return CStartResult::BindingErr; // </3
        };
        
        let ecma_runtime = ecma_runtime.lock().expect("Failed to get lock for EcmaRuntime!");
        ecma_runtime.send_log("Attempting to start ..");
        
        crate::runtime::ffi::set_state(CEcmaRuntimeState::Startup);
        
        let c_str = if options.main_module_specifier.is_null() {
            ecma_runtime.send_log("Main Module not specified ..");
            return CStartResult::EcmaRuntimeErr;
        } else {
            CStr::from_ptr(options.main_module_specifier)
        };
        
        let main_module_specifier = match c_str.to_str() {
            Ok(specifier) => specifier,
            Err(e) => {
                ecma_runtime.send_log(format!("Failed to convert to UTF-8: {}", e));
                return CStartResult::EcmaRuntimeErr;
            }
        };

        // TODO: Maybe we should be using a panic hook instead?
        // Ref: https://doc.rust-lang.org/std/panic/fn.set_hook.html
        match std::panic::catch_unwind(|| -> Result<u32, EcmaRuntimeError> {
            Ok(ecma_runtime.start(main_module_specifier)?)
        }) {
            Ok(exit_result) => match exit_result {
                Ok(exit_status) => {
                    ecma_runtime.send_log(format!("Runtime exited with status {:}", exit_status));
                    crate::runtime::ffi::set_state(CEcmaRuntimeState::Shutdown);
                    CStartResult::Ok // <3
                }
                Err(error) => match error {
                    EcmaRuntimeError::DenoAnyError(deno_error) => {
                        ecma_runtime.send_log(format!("Runtime exited with JavaScript error: {:}", deno_error));
                        crate::runtime::ffi::set_state(CEcmaRuntimeState::Shutdown);
                        CStartResult::EcmaRuntimeErr // </3
                    }
                    _ => {
                        ecma_runtime.send_log(format!("Runtime exited with error: {:#?}", error));
                        crate::runtime::ffi::set_state(CEcmaRuntimeState::Panic);
                        CStartResult::BindingErr // </3
                    }
                }
            }
            Err(payload) => {
                crate::logging::ffi::handle_panic(payload);
                crate::runtime::ffi::set_state(CEcmaRuntimeState::Panic);
                CStartResult::BindingErr // </3
            }
        }
    }
}
