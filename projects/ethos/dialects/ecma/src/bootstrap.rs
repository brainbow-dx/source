#[cfg(feature="ffi")]
pub mod ffi {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::ffi::CString;
    
    use crate::runtime::ffi::CEcmaRuntimeConfig;
    use crate::runtime::ffi::CEcmaRuntimeState;
    use crate::runtime::ffi::JS_RUNTIME_MANAGER;
    use crate::runtime::ffi::JS_RUNTIME_STATE;
    use crate::runtime::EcmaRuntimeManager;

    //---
    /// TODO
    #[repr(C)]
    #[derive(Debug)]
    pub struct CBootstrapOptions {
        pub thread_prefix: *const std::ffi::c_char,
        pub ecma_runtime_config: CEcmaRuntimeConfig,
    }

    #[repr(C)]
    pub enum CBootstrapResult {
        Ok = 0,
        UnknownError = 1,
        EcmaRuntimeMissing = 2,
        EcmaRuntimeFailed = 3,
        LogCaptureFailed = 4,
    }

    //---
    /// Initialize a global static `EcmaRuntime`` instance.
    /// 
    /// Use this when you want to create a single, managed instance of Deno's
    ///   `MainWorker` for use in another managed environment.
    #[allow(unused)]
    #[export_name = "aby__bootstrap"]
    pub extern "C" fn c_bootstrap(options: CBootstrapOptions) -> CBootstrapResult {
        let mut ecma_runtime = match EcmaRuntimeManager::try_new() {
            Ok(ecma_runtime) => ecma_runtime,
            Err(error) => return CBootstrapResult::EcmaRuntimeFailed,
        };
        
        let log_callback = options.ecma_runtime_config.log_callback_fn;
        ecma_runtime.set_log_callback(log_callback);

        // Log panics to the supplied log_callback.
        std::panic::set_hook(Box::new(move |panic_info| {
            match CString::new(crate::logging::ffi::unwrap_panic_message(panic_info)) {
                Ok(c_message) => {
                    log_callback(c_message.as_ptr());
                }
                Err(error) => {
                    eprintln!("Failed to unpack panic message: {:}", error);
                }
            }
        }));
        
        if let Err(error) = ecma_runtime.capture_trace() {
            let c_message = CString::new(format!("Error: {:}", error)).expect("TODO");
            log_callback(c_message.as_ptr());
        }
        
        JS_RUNTIME_MANAGER.get_or_init(|| Arc::new(Mutex::new(ecma_runtime)));
        
        JS_RUNTIME_STATE.store(CEcmaRuntimeState::Cold as u32, Ordering::Relaxed);
        
        CBootstrapResult::Ok
    }
}