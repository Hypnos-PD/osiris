use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use std::collections::HashMap;
// Arc is unused, keep for API if needed
// Arc intentionally unused in this module

#[derive(Debug, Clone)]
pub struct CardData {
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub type_: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u32,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
}

pub struct Database {
    pub conn: Connection,
    pub cache: HashMap<u32, CardData>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Database { conn, cache: HashMap::new() })
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Database { conn, cache: HashMap::new() })
    }

    pub fn query_card(&mut self, code: u32) -> rusqlite::Result<Option<CardData>> {
        if let Some(cached) = self.cache.get(&code) {
            return Ok(Some(cached.clone()));
        }
        let mut stmt = self.conn.prepare("SELECT id, alias, setcode, type, level, attribute, race, atk, def FROM datas WHERE id = ?")?;
        let row_opt = stmt.query_row(params![code], |r| {
            let id: u32 = r.get(0)?;
            let alias: u32 = r.get(1)?;
            let setcode: i64 = r.get(2)?; // sqlite stores as integer; map to u64
            let type_: u32 = r.get(3)?;
            let level: u32 = r.get(4)?;
            let attribute: u32 = r.get(5)?;
            let race: u32 = r.get(6)?;
            let atk: i32 = r.get(7)?;
            let def: i32 = r.get(8)?;
            Ok(CardData {
                code: id,
                alias,
                setcode: setcode as u64,
                type_, level, attribute, race,
                attack: atk,
                defense: def,
                lscale: 0,
                rscale: 0,
            })
        }).optional()?;
        if let Some(card) = &row_opt {
            self.cache.insert(code, card.clone());
        }
        Ok(row_opt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_query() {
        let mut db = Database::open_in_memory().expect("open in memory");
        db.conn.execute(
            "CREATE TABLE datas (id INTEGER, alias INTEGER, setcode INTEGER, type INTEGER, level INTEGER, attribute INTEGER, race INTEGER, atk INTEGER, def INTEGER);",
            params![]
        ).unwrap();
        db.conn.execute(
            "INSERT INTO datas (id, alias, setcode, type, level, attribute, race, atk, def) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![12345i64, 0i64, 0i64, 1i64, 4i64, 1i64, 1i64, 1500i64, 1200i64])
            .unwrap();
        let result = db.query_card(12345).expect("query ok");
        assert!(result.is_some());
        let c = result.unwrap();
        assert_eq!(c.code, 12345);
        assert_eq!(c.attack, 1500);
        assert_eq!(c.defense, 1200);
    }
}
