use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Serialize, de::DeserializeOwned};

const RECORDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("records");

pub struct SafePawDb {
    db: Database,
    path: PathBuf,
}

impl SafePawDb {
    pub fn open_default() -> Result<Self> {
        let path = default_db_path()?;
        Self::open(&path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create DB directory {}", parent.display()))?;
        }

        let db = Database::create(&path)
            .with_context(|| format!("failed to open database {}", path.display()))?;

        let write_txn = db
            .begin_write()
            .context("failed to start DB write transaction")?;
        write_txn
            .open_table(RECORDS_TABLE)
            .context("failed to open records table")?;
        write_txn
            .commit()
            .context("failed to initialize DB tables")?;

        Ok(Self { db, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn put_json<T: Serialize>(&self, namespace: &str, key: &str, value: &T) -> Result<()> {
        let full_key = namespaced_key(namespace, key);
        let bytes = serde_json::to_vec(value).context("failed to serialize DB record")?;

        let write_txn = self
            .db
            .begin_write()
            .context("failed to start DB write transaction")?;
        {
            let mut table = write_txn
                .open_table(RECORDS_TABLE)
                .context("failed to open records table")?;
            table
                .insert(full_key.as_str(), bytes.as_slice())
                .with_context(|| format!("failed to write DB record {full_key}"))?;
        }
        write_txn.commit().context("failed to commit DB write")?;

        Ok(())
    }

    pub fn get_json<T: DeserializeOwned>(&self, namespace: &str, key: &str) -> Result<Option<T>> {
        let full_key = namespaced_key(namespace, key);
        let read_txn = self
            .db
            .begin_read()
            .context("failed to start DB read transaction")?;
        let table = read_txn
            .open_table(RECORDS_TABLE)
            .context("failed to open records table")?;

        let Some(value) = table
            .get(full_key.as_str())
            .with_context(|| format!("failed to read DB record {full_key}"))?
        else {
            return Ok(None);
        };

        let record =
            serde_json::from_slice(value.value()).context("failed to deserialize DB record")?;

        Ok(Some(record))
    }

    pub fn delete(&self, namespace: &str, key: &str) -> Result<bool> {
        let full_key = namespaced_key(namespace, key);
        let write_txn = self
            .db
            .begin_write()
            .context("failed to start DB write transaction")?;
        let deleted = {
            let mut table = write_txn
                .open_table(RECORDS_TABLE)
                .context("failed to open records table")?;
            table
                .remove(full_key.as_str())
                .with_context(|| format!("failed to delete DB record {full_key}"))?
                .is_some()
        };
        write_txn.commit().context("failed to commit DB delete")?;

        Ok(deleted)
    }

    pub fn list_json<T: DeserializeOwned>(&self, namespace: &str, prefix: &str) -> Result<Vec<T>> {
        let namespace_prefix = format!("{namespace}:{prefix}");
        let read_txn = self
            .db
            .begin_read()
            .context("failed to start DB read transaction")?;
        let table = read_txn
            .open_table(RECORDS_TABLE)
            .context("failed to open records table")?;

        let mut values = Vec::new();
        for entry in table.iter().context("failed to iterate DB records")? {
            let (key, value) = entry.context("failed to read DB record during iteration")?;
            if key.value().starts_with(&namespace_prefix) {
                values.push(
                    serde_json::from_slice(value.value())
                        .context("failed to deserialize iterated DB record")?,
                );
            }
        }

        Ok(values)
    }
}

pub fn default_db_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".safepaw").join("safepaw.data"))
}

fn namespaced_key(namespace: &str, key: &str) -> String {
    format!("{namespace}:{key}")
}
