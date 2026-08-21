#![allow(unused)]

use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::MutexGuard;
use std::error::Error;
use std::fmt::Display;
use std::ffi::CString;

use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use deno_core::error::CoreError;

use tokio::runtime::Builder as TokioRuntimeBuilder;
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;

use deno_runtime::deno_core::FsModuleLoader;
use deno_runtime::deno_core::ModuleResolutionError;
use deno_runtime::deno_core::PollEventLoopOptions;
use deno_runtime::deno_core::resolve_url_or_path;

use crate::stdio::RuntimeStdio;

#[cfg(feature = "ffi")]
use crate::logging::ffi::CLogCallback;

#[allow(unused)] // TODO
pub struct EcmaRuntimeConfig {
    db_dir: Option<String>,
    log_dir: Option<String>,
}

impl EcmaRuntimeConfig {
    pub fn new() -> Self {
        EcmaRuntimeConfig {
            db_dir: None,
            log_dir: None,
        }
    }
}

//---
/// TODO
#[allow(unused)] // TODO
pub struct EcmaRuntimeManager {
    config: EcmaRuntimeConfig,

    async_runtime: TokioRuntime,

    stdio: RuntimeStdio,

    #[cfg(feature="ffi")]
    log_callback: Option<Arc<Mutex<CLogCallback>>>,

    #[cfg(feature="ffi")]
    log_callback_async: Option<Arc<TokioMutex<CLogCallback>>>,
}

impl EcmaRuntimeManager {
    pub fn try_new() -> Result<Self, std::io::Error> {
        let config = EcmaRuntimeConfig::new();

        let js_stdio = RuntimeStdio::try_new(None, None)?;

        // We don't need a TokioRuntime if anything else fails (so we create it last).
        let async_runtime = TokioRuntimeBuilder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()?;

        Ok(EcmaRuntimeManager {
            config,
            async_runtime,
            stdio: js_stdio,
            #[cfg(feature="ffi")]
            log_callback: None,
            #[cfg(feature="ffi")]
            log_callback_async: None,
        })
    }

    #[cfg(feature="ffi")]
    pub fn with_log_callback(mut self, log_callback: CLogCallback) {
        self.set_log_callback(log_callback)
    }

    #[cfg(feature="ffi")]
    pub fn set_log_callback(&mut self, log_callback: CLogCallback) {
        self.log_callback = Some(Arc::new(Mutex::new(log_callback)));
        self.log_callback_async = Some(Arc::new(TokioMutex::new(log_callback)));
    }
}

#[allow(unused)]
impl EcmaRuntimeManager {
    pub fn capture_trace(&self) -> Result<JoinHandle<u8>, EcmaRuntimeError> {
        let log_callback = self.log_callback_async.as_ref().ok_or(EcmaRuntimeError::LogCallbackMissing)?;

        // TODO: We shouldn't be cloning here. Find a way to share the data more safely.
        let log_callback = log_callback.clone();

        self.try_send_log("TODO: Capture tracing spans from Rust ..")?;

        self.async_runtime.block_on(async move {
            let log_thread = tokio::spawn(async move {
                loop {
                    match CString::new(format!("TODO: CAPTURE TRACE #003 ({:?})", log_callback)) {
                        Ok(c_string) => unsafe {
                            let log_callback = log_callback.lock().await;
                            log_callback(c_string.as_ptr());
                        }
                        Err(error) => {
                            tracing::error!("Log capture failed: {:}", error);
                        }
                    }

                    // TODO: Remove this!
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }

                0
            });

            // TODO: Remove this!
            tokio::time::sleep(std::time::Duration::from_nanos(100)).await;

            Ok(log_thread)
        })
    }

    pub(crate) fn send_log<T: ToString>(&self, message: T) {
        // TODO: Re-enable this!
        // self.try_send_log(message).expect("Failed to send log message!");
    }

    pub(crate) fn try_send_log<T: ToString>(&self, message: T) -> Result<(), EcmaRuntimeError> {
        match CString::new(message.to_string()) {
            Ok(c_message) => match self.log_callback.as_ref() {
                Some(log_callback_mtx) => match log_callback_mtx.lock() {
                    Ok(log_callback) => unsafe {
                        log_callback(c_message.as_ptr());
                        Ok(())
                    }
                    Err(error) => Err(EcmaRuntimeError::from(error)),
                }

                None => Err(EcmaRuntimeError::LogCallbackMissing),
            }

            // Couldn't get CString, probably because (TODO).
            Err(error) => Err(EcmaRuntimeError::from(error)),
        }
    }

    pub fn start(&self, main_specifier: &str) -> Result<u32, EcmaRuntimeError> {
        let stdio = self.stdio.try_clone_into()?;
        let current_dir = std::env::current_dir()?;

        // TODO: Move this to `AbyRuntime::resolve_main_module(..)`.
        let main_module = resolve_url_or_path(main_specifier, &current_dir)
            .map_err(|error| DenoAnyError::from(error))?;

        // Run a "lite" Deno runtime, with only a core.
        //  - No worker and minimal extensions.
        //  - Useful for some testing and debug scenarios.
        self.async_runtime.block_on(async move {
            let mut ecma_runtime = JsRuntime::new(RuntimeOptions {
                module_loader: Some(Rc::new(FsModuleLoader)),
                extensions: vec![],
                ..Default::default()
            });

            if let Err(error) =
                ecma_runtime.execute_script("<prelude>", include_str!("./00_prelude.js"))
            {
                tracing::error!("Failed to run prelude script: {:}", error);
            }

            if let Err(error) =
                ecma_runtime.execute_script("<debug>", include_str!("./99_debug.js"))
            {
                tracing::error!("Failed to run debug setup script: {:}", error);
            }

            if let Err(error) = ecma_runtime
                .run_event_loop(PollEventLoopOptions::default())
                .await
            {
                tracing::error!("Failed to run main worker event loop: {:}", error);
            }
        });

        // Run the "not-lite", full Deno runtime (this crate's own shared `bootstrap_main_worker`
        // handles the Tokio-reactor-context requirement; see its own doc comment). Prefer this
        // when you want all of Deno's features.
        self.async_runtime.block_on(async move {
            let mut worker = crate::worker::bootstrap_main_worker(
                &main_module,
                stdio,
                Some(std::path::PathBuf::from("./Data/Store")),
                vec![],
                false,
                None,
            );

            // TODO: Revist the Clone for `main_module`.
            let error = worker.execute_main_module(&main_module.clone()).await
                .map_err(|error| DenoAnyError::from(error))?;

            // TODO
            worker.js_runtime.run_event_loop(PollEventLoopOptions::default()).await
                .map_err(|error| DenoAnyError::from(error))?;

            Ok(0)
        })
    }
}

use deno_runtime::deno_core::anyhow::Error as DenoAnyError;

/// TODO
#[derive(Debug)]
pub enum EcmaRuntimeError {
    /// A user-supplied module-name was invalid.
    InvalidModuleSpecifier(&'static str),

    /// The runtime detected a current or future invalid atomic state.
    InvalidState(u32),

    ResourceError(&'static str, std::io::Error),

    NulError(std::ffi::NulError),

    ModuleError(ModuleResolutionError),

    DenoAnyError(DenoAnyError),

    AnyError(eyre::Error),

    CoreError(CoreError),

    /// An unknown error occurred.
    Unknown(&'static str),

    LogCallbackMissing,

    LogCallbackPoisoned,
}

impl Error for EcmaRuntimeError {}

impl Display for EcmaRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TODO")
    }
}

impl From<std::io::Error> for EcmaRuntimeError {
    fn from(error: std::io::Error) -> EcmaRuntimeError {
        EcmaRuntimeError::ResourceError("io", error)
    }
}

impl From<std::ffi::NulError> for EcmaRuntimeError {
    fn from(error: std::ffi::NulError) -> EcmaRuntimeError {
        EcmaRuntimeError::NulError(error)
    }
}

impl From<ModuleResolutionError> for EcmaRuntimeError {
    fn from(error: ModuleResolutionError) -> EcmaRuntimeError {
        EcmaRuntimeError::ModuleError(error)
    }
}

impl From<eyre::Error> for EcmaRuntimeError {
    fn from(error: eyre::Error) -> EcmaRuntimeError {
        EcmaRuntimeError::AnyError(error)
    }
}

impl From<DenoAnyError> for EcmaRuntimeError {
    fn from(error: DenoAnyError) -> EcmaRuntimeError {
        EcmaRuntimeError::DenoAnyError(error)
    }
}

impl From<PoisonError<MutexGuard<'_, CLogCallback>>> for EcmaRuntimeError {
    /// TODO: Use the actual error!
    fn from(_: PoisonError<MutexGuard<'_, CLogCallback>>) -> EcmaRuntimeError {
        EcmaRuntimeError::LogCallbackPoisoned
    }
}

//---
#[cfg(feature="ffi")]
pub mod ffi {
    use std::ffi::CString;
    use std::path::Path;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::OnceLock;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::net::SocketAddrV4;

    use deno_runtime::deno_inspector_server::InspectPublishUid;
    use tokio::runtime::Builder as TokioRuntimeBuilder;

    use deno_runtime::deno_web::InMemoryBroadcastChannel;
    use deno_runtime::deno_core::resolve_url_or_path;
    use deno_runtime::deno_io::Stdio;
    use deno_runtime::deno_io::StdioPipe;
    use deno_runtime::deno_inspector_server::InspectorServer;

    use crate::logging::ffi::CEcmaRuntimeLogLevel;
    use crate::logging::ffi::CLogCallback;
    use crate::start::ffi::CExecuteModuleOptions;
    use crate::start::ffi::CStartResult;

    use super::EcmaRuntimeConfig;
    use super::EcmaRuntimeManager;
    use super::EcmaRuntimeError;

    // The `ethos_sdk` extension's JS bootstrap files (`00_prelude.js`/`99_debug.js`) live in this
    // crate's own `src/`. The `dir "src"` below is resolved relative to this crate at compile
    // time. This is the default host-embedding SDK the FFI surface below boots; a dialect has no
    // opinion on it, this crate does.
    deno_runtime::deno_core::extension!(
        ethos_sdk,
        ops = [
            op_send_host_log,
        ],
        esm = [
            dir "src",
        ],
        lazy_loaded_esm = [
            dir "src",
            "00_prelude.js",
            "99_debug.js",
        ],
        options = {
            some_bool_shit: Option<bool>,
            lol_strings: Option<Vec<String>>,
        },
        state = |state, options| {
            // state.put(SomeLogState {
            //     //..
            // });
        },
    );

    #[derive(Clone)]
    pub struct SomeLogState {
        //..
    }

    #[deno_runtime::deno_core::op2(fast)]
    pub fn op_send_host_log(#[string] message: &str) {
        tracing::trace!("[Host]: {:}", message);
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct CEcmaRuntime {
        config: CEcmaRuntimeConfig,
    }

    impl CEcmaRuntime {
        #[allow(unused)] // TODO
        unsafe fn create_stdio<P: AsRef<Path>>(&self, dir: P) -> Result<Stdio, std::io::Error> {
            Ok(Stdio {
                stdin: StdioPipe::inherit(),
                stdout: StdioPipe::inherit(),
                stderr: StdioPipe::inherit(),
            })
        }
    }

    #[export_name = "ecma__construct_runtime"]
    pub unsafe extern "C" fn c_construct_runtime(config: CEcmaRuntimeConfig) -> *mut CEcmaRuntime {
        let ecma_runtime = CEcmaRuntime {
            config,
        };

        Box::into_raw(Box::new(ecma_runtime))
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct CExecModuleResult {
        code: CStartResult,
        message: Option<*const core::ffi::c_char>,
    }

    impl CEcmaRuntime {
        unsafe fn unwrap_ptr(ptr: &mut CEcmaRuntime) -> &mut CEcmaRuntime {
            // TODO: Ensure Pointer is safe to use.
            &mut *ptr
        }

        unsafe fn send_host_log<M: Into<String>>(&self, message: M) -> Result<bool, std::io::Error> {
            match CString::new(message.into()) {
                Ok(message) => {
                    (self.config.log_callback_fn)(message.as_ptr());
                    Ok(true) // <3
                }
                Err(error) => Err(std::io::Error::other(format!("TODO: {:}", error)))
            }
        }
    }

    #[allow(unused, unreachable_code)]
    #[export_name = "ecma__send_broadcast"]
    pub unsafe extern "C" fn c_send_broadcast(cself: *mut CEcmaRuntime, message: core::ffi::c_uint) {
        let ecma_runtime = CEcmaRuntime::unwrap_ptr(&mut *cself);

        // TODO: We need to keep this around. Options:
        //   - Store in a Boxed closure?
        //   - Pin Broadcast channel?
        let broadcast_channel = Box::new(InMemoryBroadcastChannel::default());

        let resource = todo!("Where do we get the resource?");
        let name = format!("Some broadcast channel ..");
        let data = Vec::<u8>::new(); // TODO: Construct message.

        // Note: Deno recently changed the privacy profile for the broadcast api.
        // TODO: Send the message .. somewhere?
        // broadcast_channel.send(resource, name, data);
    }

    #[export_name = "ecma__exec_module"]
    pub unsafe extern "C" fn c_exec_module(cself: *mut CEcmaRuntime, options: CExecuteModuleOptions) -> CStartResult {
        let cself = CEcmaRuntime::unwrap_ptr(&mut *cself);

        let Ok(async_runtime) = TokioRuntimeBuilder::new_current_thread().enable_time().enable_io().build() else {
            return CStartResult::FailedCreateAsyncRuntime;
        };

        let Ok(root_dir) = std::env::current_dir() else {
            return CStartResult::FailedFetchingWorkDirErr;
        };

        let Ok(data_dir) = crate::cwrap::try_unwrap_cstr(cself.config.db_dir) else {
            return CStartResult::DataDirInvalidErr;
        };

        let Ok(log_dir) = crate::cwrap::try_unwrap_cstr(cself.config.log_dir) else {
            return CStartResult::LogDirInvalidErr;
        };

        let Ok(main_module_specifier) = crate::cwrap::try_unwrap_cstr(options.main_module_specifier) else {
            return CStartResult::MainModuleInvalidErr;
        };

        // TODO: Move this to `AbyRuntime::resolve_main_module(..)`.
        let Ok(main_module) = resolve_url_or_path(main_module_specifier, &root_dir) else {
            return CStartResult::MainModuleInvalidErr;
        };

        if let Err(error) = cself.send_host_log(format!("Resolved module to {:}", main_module)) {
            tracing::error!("Failed to report main module specifier: {:}", error);
        }

        let Ok(stdio) = cself.create_stdio(&log_dir) else {
            return CStartResult::MainModuleUninitializedErr;
        };

        let maybe_inspector_server = {
            let inspector_name = "Aby Runtime 001";
            let inspector_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), cself.config.inspector_port));

            tracing::debug!("Using configured inspector address: {:}", inspector_addr);

            if let Err(error) = cself.send_host_log(format!("Inspector address set to {:}", inspector_addr)) {
                tracing::error!("Failed to report inspector address: {:}", error);
            }

            Some(Arc::new(InspectorServer::new(inspector_addr, inspector_name, InspectPublishUid::default())))
        };

        let should_wait_for_inspector_session = cself.config.inspector_wait && maybe_inspector_server.is_some();

        tracing::debug!("Executing Module: {:}", main_module);

        // `bootstrap_main_worker` handles the Tokio-reactor-context requirement internally. See
        // this crate's `worker` module doc comment. So this whole block, not just the
        // execute/event-loop calls, has to run inside `block_on`.
        async_runtime.block_on(async move {
            let mut worker = crate::worker::bootstrap_main_worker(
                &main_module,
                stdio,
                Some(std::path::PathBuf::from(data_dir)),
                vec![ethos_sdk::init(Some(true), None)],
                should_wait_for_inspector_session,
                None,
            );

            if let Err(error) = worker.execute_main_module(&main_module).await {
                tracing::warn!("Failed module execution: {:}", error);
                cself.send_host_log(format!("Failed main module execution: {:}", error)).unwrap_or(false);
                return CStartResult::FailedModuleExecErr;
            }

            if let Err(error) = worker.run_event_loop(false).await {
                tracing::warn!("Failed to run event loop: {:}", error);
                cself.send_host_log(format!("Failed main module execution: {:}", error)).unwrap_or(false);
                return CStartResult::FailedEventLoopErr;
            }

            CStartResult::Ok
        })
    }

    #[export_name = "ecma__free_runtime"]
    pub unsafe extern "C" fn c_free_runtime(obj_ptr: *mut CEcmaRuntime) {
        let _ = Box::from_raw(obj_ptr);
    }

    //---
    /// Uses `OnceLock` for lazy init and lock, `Arc` for sharing,
    /// and `Mutex` for inner mutability.
    pub(crate) static JS_RUNTIME_MANAGER: OnceLock<Arc<Mutex<EcmaRuntimeManager>>> = OnceLock::new();

    pub(crate) static JS_RUNTIME_STATE: AtomicU32 = AtomicU32::new(CEcmaRuntimeState::None as u32);

    #[derive(Debug)]
    #[repr(C)]
    pub struct CEcmaRuntimeConfig {
        pub inspector_wait: bool,
        pub inspector_port: u16,
        pub db_dir: *const core::ffi::c_char,
        pub log_dir: *const core::ffi::c_char,
        pub log_level: CEcmaRuntimeLogLevel,
        pub log_callback_fn: CLogCallback,
    }

    impl TryInto<EcmaRuntimeConfig> for CEcmaRuntimeConfig {
        type Error = EcmaRuntimeError;
        fn try_into(self) -> Result<EcmaRuntimeConfig, Self::Error> {
            Ok(EcmaRuntimeConfig::new())
        }
    }

    /// Representing the state of the current `EcmaRuntime`` instance
    /// running in the bound process.
    ///
    /// Tagged repr(C) for ffi to Unity, Unreal, etc.
    #[repr(C)]
    pub enum CEcmaRuntimeState {
        /// No state has been set, yet. Treat this as "uninitialized".
        None = 0,

        /// Runtime has been bootstrapped but not yet "warm" (running).
        Cold = 1,

        /// The runtime is executing startup operations. Try again next frame.
        Startup = 2,

        /// The runtime is working and has had no problems (yet).
        /// Check later for failures, but all good so far!
        Warm = 3,

        /// The runtime failed in a predictable way. The host is free to attempt
        /// to recover. Otherwise, shut down gracefully.
        Failed = 4,

        /// The runtime encountered an unrecoverable error. The runtime should
        /// shutdown completely before trying again or bad things can happen.
        Panic = 5,

        /// The runtime has quit for some reason.
        Shutdown = 6,
    }

    impl TryFrom<u32> for CEcmaRuntimeState {
            type Error = EcmaRuntimeError;

            fn try_from(value: u32) -> Result<CEcmaRuntimeState, Self::Error> {
            match value {
                0 => Ok(CEcmaRuntimeState::Cold),
                1 => Ok(CEcmaRuntimeState::Startup),
                2 => Ok(CEcmaRuntimeState::Warm),
                3 => Ok(CEcmaRuntimeState::Panic),
                4 => Ok(CEcmaRuntimeState::Shutdown),
                _ => Err(EcmaRuntimeError::InvalidState(value)),
            }
        }
    }

    pub(crate) fn set_state(state: CEcmaRuntimeState) {
        JS_RUNTIME_STATE.store(state as u32, Ordering::Relaxed);
    }

    #[inline(always)]
    #[export_name = "ecma__get_state"]
    pub extern "C" fn c_get_state() -> CEcmaRuntimeState {
        match CEcmaRuntimeState::try_from(JS_RUNTIME_STATE.load(Ordering::Relaxed)) {
            Ok(state) => state,
            Err(error) => {
                tracing::error!("Couldn't get state: {:?}", error);
                CEcmaRuntimeState::None
            }
        }
    }
}
