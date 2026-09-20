use std::{env, fs, path::PathBuf};

use warrant_core::schema::{DOCUMENTS, canonical_bytes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: warrant-core-schema <output-directory>")?;
    fs::create_dir_all(&output)?;

    let mut stub_count = 0;
    for document in DOCUMENTS {
        match document.generate() {
            Ok(schema) => {
                fs::write(
                    output.join(format!("{}.json", document.name)),
                    canonical_bytes(&schema)?,
                )?;
                println!("schema: {} implemented", document.name);
            }
            Err(_) => {
                stub_count += 1;
                println!("schema: {} stub", document.name);
            }
        }
    }
    println!("schema: stub-documents {stub_count}");
    Ok(())
}
