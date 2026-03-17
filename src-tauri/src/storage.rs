use crate::models::ProbeResult;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn new(db_path: PathBuf) -> rusqlite::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS probes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                service_id TEXT NOT NULL,
                reachable INTEGER NOT NULL,
                status_code INTEGER,
                latency_ms INTEGER,
                error_type TEXT,
                estimated_bytes INTEGER NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_probe(&self, result: &ProbeResult) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO probes (timestamp, service_id, reachable, status_code, latency_ms, error_type, estimated_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                result.timestamp,
                result.service_id,
                result.reachable as i32,
                result.status_code.map(|v| v as i64),
                result.latency_ms,
                result.error_type,
                result.estimated_bytes,
            ],
        )?;
        Ok(())
    }

    pub fn load_recent_probes(
        &self,
        service_id: &str,
        since_ms: i64,
    ) -> rusqlite::Result<Vec<ProbeResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, service_id, reachable, status_code, latency_ms, error_type, estimated_bytes
             FROM probes
             WHERE service_id = ?1 AND timestamp >= ?2
             ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map(params![service_id, since_ms], |row| {
            Ok(ProbeResult {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                service_id: row.get(2)?,
                reachable: {
                    let v: i64 = row.get(3)?;
                    v != 0
                },
                status_code: row.get::<_, Option<i64>>(4)?.map(|v| v as u16),
                latency_ms: row.get(5)?,
                error_type: row.get(6)?,
                estimated_bytes: row.get(7)?,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// 加载所有服务在指定时间范围内的探测记录。
    pub fn load_recent_probes_all(
        &self,
        since_ms: i64,
    ) -> rusqlite::Result<Vec<ProbeResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, service_id, reachable, status_code, latency_ms, error_type, estimated_bytes
             FROM probes
             WHERE timestamp >= ?1
             ORDER BY service_id, timestamp ASC",
        )?;

        let rows = stmt.query_map(params![since_ms], |row| {
            Ok(ProbeResult {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                service_id: row.get(2)?,
                reachable: {
                    let v: i64 = row.get(3)?;
                    v != 0
                },
                status_code: row.get::<_, Option<i64>>(4)?.map(|v| v as u16),
                latency_ms: row.get(5)?,
                error_type: row.get(6)?,
                estimated_bytes: row.get(7)?,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}

