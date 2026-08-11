use std::process::ExitCode;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use eyre::Result;

use deno_runtime::deno_core::FsModuleLoader;
use deno_runtime::deno_core::ModuleSpecifier;
use deno_runtime::deno_fs::RealFs;
use deno_runtime::deno_permissions::PermissionsContainer;
use deno_runtime::deno_permissions::Permissions;
use deno_runtime::permissions::RuntimePermissionDescriptorParser;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerServiceOptions;
use deno_runtime::worker::WorkerOptions;

use deno_resolver::npm::DenoInNpmPackageChecker;
use deno_resolver::npm::ManagedNpmResolver;
use sys_traits::impls::RealSys;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<ExitCode> {
    let js_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/main.js");
    let main_module = ModuleSpecifier::from_file_path(js_path).unwrap();
    
    let worker_service_options = WorkerServiceOptions::<DenoInNpmPackageChecker, ManagedNpmResolver<RealSys>, RealSys> {
        deno_rt_native_addon_loader: None,
        module_loader: Rc::new(FsModuleLoader),
        permissions: PermissionsContainer::new(
            Arc::new(RuntimePermissionDescriptorParser::new(RealSys)),
            Permissions::none_without_prompt(),
        ),
        blob_store: Default::default(),
        broadcast_channel: Default::default(),
        feature_checker: Default::default(),
        node_services: Default::default(),
        npm_process_state_provider: Default::default(),
        root_cert_store_provider: Default::default(),
        fetch_dns_resolver: Default::default(),
        shared_array_buffer_store: Default::default(),
        compiled_wasm_module_store: Default::default(),
        v8_code_cache: Default::default(),
        fs: Arc::new(RealFs),
        bundle_provider: None,
    };
    
    let mut worker = MainWorker::bootstrap_from_options(
        &main_module,
        worker_service_options,
        WorkerOptions {
            // ..
            extensions: vec![
                // ..
            ],
            ..Default::default()
        },
    );
    
    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;
    
    Ok(ExitCode::SUCCESS)
}