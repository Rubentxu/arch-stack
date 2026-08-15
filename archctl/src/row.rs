//! Row / Cell — typed representation of a single query result row.
//!
//! Before this module, `GraphStore::query` returned
//! `Vec<serde_json::Value>`. That leaked the serialisation crate into
//! the domain: every call site had to call `row.get("id")` on a
//! `serde_json::Map`, know the JSON semantics of `Value::Null` vs
//! missing keys, and live with the implicit `Value::String` wrap.
//!
//! `Row` + `Cell` replace that with a small typed pair that:
//! - preserves column order (matches Cypher SELECT order)
//! - offers `Row::column(name)` and `Row::get(name)` lookups
//! - offers typed accessors (`as_str`, `as_i64`, `as_bool`, `as_list`)
//! - hides `serde_json::Value` behind an explicit `to_json()` converter
//!   that lives at the adapter edge (CLI formatter)
//!
//! The domain stays agnostic of how the result will be serialised.

use serde_json::Value as Json;
use std::fmt;

/// One typed value cell. The variants are the minimal set we need to
/// model Cypher query results without falling back to JSON-shaped
/// ambiguity. `Bytes` is included for lbug `Value::Blob`; `Object` is
/// included for `Value::Struct` / `Value::Map`. Missing keys are not
/// modelled — `Row::get` returns `Option` instead.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<Cell>),
    /// Ordered key-value pairs. We use `Vec<(String, Cell)>` instead of
    /// `BTreeMap<String, Cell>` so the wire output preserves the
    /// column order produced by the engine (matters for diagnostic
    /// JSON where users expect keys in the same order they wrote
    /// `RETURN a, b, c`).
    Object(Vec<(String, Cell)>),
}

impl Cell {
    pub fn is_null(&self) -> bool {
        matches!(self, Cell::Null)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Cell::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Cell::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Cell::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Cell]> {
        match self {
            Cell::List(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Cell::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Cell::Float(f) => Some(*f),
            Cell::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Convert Cell::Object to serde_json::Map<String, Json> by building it.
    /// Returns None for non-Object variants.
    pub fn to_map(&self) -> Option<serde_json::Map<String, Json>> {
        match self {
            Cell::Object(kvs) => {
                let mut map = serde_json::Map::new();
                for (k, v) in kvs {
                    map.insert(k.clone(), v.to_json());
                }
                Some(map)
            }
            _ => None,
        }
    }

    /// Convert to `serde_json::Value`. Lives here (not in the
    /// adapter) because every Cell variant has an unambiguous JSON
    /// representation; the adapter only needs to call this when
    /// producing CLI/JSON output.
    ///
    /// Note: the domain never imports this. The `serde_json` crate
    /// only appears as the type of the return value, not as a
    /// domain concern.
    pub fn to_json(&self) -> Json {
        match self {
            Cell::Null => Json::Null,
            Cell::Bool(b) => Json::Bool(*b),
            Cell::Int(n) => Json::from(*n),
            Cell::Float(f) => serde_json::Number::from_f64(*f)
                .map(Json::Number)
                .unwrap_or(Json::Null),
            Cell::String(s) => Json::String(s.clone()),
            Cell::Bytes(b) => Json::from(format!("<bytes {}>", b.len())),
            Cell::List(items) => Json::Array(items.iter().map(Cell::to_json).collect()),
            Cell::Object(kvs) => {
                let mut map = serde_json::Map::new();
                for (k, v) in kvs {
                    map.insert(k.clone(), v.to_json());
                }
                Json::Object(map)
            }
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Null => f.write_str("null"),
            Cell::Bool(b) => write!(f, "{b}"),
            Cell::Int(n) => write!(f, "{n}"),
            Cell::Float(x) => write!(f, "{x}"),
            Cell::String(s) => write!(f, "{s}"),
            Cell::Bytes(b) => write!(f, "<bytes {}>", b.len()),
            Cell::List(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Cell::Object(kvs) => {
                f.write_str("{")?;
                for (i, (k, v)) in kvs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                f.write_str("}")
            }
        }
    }
}

/// One query result row. Columns preserve the order in which the
/// adapter pushed them, which mirrors the engine's RETURN clause.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    columns: Vec<(String, Cell)>,
}

impl Row {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a Row from positional cells (no column names).
    ///
    /// Used by `LbugStore::execute` (M51 prepared-statement path) where
    /// lbug does not expose column names through `QueryResult`. Callers
    /// that need column names should use `LbugStore::query` instead.
    ///
    /// `column_names()` returns `[]` (empty); `column(idx)` still works
    /// positionally.
    pub fn from_positional(cells: Vec<Cell>) -> Self {
        // Store cells under empty-string keys so `column(idx)` and
        // `len()` work positionally; `column_names()` filters out the
        // empty entries.
        let columns = cells.into_iter().map(|c| (String::new(), c)).collect();
        Self { columns }
    }

    pub fn push(&mut self, key: impl Into<String>, value: impl Into<Cell>) {
        self.columns.push((key.into(), value.into()));
    }

    pub fn get(&self, key: &str) -> Option<&Cell> {
        self.columns
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    /// Remove a column by name. Returns the removed cell, or None if not found.
    pub fn remove(&mut self, key: &str) -> Option<Cell> {
        let idx = self.columns.iter().position(|(k, _)| k == key)?;
        Some(self.columns.remove(idx).1)
    }

    /// Lookup by 0-based column index (position in the RETURN clause).
    pub fn column(&self, idx: usize) -> Option<(&str, &Cell)> {
        self.columns.get(idx).map(|(k, v)| (k.as_str(), v))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Cell)> {
        self.columns.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Names of the columns in order. Mirrors the RETURN clause of the
    /// query that produced this row.
    pub fn column_names(&self) -> Vec<&str> {
        // Filter out empty-string keys (used by `Row::from_positional` for
        // M51 prepared-statement results where lbug does not expose
        // column names).
        self.columns
            .iter()
            .filter_map(|(k, _)| if k.is_empty() { None } else { Some(k.as_str()) })
            .collect()
    }
}

impl From<bool> for Cell {
    fn from(v: bool) -> Self {
        Cell::Bool(v)
    }
}

impl From<i64> for Cell {
    fn from(v: i64) -> Self {
        Cell::Int(v)
    }
}

impl From<i32> for Cell {
    fn from(v: i32) -> Self {
        Cell::Int(v as i64)
    }
}

impl From<u32> for Cell {
    fn from(v: u32) -> Self {
        Cell::Int(v as i64)
    }
}

impl From<f64> for Cell {
    fn from(v: f64) -> Self {
        Cell::Float(v)
    }
}

impl From<String> for Cell {
    fn from(v: String) -> Self {
        Cell::String(v)
    }
}

impl From<&str> for Cell {
    fn from(v: &str) -> Self {
        Cell::String(v.to_string())
    }
}

impl From<Vec<u8>> for Cell {
    fn from(v: Vec<u8>) -> Self {
        Cell::Bytes(v)
    }
}

impl From<Vec<Cell>> for Cell {
    fn from(v: Vec<Cell>) -> Self {
        Cell::List(v)
    }
}

impl From<serde_json::Value> for Cell {
    /// Bridge from `serde_json::Value` so existing adapters (which
    /// already convert driver types to JSON) can lift back into the
    /// typed `Cell` hierarchy without going through `to_json`.
    ///
    /// Used by `LbugStore` to convert `lbug::Value` -> JSON -> `Cell`
    /// in one pass. The dual (`Cell::to_json`) handles the reverse.
    fn from(v: Json) -> Self {
        match v {
            Json::Null => Cell::Null,
            Json::Bool(b) => Cell::Bool(b),
            Json::Number(n) => n
                .as_i64()
                .map(Cell::Int)
                .or_else(|| n.as_f64().map(Cell::Float))
                .unwrap_or(Cell::Null),
            Json::String(s) => Cell::String(s),
            Json::Array(items) => Cell::List(items.into_iter().map(Cell::from).collect()),
            Json::Object(map) => {
                Cell::Object(map.into_iter().map(|(k, v)| (k, Cell::from(v))).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_preserves_insertion_order() {
        let mut r = Row::new();
        r.push("z", 1i64);
        r.push("a", "two");
        r.push("m", Cell::Bool(true));
        let names: Vec<&str> = r.column_names();
        assert_eq!(names, vec!["z", "a", "m"]);
    }

    #[test]
    fn row_get_returns_value() {
        let mut r = Row::new();
        r.push("id", "ev:1");
        r.push("count", 3i64);
        assert_eq!(r.get("id").and_then(Cell::as_str), Some("ev:1"));
        assert_eq!(r.get("count").and_then(Cell::as_i64), Some(3));
        assert!(r.get("missing").is_none());
    }

    #[test]
    fn row_column_returns_by_position() {
        let mut r = Row::new();
        r.push("a", 1i64);
        r.push("b", 2i64);
        let (k, v) = r.column(1).unwrap();
        assert_eq!(k, "b");
        assert_eq!(v.as_i64(), Some(2));
        assert!(r.column(99).is_none());
    }

    #[test]
    fn cell_is_null_and_typed_accessors() {
        assert!(Cell::Null.is_null());
        assert!(!Cell::Bool(false).is_null());

        assert_eq!(Cell::Bool(true).as_bool(), Some(true));
        assert_eq!(Cell::Int(7).as_i64(), Some(7));
        assert_eq!(Cell::String("hi".into()).as_str(), Some("hi"));
        assert!(Cell::Int(7).as_str().is_none());
        assert!(Cell::String("hi".into()).as_i64().is_none());
    }

    #[test]
    fn cell_to_json_round_trips_atomic_values() {
        assert_eq!(Cell::Null.to_json(), Json::Null);
        assert_eq!(Cell::Bool(true).to_json(), Json::Bool(true));
        assert_eq!(Cell::Int(42).to_json(), Json::from(42));
        assert_eq!(Cell::String("x".into()).to_json(), Json::String("x".into()));
    }

    #[test]
    fn cell_from_serde_json_covers_all_variants() {
        assert_eq!(Cell::from(Json::Null), Cell::Null);
        assert_eq!(Cell::from(Json::Bool(true)), Cell::Bool(true));
        assert_eq!(Cell::from(Json::from(5)), Cell::Int(5));
        assert_eq!(
            Cell::from(Json::String("hi".into())),
            Cell::String("hi".into())
        );
        assert_eq!(
            Cell::from(Json::Array(vec![Json::from(1), Json::from(2)])),
            Cell::List(vec![Cell::Int(1), Cell::Int(2)])
        );
        let mut obj = serde_json::Map::new();
        obj.insert("k".to_string(), Json::from(7));
        assert_eq!(
            Cell::from(Json::Object(obj.clone())),
            Cell::Object(vec![("k".to_string(), Cell::Int(7))])
        );
    }

    #[test]
    fn row_iter_yields_columns_in_order() {
        let mut r = Row::new();
        r.push("a", 1i64);
        r.push("b", 2i64);
        let pairs: Vec<(&str, i64)> = r.iter().map(|(k, v)| (k, v.as_i64().unwrap())).collect();
        assert_eq!(pairs, vec![("a", 1), ("b", 2)]);
    }

    #[test]
    fn row_object_cell_preserves_order_internally() {
        // Demonstrates that the Object variant keeps key order
        // internally — useful when the adapter walks the entries
        // for diagnostic output. The to_json() conversion goes through
        // serde_json::Map which sorts keys alphabetically (the
        // default JSON convention); the typed `iter()` preserves order.
        let mut r = Row::new();
        let inner = Cell::Object(vec![
            ("z".to_string(), Cell::Int(1)),
            ("a".to_string(), Cell::Int(2)),
        ]);
        r.push("node", inner);

        // Internal ordering is preserved.
        let obj_cell = r.get("node").unwrap();
        let mut keys: Vec<&str> = Vec::new();
        if let Cell::Object(kvs) = obj_cell {
            for (k, _) in kvs {
                keys.push(k.as_str());
            }
        } else {
            panic!("expected Cell::Object");
        }
        assert_eq!(keys, vec!["z", "a"]);

        // JSON conversion preserves insertion order (serde_json::Map with
        // `preserve_order` feature enabled transitively via merman-render →
        // dugong). Pre-M38, this used BTreeMap-backed ordering (alphabetical).
        // The contract now is "insertion order", which is also the more
        // intuitive expectation.
        let json = obj_cell.to_json();
        let obj = json.as_object().unwrap();
        let json_keys: Vec<&String> = obj.keys().collect();
        assert_eq!(json_keys, vec!["z", "a"]);
    }
}
