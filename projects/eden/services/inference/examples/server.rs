use std::ffi::CString;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;

use pyo3::PyResult;
use pyo3::Python;
use pyo3::types::PyAnyMethods;
use pyo3::types::PyListMethods;

#[allow(deprecated)]
use pyo3::ffi::Py_SetPythonHome;
use widestring::WideCString;

//---
pub fn main() -> Result<ExitCode> {
    let venv_path = PathBuf::from("C:/Brainbow/projects/eden/.venv");
    let lib_path = venv_path.join("Lib").to_string_lossy().into_owned();

    unsafe {
        let venv_path = WideCString::from_str(venv_path.to_string_lossy()).expect("wide cstring");

        #[allow(deprecated)]
        Py_SetPythonHome(venv_path.as_ptr());
    }

    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| -> PyResult<()> {
        let sys = py.import("sys")?;

        let path_obj = sys.getattr("path")?;
        let path = path_obj.downcast::<pyo3::types::PyList>()?;

        path.insert(0, lib_path)?;

        let python_source_code = CString::new(include_str!("./chat.py"))?;

        // Pass the PyString to the run method.
        py.run(python_source_code.as_c_str(), None, None)?;

        Ok(())
    })?;

    return Ok(ExitCode::SUCCESS);
}
