pub const IMPLEMENTED: bool = true;

use std::{fs, path::Path};

use clap::{Args as ClapArgs, Subcommand};
use warrant_core::policy::{PolicyError, PolicySource};

use crate::{error::CommandError, manifest::load_manifest, output, repository};

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compile the manifest-selected policy files and print the effective policy.
    Compile,
    /// Run structural policy lint without a source snapshot.
    Lint,
    /// Explain the snapshot-specific effective policy for one subject.
    Effective { subject: String },
    /// Compare two effective policies.
    Diff { policies: Vec<String> },
}

pub fn run(args: Args, format: Option<crate::cli::Format>) -> crate::error::Result<()> {
    match args.command {
        Some(Command::Compile | Command::Lint) => compile(format),
        Some(Command::Effective { subject }) => Err(not_implemented(format!(
            "`warrant policy effective {subject}` needs the snapshot-dependent policy lane"
        ))),
        Some(Command::Diff { policies }) => Err(not_implemented(format!(
            "`warrant policy diff {}` needs the policy-classification lane",
            policies.join(" ")
        ))),
        None => Err(not_implemented(
            "`warrant policy` requires `compile`, `lint`, `effective`, or `diff`".into(),
        )),
    }
}

fn compile(format: Option<crate::cli::Format>) -> crate::error::Result<()> {
    let root = repository::root()?;
    let manifest = load_manifest(&root)?;
    let files = load_policy_files(&root, &manifest.policy.paths)?;
    let sources: Vec<_> = files
        .iter()
        .map(|(path, yaml)| PolicySource::new(path, yaml))
        .collect();
    let effective = warrant_core::policy::compile(&sources).map_err(policy_error)?;
    output::document(&effective, format)
}

fn load_policy_files(
    root: &Path,
    configured: &[String],
) -> crate::error::Result<Vec<(String, String)>> {
    let mut paths = Vec::new();
    for pattern in configured {
        let configured_path = Path::new(pattern);
        if configured_path.is_absolute()
            || configured_path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(CommandError::evaluation(
                "policy-path",
                format!("policy path `{pattern}` must stay inside the repository"),
                None,
            ));
        }
        if let Some(directory) = pattern.strip_suffix("/*.yaml") {
            let directory_path = root.join(directory);
            let entries = match fs::read_dir(&directory_path) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(CommandError::evaluation(
                        "policy-io",
                        format!("{directory}: {error}"),
                        None,
                    ));
                }
            };
            for entry in entries {
                let entry = entry.map_err(|error| {
                    CommandError::evaluation("policy-io", format!("{directory}: {error}"), None)
                })?;
                let path = entry.path();
                if path.extension().and_then(|extension| extension.to_str()) == Some("yaml") {
                    paths.push(path);
                }
            }
        } else if pattern.contains('*') {
            return Err(CommandError::evaluation(
                "policy-path",
                format!("unsupported policy path pattern `{pattern}`"),
                Some("use an exact file or a directory pattern ending in `/*.yaml`".into()),
            ));
        } else {
            paths.push(root.join(pattern));
        }
    }
    paths.sort();
    paths.dedup();

    paths
        .into_iter()
        .map(|path| {
            let relative = path.strip_prefix(root).map_err(|_| {
                CommandError::evaluation(
                    "policy-path",
                    "policy files must be inside the repository",
                    None,
                )
            })?;
            let display = relative.to_string_lossy().replace('\\', "/");
            let yaml = fs::read_to_string(&path).map_err(|error| {
                CommandError::evaluation("policy-io", format!("{display}: {error}"), None)
            })?;
            Ok((display, yaml))
        })
        .collect()
}

fn policy_error(error: PolicyError) -> Box<CommandError> {
    CommandError::evaluation(
        error.code(),
        error.to_string(),
        Some("run `warrant policy lint` after correcting the named policy fields".into()),
    )
}

fn not_implemented(reason: String) -> Box<CommandError> {
    CommandError::evaluation("not-implemented", reason, None)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::load_policy_files;

    #[test]
    fn policy_loader_sorts_files_and_ignores_non_yaml_entries() {
        let root = TempDir::new().expect("temporary repository");
        let directory = root.path().join("warrant/policy");
        fs::create_dir_all(&directory).expect("policy directory");
        fs::write(directory.join("b.yaml"), "b").expect("second policy");
        fs::write(directory.join("a.yaml"), "a").expect("first policy");
        fs::write(directory.join("notes.md"), "ignored").expect("notes");

        let files = load_policy_files(root.path(), &["warrant/policy/*.yaml".into()])
            .expect("load policy files");

        assert_eq!(
            files,
            [
                ("warrant/policy/a.yaml".into(), "a".into()),
                ("warrant/policy/b.yaml".into(), "b".into()),
            ]
        );
    }
}
