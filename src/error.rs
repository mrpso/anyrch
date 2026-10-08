#[cfg(feature = "unicorn")]
use unicorn_engine::unicorn_const::uc_error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Unicorn error
    #[cfg(feature = "unicorn")]
    #[error("unicorn error: {0:?}")]
    Unicorn(uc_error),
}

#[cfg(feature = "unicorn")]
impl From<uc_error> for Error {
    fn from(value: uc_error) -> Self {
        Self::Unicorn(value)
    }
}
