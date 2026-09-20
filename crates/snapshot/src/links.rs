use crate::{Snapshot, SnapshotError, git};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use warrant_core::nouns::InventoryClass;

pub(crate) fn classify(snapshot: &mut Snapshot) -> Result<(), SnapshotError> {
    let mut links = BTreeMap::new();
    let mut paths = BTreeSet::from([String::new()]);
    for entry in &snapshot.entries {
        let mut path = entry.path.as_str();
        loop {
            paths.insert(path.to_owned());
            let Some((parent, _)) = path.rsplit_once('/') else {
                break;
            };
            path = parent;
        }
        if snapshot.mode(&entry.path) == Some("120000")
            && entry.unread.is_none()
            && let Some(oid) = &entry.blob
        {
            links.insert(
                entry.path.clone(),
                git::run(&snapshot.repo, &["cat-file", "blob", oid], None, None)?,
            );
        }
    }
    for entry in &mut snapshot.entries {
        if links.contains_key(&entry.path) && !inside(&entry.path, &links, &paths) {
            entry.class = InventoryClass::Unread;
            entry.unread = Some("external-symlink".into());
            entry.reason = "symlink target is not resolvable inside the snapshot".into();
        }
    }
    Ok(())
}

fn inside(path: &str, links: &BTreeMap<String, Vec<u8>>, paths: &BTreeSet<String>) -> bool {
    let mut pending: VecDeque<String> = path.split('/').map(str::to_owned).collect();
    let mut resolved = Vec::new();
    let mut followed = 0;
    while let Some(part) = pending.pop_front() {
        match part.as_str() {
            "" | "." => continue,
            ".." => {
                if resolved.pop().is_none() {
                    return false;
                }
                continue;
            }
            _ => resolved.push(part),
        }
        let current = resolved.join("/");
        if let Some(bytes) = links.get(&current) {
            followed += 1;
            if followed > 40 {
                return false;
            }
            let Ok(target) = std::str::from_utf8(bytes) else {
                return false;
            };
            if target.is_empty() || target.starts_with('/') || target.contains('\\') {
                return false;
            }
            resolved.pop();
            for part in target.split('/').rev() {
                pending.push_front(part.into());
            }
        } else if !paths.contains(&current) {
            return false;
        }
    }
    paths.contains(&resolved.join("/"))
}
