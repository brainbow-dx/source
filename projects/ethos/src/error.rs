pub use derive_more as smore;

//---
#[derive(oops::Error, smore::From)]
pub enum EthosError {
    /// TODO
    #[msg("path error: {:}")]
    PathError(std::io::Error),
}
