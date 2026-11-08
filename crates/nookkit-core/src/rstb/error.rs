#[derive(Debug, thiserror::Error)]
pub enum RstbError {
    #[error("{action} {path} failed: {source}")]
    Io {
        action: &'static str,
        path: String,
        source: std::io::Error,
    },

    #[error("creating {0} failed")]
    CreateDir(String),

    #[error("no game heap could lend memory to size files")]
    NoLoan,

    #[error("the game's heaps could lend {lent} MiB of the {need} MiB needed")]
    LoanTooSmall { lent: usize, need: usize },

    #[error(transparent)]
    Rstb(#[from] nookkit_rstb::Error),
}

pub fn io(action: &'static str, path: &str) -> impl FnOnce(std::io::Error) -> RstbError {
    move |source| RstbError::Io {
        action,
        path: path.to_owned(),
        source,
    }
}
