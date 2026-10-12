//! Versioned private SQLite inventory. JSON remains a reviewed legacy bridge.
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
const APP: i64 = 0x52555741;
const LIMIT: u64 = 16 * 1024 * 1024;
pub struct Snapshot {
    pub document: Vec<u8>,
    pub revision: i64,
    pub mirror_hash: String,
    pub pending: bool,
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn legacy_hash(path: &Path) -> Result<String, String> {
    match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(f) => {
            let m = f
                .metadata()
                .map_err(|_| "Cannot inspect legacy inventory")?;
            if !m.is_file()
                || m.uid() != unsafe { libc::geteuid() }
                || m.mode() & 0o077 != 0
                || m.nlink() != 1
                || m.len() > 1024 * 1024
            {
                return Err("Unsafe legacy inventory".into());
            }
            let mut b = Vec::new();
            f.take(1024 * 1024 + 1)
                .read_to_end(&mut b)
                .map_err(|_| "Cannot read legacy inventory")?;
            Ok(hash(&b))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(_) => Err("Cannot inspect legacy inventory".into()),
    }
}
fn check(path: &Path) -> Result<(), String> {
    let m = fs::symlink_metadata(path).map_err(|_| "Cannot inspect SQLite inventory")?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.nlink() != 1
        || m.len() > LIMIT
    {
        return Err("Unsafe or oversized SQLite inventory".into());
    }
    Ok(())
}
fn open(path: &Path) -> Result<Connection, String> {
    let parent = path.parent().ok_or("Inventory needs private parent")?;
    let m = fs::symlink_metadata(parent).map_err(|_| "Private inventory directory unavailable")?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err("Unsafe inventory directory".into());
    }
    let created = match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(_) => return Err("Cannot create private SQLite inventory".into()),
    };
    check(path)?;
    for suffix in ["-journal", "-wal", "-shm"] {
        let side = path.with_file_name(format!(
            "{}{}",
            path.file_name().unwrap().to_string_lossy(),
            suffix
        ));
        if side.symlink_metadata().is_ok() {
            check(&side)?;
        }
    }
    let c = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| "SQLite inventory cannot be opened")?;
    c.busy_timeout(std::time::Duration::from_millis(250))
        .map_err(|_| "SQLite timeout setup failed")?;
    let v: i64 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|_| "Invalid SQLite inventory")?;
    let app: i64 = c
        .pragma_query_value(None, "application_id", |r| r.get(0))
        .map_err(|_| "Invalid SQLite inventory")?;
    if created {
        c.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA trusted_schema=OFF; PRAGMA max_page_count=4096;
            BEGIN IMMEDIATE;
            CREATE TABLE library(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL CHECK(revision>=0),document BLOB NOT NULL,mirror_hash TEXT NOT NULL,pending INTEGER NOT NULL CHECK(pending IN(0,1)));
            CREATE TABLE endpoints(id TEXT PRIMARY KEY,position INTEGER UNIQUE NOT NULL,machine_id TEXT NOT NULL,name TEXT NOT NULL,group_path TEXT NOT NULL,favorite INTEGER NOT NULL,last_used TEXT NOT NULL,search_text TEXT NOT NULL,payload BLOB NOT NULL);
            CREATE INDEX endpoints_group ON endpoints(group_path,name);
            CREATE INDEX endpoints_favorite ON endpoints(favorite,last_used);
            CREATE INDEX endpoints_machine ON endpoints(machine_id);
            CREATE TABLE tags(endpoint TEXT NOT NULL,tag TEXT NOT NULL,PRIMARY KEY(endpoint,tag));
            CREATE INDEX tags_name ON tags(tag,endpoint);
            CREATE TABLE revisions(revision INTEGER PRIMARY KEY,document BLOB NOT NULL);
            PRAGMA application_id=1381324609; PRAGMA user_version=1; COMMIT;") .map_err(|_|"SQLite schema creation failed")?;
    } else if v != 1 || app != APP {
        return Err("Unknown SQLite inventory schema; existing database protected".into());
    }
    c.execute_batch(
        "PRAGMA synchronous=FULL; PRAGMA trusted_schema=OFF; PRAGMA max_page_count=4096;",
    )
    .map_err(|_| "SQLite inventory setup failed")?;
    let size: i64 = c
        .pragma_query_value(None, "page_size", |r| r.get(0))
        .map_err(|_| "SQLite page size unavailable")?;
    if !(512..=65536).contains(&size) {
        return Err("Invalid SQLite page size".into());
    }
    c.pragma_update(None, "max_page_count", (LIMIT as i64) / size)
        .map_err(|_| "SQLite capacity setup failed")?;
    Ok(c)
}
fn snapshot(c: &Connection) -> Result<Option<Snapshot>, String> {
    c.query_row(
        "SELECT document,revision,mirror_hash,pending FROM library WHERE id=1",
        [],
        |r| {
            Ok(Snapshot {
                document: r.get(0)?,
                revision: r.get(1)?,
                mirror_hash: r.get(2)?,
                pending: r.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|_| "SQLite inventory read failed".into())
}
fn replace(
    c: &mut Connection,
    document: &[u8],
    mirror_hash: &str,
    pending: bool,
    expected: i64,
) -> Result<i64, String> {
    if document.len() > 1024 * 1024 {
        return Err("Inventory exceeds document cap".into());
    }
    super::parse_library(document)?;
    let library: super::Library =
        serde_json::from_slice(document).map_err(|_| "Invalid inventory document")?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "SQLite inventory busy")?;
    let old = snapshot(&tx)?;
    let current = old.as_ref().map(|s| s.revision).unwrap_or(0);
    if current != expected {
        return Err("Machine library changed; refresh before saving".into());
    }
    if let Some(old) = old {
        tx.execute(
            "INSERT INTO revisions VALUES(?1,?2)",
            params![old.revision, old.document],
        )
        .map_err(|_| "Cannot retain inventory revision")?;
    }
    tx.execute_batch("DELETE FROM tags; DELETE FROM endpoints;")
        .map_err(|_| "Cannot replace inventory rows")?;
    for (position, p) in library.items.iter().enumerate() {
        let search =
            format!("{} {} {} {}", p.name, p.group, p.server, p.tags.join(" ")).to_lowercase();
        let payload = serde_json::to_vec(p).map_err(|_| "Invalid endpoint")?;
        tx.execute(
            "INSERT INTO endpoints VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                p.id,
                position as i64,
                p.machine_id,
                p.name,
                p.group,
                p.favorite,
                format!("{:020}", p.last_used),
                search,
                payload
            ],
        )
        .map_err(|_| "Endpoint transaction failed")?;
        for tag in &p.tags {
            tx.execute(
                "INSERT OR IGNORE INTO tags VALUES(?1,?2)",
                params![p.id, tag.to_lowercase()],
            )
            .map_err(|_| "Tag transaction failed")?;
        }
    }
    let next = current
        .checked_add(1)
        .ok_or("Inventory revision exhausted")?;
    tx.execute(
        "INSERT OR REPLACE INTO library VALUES(1,?1,?2,?3,?4)",
        params![next, document, mirror_hash, pending],
    )
    .map_err(|_| "Inventory transaction failed")?;
    tx.execute("DELETE FROM revisions WHERE revision NOT IN(SELECT revision FROM revisions ORDER BY revision DESC LIMIT 8)",[]).map_err(|_|"Inventory revision cleanup failed")?;
    tx.commit().map_err(|_| "Inventory commit failed")?;
    Ok(next)
}
pub fn load_legacy(
    path: &Path,
    legacy: &Path,
    incoming: &[u8],
    source_hash: &str,
) -> Result<Snapshot, String> {
    // Hash the exact raw bytes parsed by the caller, before normalizing them.
    // A later read must never label an earlier document with a newer hash.
    if legacy_hash(legacy)? != source_hash {
        return Err("Legacy library changed while reading; retry after refresh".into());
    }
    load(path, incoming, source_hash)
}
pub fn load(path: &Path, incoming: &[u8], observed_hash: &str) -> Result<Snapshot, String> {
    let mut c = open(path)?;
    let previous = snapshot(&c)?;
    match previous {
        None => {
            replace(&mut c, incoming, observed_hash, false, 0)?;
        }
        Some(s) if observed_hash.is_empty() && !s.mirror_hash.is_empty() => {
            c.execute(
                "UPDATE library SET mirror_hash='',pending=1 WHERE id=1 AND revision=?1",
                params![s.revision],
            )
            .map_err(|_| "Inventory bridge recovery failed")?;
        }
        Some(s) if s.mirror_hash != observed_hash => {
            if s.pending {
                if hash(&s.document) != observed_hash {
                    return Err(
                        "Legacy library changed during SQLite commit; both versions preserved"
                            .into(),
                    );
                }
                c.execute(
                    "UPDATE library SET mirror_hash=?1,pending=0 WHERE id=1 AND revision=?2",
                    params![observed_hash, s.revision],
                )
                .map_err(|_| "Inventory mirror recovery failed")?;
            } else {
                replace(&mut c, incoming, observed_hash, false, s.revision)?;
            }
        }
        _ => (),
    }
    let s = snapshot(&c)?.ok_or("SQLite inventory missing")?;
    super::parse_library(&s.document)?;
    Ok(s)
}
pub fn read(path: &Path) -> Result<Snapshot, String> {
    check(path)?;
    let c = open(path)?;
    let s = snapshot(&c)?.ok_or("SQLite inventory missing")?;
    super::parse_library(&s.document)?;
    Ok(s)
}
pub fn save(path: &Path, document: &[u8], expected: i64, mirror_hash: &str) -> Result<i64, String> {
    let mut c = open(path)?;
    replace(&mut c, document, mirror_hash, true, expected)
}
pub fn mirrored(path: &Path, revision: i64, observed: &str) -> Result<(), String> {
    let c = open(path)?;
    let n = c
        .execute(
            "UPDATE library SET mirror_hash=?1,pending=0 WHERE id=1 AND revision=?2 AND pending=1",
            params![observed, revision],
        )
        .map_err(|_| "Inventory mirror acknowledgement failed")?;
    if n != 1 {
        return Err("Concurrent inventory commit; reload required".into());
    }
    Ok(())
}
pub fn search(
    path: &Path,
    query: &str,
    mode: u32,
    expected: i64,
) -> Result<std::collections::HashSet<String>, String> {
    let c = open(path)?;
    let s = snapshot(&c)?.ok_or("Inventory missing")?;
    if s.revision != expected {
        return Err("Machine library changed; refresh required".into());
    }
    let mut stmt=c.prepare("SELECT id FROM endpoints WHERE instr(search_text,?1)>0 AND (?2!=1 OR favorite=1) AND (?2!=2 OR last_used!='00000000000000000000') ORDER BY position LIMIT 1000").map_err(|_|"Inventory search failed")?;
    stmt.query_map(params![query.to_lowercase(), mode], |r| {
        r.get::<_, String>(0)
    })
    .map_err(|_| "Inventory search failed")?
    .collect::<Result<_, _>>()
    .map_err(|_| "Inventory search result failed".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            use std::os::unix::fs::DirBuilderExt;
            let base = std::env::var_os("CARGO_TARGET_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            let p = base.join(ruwac_protocol::id("inventory_test"));
            fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
            Self(p)
        }
        fn db(&self) -> std::path::PathBuf {
            self.0.join("machines.sqlite3")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn doc(name: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({"schema":1,"items":[{"id":"endpoint_fixture","machine_id":"machine_fixture","name":name,"group":"Fixture/Group","protocol":"RDP","server":"localhost","username":"fixture","secret_reference":"reference_only","favorite":true,"last_used":u64::MAX,"tags":["Živjo"]}],"groups":[]})).unwrap()
    }
    #[test]
    fn transaction_persists_metadata_indexes_and_unicode_search() {
        let f = Fixture::new();
        let d = doc("ŽIVJO Windows");
        let s = load(&f.db(), &d, "legacy").unwrap();
        assert_eq!(s.document, d);
        assert!(
            search(&f.db(), "živjo", 1, s.revision)
                .unwrap()
                .contains("endpoint_fixture")
        );
        assert!(search(&f.db(), "%", 0, s.revision).unwrap().is_empty());
        assert!(
            search(&f.db(), "windows", 2, s.revision)
                .unwrap()
                .contains("endpoint_fixture")
        );
        let c = open(&f.db()).unwrap();
        let value: String = c
            .query_row("SELECT last_used FROM endpoints", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, u64::MAX.to_string());
        assert_eq!(
            c.query_row("SELECT count(*) FROM tags", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(c);
        assert_eq!(load(&f.db(), &d, "legacy").unwrap().document, d);
    }
    #[test]
    fn atomic_legacy_replacement_after_read_cannot_relabel_old_document() {
        let f = Fixture::new();
        let legacy = f.0.join("machines.json");
        let old = doc("Read before replacement");
        let newer = doc("Other window replacement");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&legacy)
            .unwrap();
        fs::write(&legacy, &old).unwrap();
        let snapshot = load_legacy(&f.db(), &legacy, &old, &hash(&old)).unwrap();
        let candidate = f.0.join("candidate.json");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&candidate)
            .unwrap();
        fs::write(&candidate, &newer).unwrap();
        fs::rename(candidate, &legacy).unwrap();
        assert!(load_legacy(&f.db(), &legacy, &old, &hash(&old)).is_err());
        let retained = read(&f.db()).unwrap();
        assert_eq!(retained.document, old);
        assert_eq!(retained.revision, snapshot.revision);
        assert_eq!(fs::read(&legacy).unwrap(), newer);
        assert_eq!(
            load_legacy(&f.db(), &legacy, &newer, &hash(&newer))
                .unwrap()
                .document,
            newer
        );
    }
    #[test]
    fn stale_writer_and_sql_failure_leave_committed_rows_unchanged() {
        let f = Fixture::new();
        let d = doc("Old");
        let s = load(&f.db(), &d, "legacy").unwrap();
        let next = save(&f.db(), &doc("New"), s.revision, "legacy").unwrap();
        assert!(save(&f.db(), &doc("Stale"), s.revision, "legacy").is_err());
        let c = open(&f.db()).unwrap();
        c.execute_batch("CREATE TRIGGER reject_endpoint BEFORE INSERT ON endpoints BEGIN SELECT RAISE(ABORT,'fixture rejection'); END;").unwrap();
        drop(c);
        assert!(save(&f.db(), &doc("Rejected"), next, "legacy").is_err());
        let c = open(&f.db()).unwrap();
        let after = snapshot(&c).unwrap().unwrap();
        assert_eq!(after.revision, next);
        assert_eq!(after.document, doc("New"));
        assert_eq!(
            c.query_row("SELECT name FROM endpoints", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "New"
        );
    }
    #[test]
    fn incomplete_publication_recovers_and_preserves_external_conflict() {
        let f = Fixture::new();
        let old = doc("Old");
        let s = load(&f.db(), &old, "old_hash").unwrap();
        let new = doc("New");
        save(&f.db(), &new, s.revision, "old_hash").unwrap();
        let after = load(&f.db(), &old, "old_hash").unwrap();
        assert!(after.pending);
        assert_eq!(after.document, new);
        assert!(load(&f.db(), &doc("External"), "external_hash").is_err());
        let committed = load(&f.db(), &new, &hash(&new)).unwrap();
        assert!(!committed.pending);
        assert_eq!(committed.document, new);
    }
    #[test]
    fn missing_bridge_keeps_sqlite_canonical_data() {
        let f = Fixture::new();
        let d = doc("Keep");
        load(&f.db(), &d, "old_hash").unwrap();
        let missing = load(&f.db(), &doc("Wrong empty bridge"), "").unwrap();
        assert!(missing.pending);
        assert_eq!(missing.document, d);
    }
    #[test]
    fn unknown_database_and_symlink_are_preserved() {
        let f = Fixture::new();
        let p = f.db();
        let c = Connection::open(&p).unwrap();
        c.execute_batch("CREATE TABLE foreign_data(value); INSERT INTO foreign_data VALUES(42);")
            .unwrap();
        drop(c);
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        let before = fs::read(&p).unwrap();
        assert!(load(&p, &doc("New"), "").is_err());
        assert_eq!(fs::read(&p).unwrap(), before);
        let link = f.0.join("link.sqlite3");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        assert!(load(&link, &doc("New"), "").is_err());
        assert_eq!(fs::read(&p).unwrap(), before);
    }
}
