//! Read-only SQL access and the pre-declared W1.9 query extension point.

use std::path::Path;

use rusqlite::{
    Connection, OpenFlags,
    hooks::{AuthAction, AuthContext, Authorization},
    types::ValueRef,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ModelError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryLimits {
    pub rows: usize,
    pub bytes: usize,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            rows: 1_000,
            bytes: 1_048_576,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub truncated: bool,
    pub continuation: Option<String>,
}

/// A model connection that is read-only at both the SQLite and statement levels.
pub struct QueryStore {
    connection: Connection,
}

impl QueryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.authorizer(Some(authorize_select))?;
        Ok(Self { connection })
    }

    pub fn sql(&self, sql: &str, limits: QueryLimits) -> Result<QueryResult> {
        if limits.rows == 0 || limits.bytes == 0 {
            return Err(ModelError::Invalid(
                "query row and byte limits must both be greater than zero".into(),
            ));
        }

        let mut statement = self.connection.prepare(sql)?;
        if statement.column_count() == 0 {
            return Err(ModelError::Invalid(
                "query must be a SELECT statement".into(),
            ));
        }
        let columns = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let column_count = statement.column_count();
        let mut query = statement.query([])?;
        let mut rows = Vec::new();
        let mut used_bytes = 0usize;
        let mut truncated = false;

        while let Some(row) = query.next()? {
            if rows.len() == limits.rows {
                truncated = true;
                break;
            }
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                values.push(json_value(row.get_ref(index)?)?);
            }
            let row_bytes = serde_json::to_vec(&values)?.len();
            if used_bytes.saturating_add(row_bytes) > limits.bytes {
                truncated = true;
                break;
            }
            used_bytes += row_bytes;
            rows.push(values);
        }

        let continuation = truncated.then(|| format!("rerun with OFFSET {}", rows.len()));
        Ok(QueryResult {
            columns,
            rows,
            truncated,
            continuation,
        })
    }
}

fn authorize_select(context: AuthContext<'_>) -> Authorization {
    match context.action {
        AuthAction::Select
        | AuthAction::Read { .. }
        | AuthAction::Function { .. }
        | AuthAction::Recursive => Authorization::Allow,
        _ => Authorization::Deny,
    }
}

fn json_value(value: ValueRef<'_>) -> Result<Value> {
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

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

/// Placeholder for the higher-level owner/interface/consumer query surface.
pub fn answer(_question: &str, _arguments: &[String]) -> Result<QueryResult> {
    Err(ModelError::Invalid(
        "not-implemented: model question queries arrive in W1.9".into(),
    ))
}
