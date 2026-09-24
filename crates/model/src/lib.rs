//! SQLite-backed program model storage.

use std::{
    collections::BTreeMap,
    error::Error as StdError,
    fmt, fs,
    path::{Path, PathBuf},
};

use petgraph::{algo::kosaraju_scc, graphmap::DiGraphMap};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params, types::ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub mod declared;
pub mod query;

/// The checked-in schema help returned by `warrant query --schema`.
pub const MODEL_SCHEMA: &str = include_str!("../../../schemas/model.md");

const SCHEMA: &str = r#"
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE units (
    id INTEGER PRIMARY KEY,
    integration TEXT NOT NULL,
    root TEXT NOT NULL,
    config_path TEXT,
    kind TEXT NOT NULL
);
CREATE TABLE modules (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    declared_by TEXT NOT NULL,
    intent TEXT,
    owner TEXT
);
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    blob TEXT NOT NULL,
    class TEXT NOT NULL,
    language TEXT,
    unit_id INTEGER NOT NULL REFERENCES units(id) DEFERRABLE INITIALLY DEFERRED,
    module_id INTEGER REFERENCES modules(id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE symbols (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) DEFERRABLE INITIALLY DEFERRED,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    exported INTEGER NOT NULL,
    export_name TEXT,
    visibility TEXT NOT NULL,
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL
);
CREATE TABLE edges (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    from_file INTEGER NOT NULL REFERENCES files(id) DEFERRABLE INITIALLY DEFERRED,
    from_symbol INTEGER REFERENCES symbols(id) DEFERRABLE INITIALLY DEFERRED,
    to_file INTEGER REFERENCES files(id) DEFERRABLE INITIALLY DEFERRED,
    to_symbol INTEGER REFERENCES symbols(id) DEFERRABLE INITIALLY DEFERRED,
    to_external TEXT,
    resolved INTEGER NOT NULL,
    unresolved_reason TEXT,
    type_only INTEGER NOT NULL,
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL,
    basis TEXT NOT NULL
);
CREATE TABLE "references" (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) DEFERRABLE INITIALLY DEFERRED,
    symbol_id INTEGER NOT NULL REFERENCES symbols(id) DEFERRABLE INITIALLY DEFERRED,
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL
);
CREATE TABLE entrypoints (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) DEFERRABLE INITIALLY DEFERRED,
    symbol_id INTEGER REFERENCES symbols(id) DEFERRABLE INITIALLY DEFERRED,
    kind TEXT NOT NULL,
    basis TEXT NOT NULL,
    declared_by TEXT
);
CREATE TABLE effects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    symbol_id INTEGER NOT NULL REFERENCES symbols(id) DEFERRABLE INITIALLY DEFERRED,
    declared_by TEXT NOT NULL
);
CREATE TABLE unsupported (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) DEFERRABLE INITIALLY DEFERRED,
    construct TEXT NOT NULL,
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL,
    reason TEXT NOT NULL
);
CREATE TABLE capability_reports (
    integration TEXT PRIMARY KEY,
    json TEXT NOT NULL
);

-- SQLite has no native materialized views. These tables are rebuilt at finish.
CREATE TABLE module_edges (
    from_module INTEGER NOT NULL REFERENCES modules(id),
    to_module INTEGER NOT NULL REFERENCES modules(id),
    edge_count INTEGER NOT NULL,
    type_only_count INTEGER NOT NULL,
    first_edge_id INTEGER NOT NULL REFERENCES edges(id),
    PRIMARY KEY (from_module, to_module)
);
CREATE TABLE symbol_consumers (
    symbol_id INTEGER NOT NULL REFERENCES symbols(id),
    consumer_file INTEGER NOT NULL REFERENCES files(id),
    via_reexport_chain INTEGER NOT NULL,
    PRIMARY KEY (symbol_id, consumer_file)
);
CREATE TABLE module_cycles (
    cycle_id INTEGER NOT NULL,
    module_id INTEGER NOT NULL REFERENCES modules(id),
    position INTEGER NOT NULL,
    PRIMARY KEY (cycle_id, position),
    UNIQUE (cycle_id, module_id)
);
CREATE VIEW unowned_files AS
    SELECT files.* FROM files
    WHERE module_id IS NULL AND class = 'source';
"#;

/// Errors returned while building or querying a model.
#[derive(Debug)]
pub enum ModelError {
    Sql(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(error) => write!(formatter, "SQLite error: {error}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl StdError for ModelError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Sql(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<rusqlite::Error> for ModelError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<std::io::Error> for ModelError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub type Result<T> = std::result::Result<T, ModelError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitRow {
    pub id: i64,
    pub integration: String,
    pub root: String,
    pub config_path: Option<String>,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRow {
    pub id: i64,
    pub path: String,
    pub blob: String,
    pub class: String,
    pub language: Option<String>,
    pub unit_id: i64,
    pub module_id: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleRow {
    pub id: i64,
    pub name: String,
    pub declared_by: String,
    pub intent: Option<String>,
    pub owner: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolRow {
    pub id: i64,
    pub file_id: i64,
    pub name: String,
    pub kind: String,
    pub exported: bool,
    pub export_name: Option<String>,
    pub visibility: String,
    pub span_start: i64,
    pub span_end: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRow {
    pub id: i64,
    pub kind: String,
    pub from_file: i64,
    pub from_symbol: Option<i64>,
    pub to_file: Option<i64>,
    pub to_symbol: Option<i64>,
    pub to_external: Option<String>,
    pub resolved: bool,
    pub unresolved_reason: Option<String>,
    pub type_only: bool,
    pub span_start: i64,
    pub span_end: i64,
    pub basis: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceRow {
    pub id: i64,
    pub file_id: i64,
    pub symbol_id: i64,
    pub span_start: i64,
    pub span_end: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrypointRow {
    pub id: i64,
    pub file_id: i64,
    pub symbol_id: Option<i64>,
    pub kind: String,
    pub basis: String,
    pub declared_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectRow {
    pub id: i64,
    pub name: String,
    pub symbol_id: i64,
    pub declared_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedRow {
    pub id: i64,
    pub file_id: i64,
    pub construct: String,
    pub span_start: i64,
    pub span_end: i64,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CapabilityReportRow {
    pub integration: String,
    pub json: Value,
}

/// One integration's streaming contribution for a compilation unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegrationReport {
    pub unit: UnitRow,
    pub files: Vec<FileRow>,
    pub symbols: Vec<SymbolRow>,
    pub edges: Vec<EdgeRow>,
    pub references: Vec<ReferenceRow>,
    pub entrypoints: Vec<EntrypointRow>,
    pub effects: Vec<EffectRow>,
    pub unsupported: Vec<UnsupportedRow>,
    pub capability_report: CapabilityReportRow,
}

/// A single-use streaming writer. Dropping it before `finish` rolls back the model.
pub struct ModelBuilder {
    path: PathBuf,
    connection: Connection,
}

impl ModelBuilder {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            return Err(ModelError::Invalid(format!(
                "model already exists: {}",
                path.display()
            )));
        }
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        connection.execute_batch(SCHEMA)?;
        connection.execute_batch("BEGIN IMMEDIATE;")?;
        Ok(Self {
            path: path.to_path_buf(),
            connection,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write_meta(&mut self, key: &str, value: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO meta(key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn write_unit(&mut self, row: &UnitRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO units(id, integration, root, config_path, kind) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![row.id, row.integration, row.root, row.config_path, row.kind],
        )?;
        declared::write_for_unit(self, row)
    }

    pub fn write_file(&mut self, row: &FileRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO files(id, path, blob, class, language, unit_id, module_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![row.id, row.path, row.blob, row.class, row.language, row.unit_id, row.module_id],
        )?;
        Ok(())
    }

    pub fn write_module(&mut self, row: &ModuleRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO modules(id, name, declared_by, intent, owner) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![row.id, row.name, row.declared_by, row.intent, row.owner],
        )?;
        Ok(())
    }

    pub fn write_symbol(&mut self, row: &SymbolRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO symbols(id, file_id, name, kind, exported, export_name, visibility, span_start, span_end) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![row.id, row.file_id, row.name, row.kind, row.exported, row.export_name, row.visibility, row.span_start, row.span_end],
        )?;
        Ok(())
    }

    pub fn write_edge(&mut self, row: &EdgeRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO edges(id, kind, from_file, from_symbol, to_file, to_symbol, to_external, resolved, unresolved_reason, type_only, span_start, span_end, basis) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![row.id, row.kind, row.from_file, row.from_symbol, row.to_file, row.to_symbol, row.to_external, row.resolved, row.unresolved_reason, row.type_only, row.span_start, row.span_end, row.basis],
        )?;
        Ok(())
    }

    pub fn write_reference(&mut self, row: &ReferenceRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO \"references\"(id, file_id, symbol_id, span_start, span_end) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![row.id, row.file_id, row.symbol_id, row.span_start, row.span_end],
        )?;
        Ok(())
    }

    pub fn write_entrypoint(&mut self, row: &EntrypointRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO entrypoints(id, file_id, symbol_id, kind, basis, declared_by) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![row.id, row.file_id, row.symbol_id, row.kind, row.basis, row.declared_by],
        )?;
        Ok(())
    }

    pub fn write_effect(&mut self, row: &EffectRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO effects(id, name, symbol_id, declared_by) VALUES (?1, ?2, ?3, ?4)",
            params![row.id, row.name, row.symbol_id, row.declared_by],
        )?;
        Ok(())
    }

    pub fn write_unsupported(&mut self, row: &UnsupportedRow) -> Result<()> {
        self.connection.execute(
            "INSERT INTO unsupported(id, file_id, construct, span_start, span_end, reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![row.id, row.file_id, row.construct, row.span_start, row.span_end, row.reason],
        )?;
        Ok(())
    }

    pub fn write_capability_report(&mut self, row: &CapabilityReportRow) -> Result<()> {
        let json = serde_json_canonicalizer::to_string(&row.json)?;
        let stored: Option<String> = self
            .connection
            .query_row(
                "SELECT json FROM capability_reports WHERE integration = ?1",
                [&row.integration],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(stored) = stored {
            return if stored == json {
                Ok(())
            } else {
                Err(ModelError::Invalid(format!(
                    "conflicting capability report for integration {}",
                    row.integration
                )))
            };
        }
        self.connection.execute(
            "INSERT INTO capability_reports(integration, json) VALUES (?1, ?2)",
            params![row.integration, json],
        )?;
        Ok(())
    }

    pub fn write_report(&mut self, report: &IntegrationReport) -> Result<()> {
        self.write_unit(&report.unit)?;
        for row in &report.files {
            self.write_file(row)?;
        }
        for row in &report.symbols {
            self.write_symbol(row)?;
        }
        for row in &report.edges {
            self.write_edge(row)?;
        }
        for row in &report.references {
            self.write_reference(row)?;
        }
        for row in &report.entrypoints {
            self.write_entrypoint(row)?;
        }
        for row in &report.effects {
            self.write_effect(row)?;
        }
        for row in &report.unsupported {
            self.write_unsupported(row)?;
        }
        self.write_capability_report(&report.capability_report)
    }

    pub fn finish(mut self) -> Result<String> {
        self.materialize_views()?;
        let digest = canonical_digest(&self.connection)?;
        self.connection.execute(
            "INSERT INTO meta(key, value) VALUES ('model_digest', ?1)",
            [&digest],
        )?;
        self.connection.execute_batch("COMMIT;")?;
        Ok(digest)
    }

    fn materialize_views(&mut self) -> Result<()> {
        self.connection.execute_batch(
            "DELETE FROM module_edges;
             DELETE FROM symbol_consumers;
             DELETE FROM module_cycles;
             INSERT INTO module_edges(from_module, to_module, edge_count, type_only_count, first_edge_id)
             SELECT source.module_id, target.module_id, COUNT(*), SUM(edges.type_only), MIN(edges.id)
             FROM edges
             JOIN files AS source ON source.id = edges.from_file
             JOIN files AS target ON target.id = edges.to_file
             WHERE source.module_id IS NOT NULL AND target.module_id IS NOT NULL
                   AND source.module_id != target.module_id
             GROUP BY source.module_id, target.module_id;
             INSERT INTO symbol_consumers(symbol_id, consumer_file, via_reexport_chain)
             SELECT to_symbol, from_file, MAX(kind = 'reexport')
             FROM edges
             WHERE to_symbol IS NOT NULL
             GROUP BY to_symbol, from_file;",
        )?;

        let mut graph = DiGraphMap::<i64, ()>::new();
        {
            let mut modules = self
                .connection
                .prepare("SELECT id FROM modules ORDER BY id")?;
            let ids = modules
                .query_map([], |row| row.get::<_, i64>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for id in ids {
                graph.add_node(id);
            }
        }
        {
            let mut edges = self
                .connection
                .prepare("SELECT from_module, to_module FROM module_edges")?;
            let pairs = edges
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (from, to) in pairs {
                graph.add_edge(from, to, ());
            }
        }

        let mut cycles = kosaraju_scc(&graph)
            .into_iter()
            .filter(|component| {
                component.len() > 1
                    || component
                        .first()
                        .is_some_and(|module| graph.contains_edge(*module, *module))
            })
            .map(|mut component| {
                component.sort_unstable();
                component
            })
            .collect::<Vec<_>>();
        cycles.sort();
        for (cycle_index, cycle) in cycles.iter().enumerate() {
            let cycle_id = i64::try_from(cycle_index + 1)
                .map_err(|_| ModelError::Invalid("too many module cycles".into()))?;
            for (position, module_id) in cycle.iter().enumerate() {
                let position = i64::try_from(position)
                    .map_err(|_| ModelError::Invalid("module cycle is too large".into()))?;
                self.connection.execute(
                    "INSERT INTO module_cycles(cycle_id, module_id, position) VALUES (?1, ?2, ?3)",
                    params![cycle_id, module_id, position],
                )?;
            }
        }
        Ok(())
    }
}

/// Build a complete model from module declarations and streamed integration reports.
pub fn build_model(
    path: impl AsRef<Path>,
    modules: impl IntoIterator<Item = ModuleRow>,
    reports: impl IntoIterator<Item = IntegrationReport>,
) -> Result<String> {
    let mut builder = ModelBuilder::create(path)?;
    for module in modules {
        builder.write_module(&module)?;
    }
    for report in reports {
        builder.write_report(&report)?;
    }
    builder.finish()
}

fn canonical_digest(connection: &Connection) -> Result<String> {
    const TABLES: &[(&str, &str)] = &[
        ("meta", "key"),
        ("units", "id"),
        ("files", "id"),
        ("modules", "id"),
        ("symbols", "id"),
        ("edges", "id"),
        ("\"references\"", "id"),
        ("entrypoints", "id"),
        ("effects", "id"),
        ("unsupported", "id"),
        ("capability_reports", "integration"),
        ("module_edges", "from_module, to_module"),
        ("symbol_consumers", "symbol_id, consumer_file"),
        ("module_cycles", "cycle_id, position"),
    ];

    let mut dump = Vec::with_capacity(TABLES.len());
    for (table, order) in TABLES {
        let sql = if *table == "meta" {
            "SELECT * FROM meta WHERE key != 'model_digest' AND key NOT LIKE '%_at' ORDER BY key"
                .to_owned()
        } else {
            format!("SELECT * FROM {table} ORDER BY {order}")
        };
        let mut statement = connection.prepare(&sql)?;
        let column_count = statement.column_count();
        let mut query = statement.query([])?;
        let mut rows = Vec::new();
        while let Some(row) = query.next()? {
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                values.push(sql_value(row.get_ref(index)?)?);
            }
            if *table == "meta" {
                normalize_meta_row(&mut values);
            }
            rows.push(Value::Array(values));
        }
        dump.push(json!({"table": table.trim_matches('"'), "rows": rows}));
    }
    let bytes = serde_json_canonicalizer::to_vec(&dump)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{}", hex(&digest)))
}

fn sql_value(value: ValueRef<'_>) -> Result<Value> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::from(value),
        ValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| ModelError::Invalid("non-finite SQLite value".into()))?,
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::String(format!("blob:{}", hex(value))),
    })
}

fn normalize_meta_row(row: &mut [Value]) {
    let Some(Value::String(value)) = row.get_mut(1) else {
        return;
    };
    let Ok(mut json) = serde_json::from_str::<Value>(value) else {
        return;
    };
    remove_incidental_timestamps(&mut json);
    if let Ok(canonical) = serde_json_canonicalizer::to_string(&json) {
        *value = canonical;
    }
}

fn remove_incidental_timestamps(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                remove_incidental_timestamps(value);
            }
        }
        Value::Object(values) => {
            values.retain(|key, _| !is_timestamp_key(key));
            for value in values.values_mut() {
                remove_incidental_timestamps(value);
            }
        }
        _ => {}
    }
}

fn is_timestamp_key(key: &str) -> bool {
    matches!(
        key,
        "timestamp" | "time" | "taken_at" | "created_at" | "updated_at" | "built_at"
    ) || key.ends_with("_timestamp")
}

fn hex(bytes: &[u8]) -> String {
    use fmt::Write;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

/// Return a stable table-to-columns map for `warrant query --schema` callers.
pub fn schema_columns() -> BTreeMap<&'static str, &'static [&'static str]> {
    BTreeMap::from([
        ("meta", &["key", "value"] as &[_]),
        (
            "units",
            &["id", "integration", "root", "config_path", "kind"] as &[_],
        ),
        (
            "files",
            &[
                "id",
                "path",
                "blob",
                "class",
                "language",
                "unit_id",
                "module_id",
            ] as &[_],
        ),
        (
            "modules",
            &["id", "name", "declared_by", "intent", "owner"] as &[_],
        ),
        (
            "symbols",
            &[
                "id",
                "file_id",
                "name",
                "kind",
                "exported",
                "export_name",
                "visibility",
                "span_start",
                "span_end",
            ] as &[_],
        ),
        (
            "edges",
            &[
                "id",
                "kind",
                "from_file",
                "from_symbol",
                "to_file",
                "to_symbol",
                "to_external",
                "resolved",
                "unresolved_reason",
                "type_only",
                "span_start",
                "span_end",
                "basis",
            ] as &[_],
        ),
        (
            "references",
            &["id", "file_id", "symbol_id", "span_start", "span_end"] as &[_],
        ),
        (
            "entrypoints",
            &["id", "file_id", "symbol_id", "kind", "basis", "declared_by"] as &[_],
        ),
        (
            "effects",
            &["id", "name", "symbol_id", "declared_by"] as &[_],
        ),
        (
            "unsupported",
            &[
                "id",
                "file_id",
                "construct",
                "span_start",
                "span_end",
                "reason",
            ] as &[_],
        ),
        ("capability_reports", &["integration", "json"] as &[_]),
        (
            "module_edges",
            &[
                "from_module",
                "to_module",
                "edge_count",
                "type_only_count",
                "first_edge_id",
            ] as &[_],
        ),
        (
            "symbol_consumers",
            &["symbol_id", "consumer_file", "via_reexport_chain"] as &[_],
        ),
        (
            "module_cycles",
            &["cycle_id", "module_id", "position"] as &[_],
        ),
        (
            "unowned_files",
            &[
                "id",
                "path",
                "blob",
                "class",
                "language",
                "unit_id",
                "module_id",
            ] as &[_],
        ),
    ])
}
