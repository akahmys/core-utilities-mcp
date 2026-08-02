use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Command execution error: {0}")]
    Command(String),

    #[error("File operation failed: {0}")]
    File(String),

    #[error("Guardrail violation: {0}")]
    Guardrail(String),

    #[error("Parsing error: {0}")]
    Parsing(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Process execution failed: {0}")]
    Process(String),

    #[error("Generic error: {0}")]
    General(String),
}

impl CoreError {
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            CoreError::Command(_) => "Command",
            CoreError::File(_) => "File",
            CoreError::Guardrail(_) => "Guardrail",
            CoreError::Parsing(_) => "Parsing",
            CoreError::System(_) => "System",
            CoreError::Process(_) => "Process",
            CoreError::General(_) => "General",
        }
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
