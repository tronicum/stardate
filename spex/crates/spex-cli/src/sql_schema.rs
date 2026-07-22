use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::path::Path;
use std::process::Command;

/// Runs the real `sqlite3` CLI against a database file and converts its
/// schema into a `spex_graph::Graph`: one node per table, real row count
/// (`SELECT COUNT(*)`) driving color, real columns + foreign keys (via
/// `PRAGMA table_info`/`PRAGMA foreign_key_list`) as metadata. A table's
/// first foreign key becomes its tree parent — a table with none (or whose
/// only FKs are self-referential) is a root of its own tree in the forest,
/// same as `disk_usage`'s "parent not found in the captured set" pattern.
/// Only the first FK is used as the tree edge (`Graph` is tree-only, same
/// known limitation as `brew-deps`'s shared-dependency duplication); any
/// additional FKs are still recorded in metadata so no information is lost,
/// just not drawn as a second parent edge.
pub fn run(db_path: &Path) -> Result<Graph> {
    if !db_path.exists() {
        bail!("{} does not exist", db_path.display());
    }
    let tables = query_column(db_path, "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;")?;
    if tables.is_empty() {
        bail!("{} has no user tables", db_path.display());
    }

    let mut nodes = Vec::with_capacity(tables.len());
    for table in &tables {
        let row_count: f64 = query_scalar(db_path, &format!("SELECT COUNT(*) FROM \"{table}\";"))?;
        let foreign_keys = foreign_key_list(db_path, table)?;
        let columns = table_info(db_path, table)?;

        let parent = foreign_keys
            .iter()
            .map(|fk| fk.to_table.clone())
            .find(|target| target != table && tables.contains(target));

        let mut metadata = Map::new();
        metadata.insert("rowCount".to_string(), Value::from(row_count));
        metadata.insert(
            "columns".to_string(),
            Value::from(columns.iter().map(|c| format!("{} {}", c.name, c.ty)).collect::<Vec<_>>()),
        );
        metadata.insert(
            "foreignKeys".to_string(),
            Value::from(
                foreign_keys
                    .iter()
                    .map(|fk| format!("{} -> {}.{}", fk.from_column, fk.to_table, fk.to_column))
                    .collect::<Vec<_>>(),
            ),
        );

        nodes.push(GraphNode {
            id: table.clone(),
            label: table.clone(),
            parent,
            metric: Some(row_count),
            metadata,
        });
    }

    Ok(Graph {
        title: Some(format!("SQL schema: {}", db_path.display())),
        metric_label: Some("row count".to_string()),
        nodes,
    })
}

struct ForeignKey {
    from_column: String,
    to_table: String,
    to_column: String,
}

/// `PRAGMA foreign_key_list(<table>)` columns: id|seq|table|from|to|on_update|on_delete|match
fn foreign_key_list(db_path: &Path, table: &str) -> Result<Vec<ForeignKey>> {
    let rows = run_sqlite(db_path, &format!("PRAGMA foreign_key_list(\"{table}\");"))?;
    Ok(rows
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('|').collect();
            Some(ForeignKey {
                to_table: cols.get(2)?.to_string(),
                from_column: cols.get(3)?.to_string(),
                to_column: cols.get(4)?.to_string(),
            })
        })
        .collect())
}

struct Column {
    name: String,
    ty: String,
}

/// `PRAGMA table_info(<table>)` columns: cid|name|type|notnull|dflt_value|pk
fn table_info(db_path: &Path, table: &str) -> Result<Vec<Column>> {
    let rows = run_sqlite(db_path, &format!("PRAGMA table_info(\"{table}\");"))?;
    Ok(rows
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('|').collect();
            Some(Column {
                name: cols.get(1)?.to_string(),
                ty: cols.get(2)?.to_string(),
            })
        })
        .collect())
}

fn query_column(db_path: &Path, sql: &str) -> Result<Vec<String>> {
    let out = run_sqlite(db_path, sql)?;
    Ok(out.lines().filter(|l| !l.is_empty()).map(|s| s.to_string()).collect())
}

fn query_scalar(db_path: &Path, sql: &str) -> Result<f64> {
    let out = run_sqlite(db_path, sql)?;
    out.trim().parse().with_context(|| format!("expected a numeric result from `{sql}`, got {out:?}"))
}

fn run_sqlite(db_path: &Path, sql: &str) -> Result<String> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .context("running `sqlite3` (is it on PATH?)")?;
    if !output.status.success() {
        bail!("sqlite3 failed on `{sql}`: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    fn sample_db(dir: &Path) -> std::path::PathBuf {
        let db_path = dir.join("sample.db");
        let sql = "\
            CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT);\n\
            CREATE TABLE orders (id INTEGER PRIMARY KEY, customer_id INTEGER, \
                FOREIGN KEY(customer_id) REFERENCES customers(id));\n\
            INSERT INTO customers VALUES (1, 'Alice'), (2, 'Bob');\n\
            INSERT INTO orders VALUES (1, 1), (2, 1), (3, 2);\n\
        ";
        let status = StdCommand::new("sqlite3")
            .arg(&db_path)
            .arg(sql)
            .status()
            .expect("sqlite3 must be on PATH to run this test");
        assert!(status.success());
        db_path
    }

    #[test]
    fn builds_a_forest_from_real_foreign_keys_and_row_counts() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = sample_db(dir.path());

        let graph = run(&db_path).unwrap();
        assert_eq!(graph.nodes.len(), 2);

        let customers = graph.nodes.iter().find(|n| n.id == "customers").unwrap();
        assert_eq!(customers.parent, None, "customers has no FK, so it's a forest root");
        assert_eq!(customers.metric, Some(2.0));

        let orders = graph.nodes.iter().find(|n| n.id == "orders").unwrap();
        assert_eq!(orders.parent.as_deref(), Some("customers"));
        assert_eq!(orders.metric, Some(3.0));
        let fks = orders.metadata.get("foreignKeys").unwrap().as_array().unwrap();
        assert_eq!(fks.len(), 1);
        assert_eq!(fks[0], "customer_id -> customers.id");
    }

    #[test]
    fn errors_on_missing_db_file() {
        let err = run(Path::new("/no/such/file.db")).unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }
}
