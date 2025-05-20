mod cli;
mod generator;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::cli::Args;
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

fn main() -> Result<()> {
    let arguments = Args::parse();

    // Check if the output directory already exists.
    if arguments.output.exists() {
        anyhow::bail!(
            "Output directory already exists: {}. Please remove it or choose a different location.",
            arguments.output.display()
        );
    }

    // Create the output directory.
    fs::create_dir_all(&arguments.output).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            arguments.output.display()
        )
    })?;

    // Copy the templates directory to the output directory.
    let templates_directory = Path::new("templates");
    if !templates_directory.exists() {
        anyhow::bail!("Templates directory not found");
    }
    copy_dir_all(templates_directory, &arguments.output)
        .with_context(|| "Failed to copy templates directory")?;

    // Read and parse the OpenAPI specification.
    let content = fs::read_to_string(&arguments.file)
        .with_context(|| format!("Failed to read file: {}", arguments.file.display()))?;
    let openapi: Value =
        serde_json::from_str(&content).with_context(|| "Failed to parse OpenAPI spec as JSON")?;

    // Generate TypeScript code.
    let generator = CodeGenerator::new(openapi);
    let typescript = generator.generate();

    // Write index.ts to the output/src directory.
    let output_source = arguments.output.join("src");
    fs::create_dir_all(&output_source)
        .with_context(|| "Failed to create src directory in output".to_string())?;
    fs::write(output_source.join("index.ts"), typescript)
        .with_context(|| "Failed to write index.ts")?;

    println!(
        "Successfully generated TypeScript code in: {}",
        arguments.output.display()
    );

    Ok(())
}
