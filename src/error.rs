use std::path::{PathBuf, StripPrefixError};
use thiserror::Error;

/// Errors that can occur during OpenAPI to MCP server code generation.
#[derive(Error, Debug)]
pub enum OpenApiToMcpError {
    /// An I/O error occurred during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse JSON data.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to strip path prefix.
    #[error("Path strip prefix error: {0}")]
    StripPrefix(#[from] StripPrefixError),

    /// The output directory already exists.
    #[error("Output directory already exists: {0}")]
    OutputDirectoryExists(PathBuf),

    /// The templates directory was not found.
    #[error("Templates directory not found")]
    TemplatesDirectoryNotFound,

    /// Failed to create the output directory.
    #[error("Failed to create output directory: {0}")]
    OutputDirectoryCreation(PathBuf),

    /// Failed to copy the templates directory.
    #[error("Failed to copy templates directory")]
    TemplatesCopy,

    /// Failed to read the OpenAPI file.
    #[error("Failed to read OpenAPI file: {0}")]
    OpenApiFileRead(PathBuf),

    /// Failed to parse the OpenAPI specification as JSON.
    #[error("Failed to parse OpenAPI spec as JSON")]
    OpenApiParse,

    /// Failed to create the source directory in the output directory.
    #[error("Failed to create src directory in output")]
    SrcDirectoryCreation,

    /// Failed to write the index.ts file.
    #[error("Failed to write index.ts")]
    IndexFileWrite,
}

/// A type alias for `Result<T, OpenApiToMcpError>`.
pub type Result<T> = std::result::Result<T, OpenApiToMcpError>; 
