use std::fmt;

#[derive(Debug)]
pub struct AppError(pub String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError(e.to_string())
    }
}

impl From<tonic::Status> for AppError {
    fn from(s: tonic::Status) -> Self {
        AppError(s.message().to_string())
    }
}

// Tauri requiere que los errores de comandos sean String-serializable
impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.0
    }
}
