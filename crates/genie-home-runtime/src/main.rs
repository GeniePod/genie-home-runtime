use anyhow::Result;
use genie_home_core::{
    AuditEntry, Entity, RuntimeRequest, RuntimeResponse, demo_runtime, demo_turn_on_kitchen_command,
};
use rusqlite::{Connection, params, types::Type};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_SOCKET_PATH: &str = "/tmp/genie-home-runtime.sock";
const DEFAULT_AUDIT_LOG_PATH: &str = "/tmp/genie-home-runtime-audit.jsonl";
const DEFAULT_STATE_DB_PATH: &str = "/tmp/genie-home-runtime-state.sqlite3";

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str).unwrap_or("status") {
        "status" => print_status()?,
        "demo" => run_demo()?,
        "entities" => list_entities()?,
        "evaluate" => handle_json_request(false)?,
        "execute" => handle_json_request(true)?,
        "serve" => serve(
            args.get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_SOCKET_PATH),
            args.get(3)
                .map(String::as_str)
                .unwrap_or(DEFAULT_AUDIT_LOG_PATH),
            args.get(4)
                .map(String::as_str)
                .unwrap_or(DEFAULT_STATE_DB_PATH),
        )?,
        "request" => request(
            args.get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_SOCKET_PATH),
        )?,
        "help" | "--help" | "-h" => print_help(),
        other => {
            anyhow::bail!("unknown command: {other}");
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "\
genie-home-runtime

USAGE:
    genie-home-runtime <COMMAND>
    genie-home-runtime serve [SOCKET] [AUDIT_LOG] [STATE_DB]
    genie-home-runtime request [SOCKET]

COMMANDS:
    status    Print demo runtime status
    demo      Run an in-memory safety/action demo
    entities  Print demo entity graph
    evaluate  Read a HomeCommand JSON from stdin and evaluate without executing
    execute   Read a HomeCommand JSON from stdin and execute if allowed
    serve     Serve RuntimeRequest JSON over a Unix socket
    request   Send RuntimeRequest JSON from stdin to a Unix socket
    help      Show this help"
    );
}

fn print_status() -> Result<()> {
    let runtime = demo_runtime();
    println!("{}", serde_json::to_string_pretty(&runtime.status())?);
    Ok(())
}

fn run_demo() -> Result<()> {
    let mut runtime = demo_runtime();
    let decision = runtime.execute(demo_turn_on_kitchen_command());

    println!("{}", serde_json::to_string_pretty(&decision)?);
    Ok(())
}

fn list_entities() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ListEntities);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn handle_json_request(execute: bool) -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let command = serde_json::from_str(&input)?;
    let request = if execute {
        RuntimeRequest::Execute { command }
    } else {
        RuntimeRequest::Evaluate { command }
    };
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(request);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

#[cfg(unix)]
fn serve(socket_path: &str, audit_log_path: &str, state_db_path: &str) -> Result<()> {
    use std::os::unix::net::UnixListener;

    let path = Path::new(socket_path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(path)?;
    let mut runtime = demo_runtime();
    let mut state_store = SqliteStateStore::open(state_db_path)?;
    let restored_entities = state_store.load_entities()?;
    if restored_entities.is_empty() {
        state_store.save_entities(runtime.graph().entities())?;
    } else {
        for entity in restored_entities {
            runtime.upsert_entity(entity);
        }
    }
    let restored = load_audit_entries(audit_log_path)?;
    runtime.restore_audit_entries(restored);
    eprintln!("genie-home-runtime listening on {}", path.display());
    eprintln!("audit log: {}", audit_log_path);
    eprintln!("state db: {}", state_db_path);
    eprintln!("entity count: {}", runtime.graph().len());
    eprintln!("restored audit entries: {}", runtime.audit_len());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut input = String::new();
                stream.read_to_string(&mut input)?;
                let audit_start = runtime.audit_len();
                let response = handle_runtime_request(&mut runtime, &input);
                if matches!(&response, RuntimeResponse::Command { result } if result.executed) {
                    state_store.save_entities(runtime.graph().entities())?;
                }
                let output = serialize_runtime_response(&response);
                append_new_audit_entries(audit_log_path, runtime.audit_since(audit_start))?;
                stream.write_all(output.as_bytes())?;
                stream.write_all(b"\n")?;
            }
            Err(err) => {
                eprintln!("connection error: {err}");
            }
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn serve(_socket_path: &str, _audit_log_path: &str, _state_db_path: &str) -> Result<()> {
    anyhow::bail!("Unix socket runtime API is only supported on Unix targets")
}

#[cfg(unix)]
fn request(socket_path: &str) -> Result<()> {
    use std::os::unix::net::UnixStream;

    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(input.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut output = String::new();
    stream.read_to_string(&mut output)?;
    print!("{output}");
    Ok(())
}

#[cfg(not(unix))]
fn request(_socket_path: &str) -> Result<()> {
    anyhow::bail!("Unix socket runtime API is only supported on Unix targets")
}

fn append_new_audit_entries(path: &str, entries: &[AuditEntry]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let path = Path::new(path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn load_audit_entries(path: &str) -> Result<Vec<AuditEntry>> {
    let path = Path::new(path);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry = serde_json::from_str(line)
            .map_err(|err| anyhow::anyhow!("invalid audit log line {}: {err}", index + 1))?;
        entries.push(entry);
    }
    Ok(entries)
}

fn handle_runtime_request(
    runtime: &mut genie_home_core::HomeRuntime,
    input: &str,
) -> RuntimeResponse {
    match serde_json::from_str::<RuntimeRequest>(input) {
        Ok(request) => runtime.handle_request(request),
        Err(err) => RuntimeResponse::Error {
            error: format!("invalid runtime request: {err}"),
        },
    }
}

fn serialize_runtime_response(response: &RuntimeResponse) -> String {
    serde_json::to_string(response).unwrap_or_else(|err| {
        serde_json::json!({
            "type": "error",
            "error": format!("failed to serialize runtime response: {err}")
        })
        .to_string()
    })
}

struct SqliteStateStore {
    conn: Connection,
}

impl SqliteStateStore {
    fn open(path: &str) -> Result<Self> {
        let path = Path::new(path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "\
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY NOT NULL,
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(Self { conn })
    }

    fn load_entities(&self) -> Result<Vec<Entity>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM entities ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            serde_json::from_str(&json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    fn save_entities<'a>(&mut self, entities: impl IntoIterator<Item = &'a Entity>) -> Result<()> {
        let updated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let tx = self.conn.transaction()?;
        for entity in entities {
            let json = serde_json::to_string(entity)?;
            tx.execute(
                "\
                INSERT INTO entities (id, json, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    json = excluded.json,
                    updated_at = excluded.updated_at
                ",
                params![entity.id.as_str(), json, updated_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genie_home_core::EntityState;

    fn temp_db_path(name: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "genie-home-runtime-{name}-{}-{}.sqlite3",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn sqlite_state_store_persists_entities() {
        let path = temp_db_path("entities");
        let mut store = SqliteStateStore::open(&path).unwrap();
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());

        store.save_entities(runtime.graph().entities()).unwrap();
        let entities = store.load_entities().unwrap();
        let kitchen = entities
            .iter()
            .find(|entity| entity.id.as_str() == "light.kitchen")
            .unwrap();

        assert_eq!(kitchen.state, EntityState::On);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn runtime_request_handler_keeps_error_shape() {
        let mut runtime = demo_runtime();
        let response = handle_runtime_request(&mut runtime, "{not json");

        let RuntimeResponse::Error { error } = response else {
            panic!("expected error response");
        };
        assert!(error.contains("invalid runtime request"));
    }
}
