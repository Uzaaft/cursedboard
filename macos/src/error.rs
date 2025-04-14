#[derive(Debug)]
pub enum AppError{
    IOError(std::io::Error),
    ConfigError(String)
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            AppError::IOError(error) => write!(f, "IOError: {error}"),
            AppError::ConfigError(s) => write!(f, "Config error: {s}")
        }
    }
}

impl From<std::io::Error> for AppError{
    fn from(value: std::io::Error) -> Self {
        AppError::IOError(value)
    }
}
