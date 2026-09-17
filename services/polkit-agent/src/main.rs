use {
    log::{error, info, warn},
    std::{collections::HashMap, sync::Arc},
    tokio::sync::Mutex,
    zbus::interface,
};

#[derive(Debug, Clone)]
struct PendingAuth {
    action_id: String,
    message: String,
    cookie: String,
}

struct Agent {
    pending: Arc<Mutex<HashMap<String, PendingAuth>>>,
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl Agent {
    async fn begin_authentication(
        &self,
        action_id: String,
        message: String,
        _icon_name: String,
        _details: HashMap<String, String>,
        cookie: String,
        identities: Vec<(String, HashMap<String, zbus::zvariant::Value<'_>>)>,
    ) {
        info!("polkit BeginAuthentication action={} cookie={} identities={:?}", action_id, cookie, identities.len());
        let pending = PendingAuth { action_id: action_id.clone(), message: message.clone(), cookie: cookie.clone() };
        self.pending.lock().await.insert(cookie.clone(), pending);
        let pending_map = self.pending.clone();
        tokio::spawn(async move {
            let prompt = if message.is_empty() { format!("Authenticate for {}", action_id) } else { message };
            let req = ipc::modals::ModalRequest::PamAuth { prompt: prompt.clone() };
            let resp =
                tokio::task::spawn_blocking(move || ipc::modals::send_modal_request(req)).await.ok().and_then(|r| r.ok());
            let password = match resp {
                Some(ipc::modals::ModalResponse::Success { data: Some(p) }) => p,
                Some(ipc::modals::ModalResponse::Canceled) => {
                    warn!("polkit auth canceled for {}", cookie);
                    cancel_auth(&cookie).await;
                    pending_map.lock().await.remove(&cookie);
                    return;
                }
                _ => {
                    warn!("polkit modal failed for {}", cookie);
                    pending_map.lock().await.remove(&cookie);
                    return;
                }
            };
            if let Err(e) = complete_auth(&cookie, &password).await {
                error!("polkit complete failed: {}", e);
            }
            pending_map.lock().await.remove(&cookie);
        });
    }

    async fn cancel_authentication(&self, cookie: String) {
        info!("polkit CancelAuthentication {}", cookie);
        self.pending.lock().await.remove(&cookie);
    }
}

async fn complete_auth(cookie: &str, password: &str) -> zbus::Result<()> {
    let conn = zbus::Connection::session().await?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )
    .await?;
    let mut identities: HashMap<String, zbus::zvariant::Value> = HashMap::new();
    identities.insert("password".to_string(), zbus::zvariant::Value::new(password.to_string()));
    let subject = subject_for_self();
    let _: () = proxy.call("AuthenticationAgentResponse", &(cookie, &subject, &identities)).await?;
    info!("polkit auth response sent for {}", cookie);
    Ok(())
}

async fn cancel_auth(cookie: &str) {
    let Ok(conn) = zbus::Connection::session().await else { return };
    let Ok(proxy) = zbus::Proxy::new(
        &conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )
    .await
    else {
        return;
    };
    let subject = subject_for_self();
    let _: Result<(), _> = proxy
        .call("AuthenticationAgentResponse", &(cookie, &subject, &HashMap::<String, zbus::zvariant::Value>::new()))
        .await;
}

fn subject_for_self() -> HashMap<String, zbus::zvariant::Value<'static>> {
    let pid = std::process::id();
    let start_time = proc_start_time().unwrap_or(0);
    let mut m = HashMap::new();
    m.insert(
        "unix-process".to_string(),
        zbus::zvariant::Value::new(HashMap::<String, zbus::zvariant::Value>::from([
            ("pid".to_string(), zbus::zvariant::Value::new(pid)),
            ("start-time".to_string(), zbus::zvariant::Value::new(start_time)),
        ])),
    );
    m
}

fn proc_start_time() -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", std::process::id())).ok()?;
    let fields: Vec<&str> = stat.rsplit(')').next().unwrap_or(&stat).split_whitespace().collect();
    let idx = 20;
    if fields.len() > idx {
        fields[idx].parse::<u64>().ok()
    } else {
        None
    }
}

async fn register_agent(conn: &zbus::Connection, object_path: &str) -> zbus::Result<()> {
    let proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )
    .await?;
    let subject = subject_for_self();
    let locale = std::env::var("LANG").unwrap_or_else(|_| "C".to_string());
    let _: () = proxy.call("RegisterAuthenticationAgent", &(subject, locale.as_str(), object_path)).await?;
    info!("polkit agent registered at {}", object_path);
    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let pending: Arc<Mutex<HashMap<String, PendingAuth>>> = Arc::new(Mutex::new(HashMap::new()));
    let agent = Agent { pending: pending.clone() };
    let object_path = "/org/freedesktop/PolicyKit1/AuthenticationAgent";
    loop {
        match try_run_agent(agent_pending_clone(&pending), object_path).await {
            Ok(_) => break,
            Err(e) => {
                error!("polkit agent error: {} — retrying in 2s", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

fn agent_pending_clone(pending: &Arc<Mutex<HashMap<String, PendingAuth>>>) -> Arc<Mutex<HashMap<String, PendingAuth>>> {
    pending.clone()
}

async fn try_run_agent(pending: Arc<Mutex<HashMap<String, PendingAuth>>>, object_path: &str) -> zbus::Result<()> {
    let conn = zbus::Connection::session().await?;
    conn.object_server().at(object_path, Agent { pending }).await?;
    conn.request_name("org.freedesktop.PolicyKit1.AuthenticationAgent").await?;
    info!("polkit agent object exported, attempting registration");
    if let Err(e) = register_agent(&conn, object_path).await {
        warn!("register failed (polkit may not be running): {}", e);
    }
    std::future::pending::<()>().await;
    Ok(())
}
