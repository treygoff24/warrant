use std::{fs, path::Path};

use warrant_core::manifest::WarrantManifest;

use crate::error::CommandError;

pub fn load_manifest(root: &Path) -> crate::error::Result<WarrantManifest> {
    let path = root.join("warrant/warrant.yaml");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            "schema_version: warrant.manifest/1\n".into()
        }
        Err(error) => {
            return Err(CommandError::evaluation(
                "manifest-io",
                format!("{}: {error}", path.display()),
                None,
            ));
        }
    };
    WarrantManifest::parse(&text).map_err(|error| {
        CommandError::evaluation(
            "invalid-manifest",
            error.to_string(),
            Some(path.display().to_string()),
        )
    })
}
