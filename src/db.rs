
use rusqlite::{params, Connection, Result};

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS vni (
    vlan INTEGER PRIMARY KEY,
    vni INTEGER NOT NULL UNIQUE
);

CREATE UNIQUE INDEX IF NOT EXISTS vni_vni ON vni (vni);
"#;

#[derive(Debug, Clone, Copy)]
pub struct Vni {
    pub vlan: u16,
    pub vni: u32,
}

#[derive(Debug)]
pub struct Database {
    conn: Connection,
    initial_vlan: u16,
}

impl Database {
    pub fn open<P: AsRef<std::path::Path>>(path: P, initial_vlan: u16) -> Result<Self> {
        // create the parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn, initial_vlan })
    }

    pub fn get_vlan(&self, vni: u32) -> Result<Option<u16>> {
        let mut stmt = self.conn.prepare("SELECT vlan FROM vni WHERE vni = ?")?;
        let mut rows = stmt.query(params![vni])?;
        let row = rows.next()?;
        match row {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn add_vni(&self, vni: u32) -> Result<u16> {
        let mut stmt = self.conn.prepare("SELECT vlan FROM vni ORDER BY vlan DESC LIMIT 1")?;
        let mut rows = stmt.query([])?;
        let row = rows.next()?;
        let vlan = row.map_or(self.initial_vlan, |row| { row.get::<_, u16>(0).unwrap() + 1u16 });
        
        self.conn.execute("INSERT INTO vni (vlan, vni) VALUES (?, ?) ON CONFLICT DO NOTHING", params![vlan, vni])?;

        let vlan = self.get_vlan(vni)?.unwrap();
        Ok(vlan)
    }

    pub fn remove_vni(&self, vni: u32) -> Result<u16> {
        let vlan = self.get_vlan(vni)?;
        self.conn.execute("DELETE FROM vni WHERE vni = ?", params![vni])?;
        Ok(vlan.unwrap())
    }

    pub fn list_vni(&self) -> Result<Vec<Vni>> {
        let mut stmt = self.conn.prepare("SELECT vlan, vni FROM vni")?;
        let rows = stmt.query_map([], |row| Ok(Vni { vlan: row.get(0)?, vni: row.get(1)? }))?;
        let mut vnis = Vec::new();
        for vni in rows {
            vnis.push(vni?);
        }
        Ok(vnis)
    }
}
