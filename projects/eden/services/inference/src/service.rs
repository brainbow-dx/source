use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug)]
#[derive(Serialize, Deserialize)]
pub struct InferenceService<B> {
    //..
    _backend: B,
}

impl<B> InferenceService<B> {
    pub fn new(backend: B) -> Self {
        InferenceService {
            _backend: backend,
        }
    }
}

impl<B> InferenceService<B> {
    pub fn send_chat_message(_message: &str) -> Result<(), ()> {
        Ok(())
    }
}

//---
#[allow(unused)]
pub mod c_module {
    use super::*;

    #[repr(C)]
    #[derive(Serialize, Deserialize)]
    pub struct InferenceServiceCBridge {
        inner: InferenceService<()>,
    }
}

//---
// #[allow(unused)]
// pub mod ecma_module {
//     use super::*;

//     #[deno_bindgen::deno_bindgen]
//     pub struct InferenceServiceEcmaBridge {
//         inner: super::c_module::InferenceServiceCBridge,
//     }

//     // #[deno_bindgen]
//     // impl InferenceServiceEcmaBridge {
//     //     #[constructor]
//     //     fn new() -> Self {
//     //         InferenceServiceEcmaBridge {
//     //             // inner: InferenceService::new(()),
//     //         }
//     //     }

//     //     fn get_some_string_thing(&mut self, num: i32, name: &str, py_kwargs: Option<&Bound<'_, PyDict>>) -> String {
//     //         println!("Getting some string thing <3");
//     //         format!("Something???")
//     //     }

//     //     fn make_change(&mut self, num: i32) -> PyResult<String> {
//     //         Ok(format!("changed!"))
//     //     }
//     // }
// }

//---
#[allow(unused)]
#[pyo3::pymodule(name = "eden_inference_service")]
mod python_module {
    use super::*;

    use pyo3::prelude::*;
    use pyo3::types::*;

    #[pyo3::pyfunction(name = "init_runtime")]
    fn py_init_runtime(a: usize, b: usize) -> PyResult<String> {
        println!("Hello from pyfunction: {a} + {b}");
        Ok((a + b).to_string())
    }

    #[pyo3::pyclass(name = "InferenceService")]
    struct InferenceServicePythonBridge {
        inner: InferenceService<()>,
    }

    #[pyo3::pymethods]
    impl InferenceServicePythonBridge {
        #[new]
        fn new() -> Self {
            InferenceServicePythonBridge {
                inner: InferenceService::new(()),
            }
        }

        fn get_some_string_thing(&mut self, num: i32, name: &str, py_kwargs: Option<&Bound<'_, PyDict>>) -> String {
            println!("Getting some string thing <3");
            format!("Something???")
        }

        fn make_change(&mut self, num: i32) -> PyResult<String> {
            Ok(format!("changed!"))
        }
    }
}
