mod cli;
mod error;
mod generator;

use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::cli::Args;
use crate::error::{OpenApiToMcpError, Result};
use crate::generator::CodeGenerator;

/// Recursively copies all files and subdirectories from the source directory to the destination directory.
///
/// This function traverses the source directory, creating corresponding directories and copying files
/// into the destination directory. If the destination directory does not exist, it will be created.
/// All files and subdirectories are copied, preserving the directory structure.
///
/// # Arguments
///
/// * `source` - A reference to the path of the source directory to copy from.
/// * `destination` - A reference to the path of the destination directory to copy to.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the operation succeeds, or an error if any file or directory operation fails.
fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    if !destination.exists() {
        fs::create_dir_all(destination)?;
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(source_path.strip_prefix(source)?);

        if file_type.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}

/// Generates MCP server code from an OpenAPI specification.
///
/// # Arguments
///
/// * `openapi_file` - Path to the OpenAPI specification file.
/// * `output_dir` - Directory where the generated code will be written.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the generation succeeds, or an error if any step fails.
fn generate_mcp_server(openapi_file: &Path, output_dir: &Path) -> Result<()> {
    // Check if the output directory already exists.
    if output_dir.exists() {
        return Err(OpenApiToMcpError::OutputDirectoryExists(
            output_dir.to_path_buf(),
        ));
    }

    // Create the output directory.
    fs::create_dir_all(output_dir)
        .map_err(|_| OpenApiToMcpError::OutputDirectoryCreation(output_dir.to_path_buf()))?;

    // Copy the templates directory to the output directory.
    let templates_directory = Path::new("templates");
    if !templates_directory.exists() {
        return Err(OpenApiToMcpError::TemplatesDirectoryNotFound);
    }
    copy_dir_all(templates_directory, output_dir).map_err(|_| OpenApiToMcpError::TemplatesCopy)?;

    // Read and parse the OpenAPI specification.
    let content = fs::read_to_string(openapi_file)
        .map_err(|_| OpenApiToMcpError::OpenApiFileRead(openapi_file.to_path_buf()))?;
    let openapi: Value =
        serde_json::from_str(&content).map_err(|_| OpenApiToMcpError::OpenApiParse)?;

    // Generate TypeScript code.
    let generator = CodeGenerator::new(openapi);
    let typescript = generator.generate();

    // Write index.ts to the output/src directory.
    let output_source = output_dir.join("src");
    fs::create_dir_all(&output_source).map_err(|_| OpenApiToMcpError::SrcDirectoryCreation)?;
    fs::write(output_source.join("index.ts"), typescript)
        .map_err(|_| OpenApiToMcpError::IndexFileWrite)?;

    println!(
        "Successfully generated TypeScript code in: {}",
        output_dir.display()
    );

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    generate_mcp_server(&args.file, &args.output)
}
