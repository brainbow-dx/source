use deno_runtime::deno_core::*;
use deno_runtime::deno_core::error::ModuleLoaderError;

/// TODO
pub struct AbyModuleLoader;

impl ModuleLoader for AbyModuleLoader {
    /// TODO
    fn resolve(&self, specifier: &str, referrer: &str, _resolution_kind: ResolutionKind) -> Result<ModuleSpecifier, ModuleLoaderError> {
        // Resolve the module specifier to an absolute URL or path
        resolve_import(specifier, referrer)
            .map_err(|error| ModuleLoaderError::from_err(error))
    }
    
    /// TODO
    fn load(&self, module_specifier: &ModuleSpecifier, _: Option<&ModuleLoadReferrer>, _: ModuleLoadOptions) -> ModuleLoadResponse {
        let module_specifier = module_specifier.to_owned();
        let module_type = ModuleType::JavaScript;
        let module_code = ModuleSourceCode::String(ModuleCodeString::from_static("console.log('Hello from the module!');"));
        
        println!("Loading module {:}", module_specifier);
        
        ModuleLoadResponse::Async(Box::pin(async move {
            Ok(ModuleSource::new(module_type, module_code, &module_specifier, None))
        }))
    }
}
