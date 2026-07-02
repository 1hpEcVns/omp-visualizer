use rusqlite::{Connection, params};
use std::fs;
use sha2::{Sha256, Digest};

/// Manages the SQLite cache database for fast session indexing and search.
pub struct SessionIndex {
    conn: Connection,
}

impl SessionIndex {
    /// Open or create the index database.
    pub fn open(db_path: Option<&str>) -> Result<Self, String> {
        let path = db_path.map(|p| p.to_string()).unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let cache_dir = format!("{}/.cache/omp-visualizer", home);
            fs::create_dir_all(&cache_dir).ok();
            format!("{}/index.db", cache_dir)
        });

        let conn = Connection::open(&path)
            .map_err(|e| format!("Cannot open index db: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| format!("Pragma error: {}", e))?;

        let mut index = SessionIndex { conn };
        index.init_schema()?;
        Ok(index)
    }

    fn init_schema(&mut self) -> Result<(), String> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS session_index (
                id TEXT PRIMARY KEY,
                title TEXT,
                directory TEXT,
                timestamp TEXT,
                message_count INTEGER DEFAULT 0,
                subagent_count INTEGER DEFAULT 0,
                fingerprint TEXT,
                updated_at TEXT
            );

            CREATE TABLE IF NOT EXISTS boot_cache (
                session_id TEXT PRIMARY KEY,
                fingerprint TEXT,
                payload BLOB,
                created_at TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS session_search USING fts5(
                session_id, title, directory, content
            );
        ").map_err(|e| format!("Schema error: {}", e))?;

        Ok(())
    }

    /// Compute a fingerprint for a session file (mtime + size).
    pub fn compute_fingerprint(path: &std::path::Path) -> Option<String> {
        let meta = fs::metadata(path).ok()?;
        let mtime = meta.modified().ok()?
            .duration_since(std::time::UNIX_EPOCH).ok()?
            .as_secs();
        let size = meta.len();
        let mut hasher = Sha256::new();
        hasher.update(mtime.to_le_bytes());
        hasher.update(size.to_le_bytes());
        Some(hex::encode(hasher.finalize()))
    }

    /// Get cached session metadata if fingerprint matches.
    pub fn get_cached_meta(&self, id: &str, fingerprint: &str) -> Option<CachedSession> {
        let mut stmt = self.conn.prepare(
            "SELECT title, directory, timestamp, message_count, subagent_count FROM session_index WHERE id=?1 AND fingerprint=?2"
        ).ok()?;

        stmt.query_row(params![id, fingerprint], |row| {
            Ok(CachedSession {
                title: row.get(0)?,
                directory: row.get(1)?,
                timestamp: row.get(2)?,
                message_count: row.get(3)?,
                subagent_count: row.get(4)?,
            })
        }).ok()
    }

    /// Store/update session metadata with fingerprint.
    pub fn store_meta(&self, id: &str, title: Option<&str>, directory: &str,
                      timestamp: &str, message_count: i64, subagent_count: i64,
                      fingerprint: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO session_index (id, title, directory, timestamp, message_count, subagent_count, fingerprint, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, title, directory, timestamp, message_count, subagent_count, fingerprint, now],
        ).map_err(|e| format!("Store meta error: {}", e))?;
        Ok(())
    }

    /// Get cached boot payload if fingerprint matches.
    pub fn get_cached_boot(&self, session_id: &str, fingerprint: &str) -> Option<Vec<u8>> {
        self.conn.query_row(
            "SELECT payload FROM boot_cache WHERE session_id=?1 AND fingerprint=?2",
            params![session_id, fingerprint],
            |row| row.get(0),
        ).ok()
    }

    /// Store boot payload with fingerprint.
    pub fn store_boot(&self, session_id: &str, fingerprint: &str, payload: &[u8]) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO boot_cache (session_id, fingerprint, payload, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, fingerprint, payload, now],
        ).map_err(|e| format!("Store boot error: {}", e))?;
        Ok(())
    }

    /// Update FTS index for a session.
    pub fn update_fts(&self, id: &str, title: Option<&str>, directory: &str, content: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM session_search WHERE session_id = ?1",
            params![id],
        ).map_err(|e| format!("FTS delete error: {}", e))?;
        self.conn.execute(
            "INSERT INTO session_search (session_id, title, directory, content) VALUES (?1, ?2, ?3, ?4)",
            params![id, title.unwrap_or(""), directory, content],
        ).map_err(|e| format!("FTS insert error: {}", e))?;
        Ok(())
    }

    /// Search FTS index across all indexed sessions.
    /// Search FTS index across all indexed sessions.
    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<FtsResult>, String> {
        let sanitized = query.replace(|c: char| !c.is_alphanumeric() && c != ' ', " ");
        let fts_query = sanitized.split_whitespace()
            .map(|w| format!("{}*", w))
            .collect::<Vec<_>>()
            .join(" ");

        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT session_id, title, directory, snippet(session_search, 3, '<mark>', '</mark>', '...', 32)
             FROM session_search WHERE session_search MATCH ?1 LIMIT ?2"
        ).map_err(|e| format!("FTS search error: {}", e))?;

        let results = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok(FtsResult {
                session_id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                snippet: row.get(3)?,
            })
        }).map_err(|e| format!("FTS query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct CachedSession {
    pub title: Option<String>,
    pub directory: Option<String>,
    pub timestamp: String,
    pub message_count: i64,
    pub subagent_count: i64,
}

#[derive(Debug, Clone)]
pub struct FtsResult {
    pub session_id: String,
    pub title: Option<String>,
    pub directory: Option<String>,
    pub snippet: String,
}
