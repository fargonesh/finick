use std::{sync::mpsc::Sender, thread};

use config::ty::App;
use index::ty::{Request, SearchResult};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let manager = SqliteConnectionManager::file(config::finick_root().join("index.db"));
    let pool = Pool::new(manager).unwrap();
    pool.get().unwrap().execute_batch("PRAGMA journal_mode = WAL;").unwrap();

    pool.get()
        .unwrap()
        .execute(
            "CREATE TABLE IF NOT EXISTS files (
        name TEXT NOT NULL,
        path TEXT PRIMARY KEY,
        parent_path TEXT,
        depth INTEGER NOT NULL,
        executable BOOL NOT NULL,
        desktop BOOL NOT NULL,
        icon TEXT,
        is_dir BOOL NOT NULL DEFAULT 0,
        size INTEGER,
        last_accessed INTEGER NOT NULL
    )",
            params![],
        )
        .unwrap();

    let _ = pool.get().unwrap().execute("CREATE INDEX IF NOT EXISTS idx_parent_path ON files (parent_path);", params![]);

    thread::spawn({
        let pool = pool.clone();
        move || index::watch(pool.clone())
    });
    thread::spawn({
        let pool = pool.clone();
        move || index::index(None, pool.clone())
    });
    ipsea::start_server(App::IndexService.to_string(), {
        let pool = pool.clone();
        move |req: Request, sender: Sender<SearchResult>| match req {
            Request::Search { query } => {
                println!("Searching for {}", query);
                index::search(&query, pool.clone(), |v| {
                    let _ = sender.send(v);
                });
            }
            Request::ListDir { path } => {
                println!("Listing dir {}", path);
                index::list_dir(&path, pool.clone(), |v| {
                    let _ = sender.send(v);
                });
            }
        }
    })
    .expect("Failed to start index service");
}
