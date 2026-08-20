#![allow(unused)]

use std::process::{Command, ExitCode, ExitStatus};
use std::fs::File;
use std::io::Write;

use anyhow::Result;
use anyhow::Error as AnyError;

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::execution_engine::JitFunction;

use ethos::exec::Exec;

type ReadGlobalConfigFn = unsafe extern "C" fn() -> i32;

static CLANG_EXE: &str = "C:/LLVM/16.0.5/bin/clang.exe";

pub struct Scribe {
    context: Context,
}

impl From<Context> for Scribe {
    fn from(context: Context) -> Scribe {
        Scribe {
            context,
        }
    }
}

pub struct ModuleBuilder<'ctx> {
    module: Module<'ctx>,
}

impl<'ctx> From<Module<'ctx>> for ModuleBuilder<'ctx> {
    fn from(module: Module<'ctx>) -> ModuleBuilder<'ctx> {
        ModuleBuilder {
            module,
        }
    }
}

fn main() -> Result<ExitCode> {
    ethos::log::init("warn,franken=debug,ethos_core=debug,ethos=debug");
    
    //--
    // Step 0: Environment Setup
    
    // Create a new LLVM context
    let context = Context::create();
    let scribe = Scribe::from(Context::create());

    // 
    // let builder = ModuleBuilder::from(context.create_module("config_mod"));

    //--
    // Stage 1: Construct source(s) into Modules ..
    
    // Create a new module within that context
    let module = context.create_module("config_mod");
    
    // Create an i32 type
    let i32_type = context.i32_type();
    let f64_type = context.f64_type();
    
    // Add a global variable of type i32 named "my_global"
    let global_var = module.add_global(i32_type, None, "global_cfg");
    
    // Initialize the global variable with the value 42
    let const_int = i32_type.const_int(rand::random::<u64>(), false);
    global_var.set_initializer(&const_int);
    
    // Define a function that returns an i32 and takes no arguments
    let fn_type = i32_type.fn_type(&[], false);
    
    // Create a basic block named "entry" in the function
    let function = module.add_function("read_global_cfg", fn_type, None);
    let body_block = context.append_basic_block(function, "body");
    
    // Create a builder to build instructions in the basic block
    let instructions = context.create_builder();
    instructions.position_at_end(body_block);
    
    // Load the value from the global variable
    let load_inst = instructions.build_load(i32_type, global_var.as_pointer_value(), "load_global_cfg")?;
    
    // Return the loaded value
    instructions.build_return(Some(&load_inst))?;
    
    //--
    // Create an execution engine for JIT (Just-in-Time) compilation
    let module_executor = module.create_jit_execution_engine(OptimizationLevel::None)
        .map_err(|error| AnyError::msg(error.to_string()))?;

    //--
    unsafe {
        // Get a reference to the compiled 'read_global_cfg' function
        let compiled_fn = module_executor.get_function::<ReadGlobalConfigFn>("read_global_cfg")?;

        // Execute the function and print the result
        let result = compiled_fn.call();
        tracing::info!("The result of 'read_global_cfg' is: {}", result);
    }
    
    //--
    // Output the LLVM IR to stderr
    #[cfg(feature = "verbose")]
    module.print_to_stderr();
    
    //--
    // Step ##: Commit Generated Artifacts
    
    // Save the resulting LLVM IR to file.
    module.print_to_file("./examples/franken/franken.ll")
        .map_err(|error| AnyError::msg(error.to_string()))?;

    tracing::debug!("Saved IR Snapshot at `{:}`", "./examples/franken/franken.ll");

    //--
    // Build the library object file ..
    let build_obj_args = [
        "./examples/franken/franken.ll",
        "-o", "./examples/franken/libfranken.obj",
        "-c", // Export library ..
    ];
    
    let build_obj_stdout = Exec::run(CLANG_EXE, build_obj_args)?;
    
    tracing::debug!("Built Shared Object at `{:}`", "./examples/franken/libfranken.obj");
    #[cfg(feature = "verbose")]
    {
        tracing::debug!("STDOUT: {:}", build_obj_stdout);
    }
    
    //--
    // Build the final executable ..
    let build_exe_args = [
        "./examples/franken/main.c",
        "./examples/franken/libfranken.obj",
        "-o", "./examples/franken/franken.exe",
    ];
    
    let build_exe_stdout = Exec::run(CLANG_EXE, build_exe_args)?;
    tracing::debug!("Built Executable at `{:}`", "./examples/franken/franken.exe");
    #[cfg(feature = "verbose")]
    {
        tracing::debug!("STDOUT: {:}", build_exe_stdout);
    }
    
    //--
    Ok(ExitCode::SUCCESS)
}
