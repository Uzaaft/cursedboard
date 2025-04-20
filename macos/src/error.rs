#[derive(Debug)]
pub enum AppError{
    IOError(std::io::Error),
    ConfigError(String),
    NumError(std::num::TryFromIntError)
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            AppError::IOError(error) => write!(f, "IOError: {error}"),
            AppError::ConfigError(s) => write!(f, "Config error: {s}"),
            AppError::NumError(s) => write!(f, "Numeric error: {s}"),
        }
    }
}

impl From<std::io::Error> for AppError{
    fn from(value: std::io::Error) -> Self {
        AppError::IOError(value)
    }
}

impl From<std::num::TryFromIntError> for AppError{
    fn from(value: std::num::TryFromIntError) -> Self{
        AppError::NumError(value)
    }
}
