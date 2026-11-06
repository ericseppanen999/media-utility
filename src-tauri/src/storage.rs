//! JSON record store backed by SQLite, owned by a single worker thread.

use crate::domain::*;
use rusqlite::{params, Connection};
use std::path::Path;
use tokio::sync::{mpsc, oneshot};

const SCHEMA: &str = "
    PRAGMA journal_mode=WAL;
    PRAGMA foreign_keys=ON;
    CREATE TABLE IF NOT EXISTS records(
        kind TEXT NOT NULL,
        id TEXT NOT NULL,
        value TEXT NOT NULL,
        PRIMARY KEY(kind,id)
    );
    PRAGMA user_version=1;
";

const UPSERT_RECORD: &str = "
    INSERT INTO records(kind,id,value) VALUES(?1,?2,?3)
    ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value
";

const SELECT_RECORD: &str = "SELECT value FROM records WHERE kind=?1 AND id=?2";
const SELECT_RECORDS: &str = "SELECT value FROM records WHERE kind=?1 ORDER BY rowid DESC";
const SELECT_JOBS: &str = "SELECT id,value FROM records WHERE kind='job'";
const UPDATE_JOB: &str = "UPDATE records SET value=?1 WHERE kind='job' AND id=?2";

type Reply = oneshot::Sender<Result<String>>;

enum Request {
    Put {
        kind: String,
        id: String,
        value: String,
        reply: Reply,
    },
    Get {
        kind: String,
        id: String,
        reply: Reply,
    },
    List {
        kind: String,
        reply: Reply,
    },
}

#[derive(Clone)]
pub struct Storage {
    tx: mpsc::Sender<Request>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        // Recover durable state before starting the scheduler.
        interrupt_unfinished_jobs(&mut conn)?;
        let (sender, receiver) = mpsc::channel::<Request>(128);
        std::thread::spawn(move || serve_requests(conn, receiver));
        Ok(Self { tx: sender })
    }

    pub async fn put<T: serde::Serialize>(&self, kind: &str, id: &str, value: &T) -> Result<()> {
        let value = serde_json::to_string(value)?;
        self.request(|reply| Request::Put {
            kind: kind.into(),
            id: id.into(),
            value,
            reply,
        })
        .await?;
        Ok(())
    }

    pub async fn get<T: serde::de::DeserializeOwned>(&self, kind: &str, id: &str) -> Result<T> {
        let json = self
            .request(|reply| Request::Get {
                kind: kind.into(),
                id: id.into(),
                reply,
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    pub async fn list<T: serde::de::DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        let json = self
            .request(|reply| Request::List {
                kind: kind.into(),
                reply,
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    async fn request(&self, build: impl FnOnce(Reply) -> Request) -> Result<String> {
        let (reply, response) = oneshot::channel();
        self.tx
            .send(build(reply))
            .await
            .map_err(|_| database_closed())?;
        response.await.map_err(|_| database_closed())?
    }
}

fn database_closed() -> AppError {
    AppError::new("DATABASE", "Database closed")
}

/// Marks jobs that were in flight when the app last stopped as interrupted.
fn interrupt_unfinished_jobs(conn: &mut Connection) -> Result<()> {
    let jobs: Vec<(String, String)> = {
        let mut stmt = conn.prepare(SELECT_JOBS)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    let tx = conn.transaction()?;
    for (id, json) in jobs {
        let mut job: Job = serde_json::from_str(&json)?;
        if ["queued", "running", "cancelling"].contains(&job.state.as_str()) {
            job.state = "interrupted".into();
            job.error = Some(AppError::new(
                "INTERRUPTED",
                "The app stopped before this job completed. Retry to continue.",
            ));
            tx.execute(UPDATE_JOB, params![serde_json::to_string(&job)?, id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn serve_requests(mut conn: Connection, mut receiver: mpsc::Receiver<Request>) {
    while let Some(request) = receiver.blocking_recv() {
        match request {
            Request::Put {
                kind,
                id,
                value,
                reply,
            } => {
                let _ = reply.send(put_record(&mut conn, &kind, &id, &value));
            }
            Request::Get { kind, id, reply } => {
                let _ = reply.send(get_record(&conn, &kind, &id));
            }
            Request::List { kind, reply } => {
                let _ = reply.send(list_records(&conn, &kind));
            }
        }
    }
}

fn put_record(conn: &mut Connection, kind: &str, id: &str, value: &str) -> Result<String> {
    let tx = conn.transaction()?;
    tx.execute(UPSERT_RECORD, params![kind, id, value])?;
    tx.commit()?;
    Ok(String::new())
}

fn get_record(conn: &Connection, kind: &str, id: &str) -> Result<String> {
    conn.query_row(SELECT_RECORD, params![kind, id], |r| r.get(0))
        .map_err(AppError::from)
}

/// Returns the records of one kind as a JSON array, newest first.
fn list_records(conn: &Connection, kind: &str) -> Result<String> {
    let mut stmt = conn.prepare(SELECT_RECORDS)?;
    let rows = stmt.query_map([kind], |r| r.get::<_, String>(0))?;
    let values = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", values.join(",")))
}
