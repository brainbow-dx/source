#[cfg(feature = "ffi")]
pub mod ffi {
    /// TODO
    #[repr(C)]
    pub enum CEcmaRuntimeEventKind {
        Hup = 0,
    }
}
