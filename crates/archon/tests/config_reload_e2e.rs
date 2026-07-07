//! #529 step 9 — end-to-end SIGHUP reload regression.
//!
//! Spawns the REAL `harmonia serve` binary, drives it over its actual HTTP
//! API, rewrites its TOML on disk, sends a real `SIGHUP`, and observes the
//! three reload classes through the real signal path: LIVE (`opds_page_size`
//! visible on the next request), auth-LIVE (`jwt_secret` rotation
//! invalidates the outstanding bearer immediately, refresh recovers), and
//! RESTART (`database.db_path` held back + reported in `restart_pending`).

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;

// WHY: the same gitleaks-allowlisted fixture secret used across the
// exousia/paroche/archon test suites (`.gitleaks.toml` allowlist regex).
const JWT_SECRET_INITIAL: &str = "test-secret-that-is-long-enough-for-hs256";

const STARTUP_DEADLINE: Duration = Duration::from_secs(20);
const RELOAD_DEADLINE: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Kills and reaps the spawned `harmonia` process on drop so a test panic
/// (or an early return) never leaks a running server.
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        // WHY: best-effort teardown — the process may have already exited
        // (e.g. it crashed during the test); failure to kill/reap here is
        // non-fatal to the test outcome and the OS reclaims an orphan.
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener
        .local_addr()
        .expect("listener has a local addr")
        .port()
}

/// The parts of `harmonia.toml` that stay fixed across the pre- and
/// post-reload rewrite (config file location, download dir, ports).
struct HarmoniaFixture {
    config_path: std::path::PathBuf,
    download_dir: std::path::PathBuf,
    http_port: u16,
    quic_port: u16,
}

impl HarmoniaFixture {
    /// Writes `harmonia.toml` with the given (variable, reload-relevant)
    /// field values. Called twice: once before spawning the child, once
    /// after — to construct the config-reload delta the test drives via
    /// SIGHUP.
    fn write_toml(&self, db_path: &std::path::Path, opds_page_size: u32, jwt_secret: &str) {
        let download_dir = self.download_dir.display();
        let toml = format!(
            r#"[database]
db_path = "{db_path}"

[exousia]
jwt_secret = "{jwt_secret}"

[paroche]
listen_addr = "127.0.0.1"
port = {http_port}
renderer_quic_port = {quic_port}
opds_page_size = {opds_page_size}

[ergasia]
download_dir = "{download_dir}"
session_state_path = "{download_dir}/.librqbit-state"
"#,
            db_path = db_path.display(),
            http_port = self.http_port,
            quic_port = self.quic_port,
        );
        std::fs::write(&self.config_path, toml).expect("write harmonia.toml");
    }
}

/// Spawns the real `harmonia serve` binary against `config_path`, capturing
/// stdout on a background thread so the child never blocks on a full pipe
/// buffer. Returns the guard (kills the child on drop) and a channel that
/// yields every stdout line as it is written.
fn spawn_harmonia(config_path: &std::path::Path) -> (ChildGuard, mpsc::Receiver<String>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_harmonia"))
        .arg("serve")
        .arg("-c")
        .arg(config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn harmonia serve");

    let stdout = child.stdout.take().expect("child stdout is piped");
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            // WHY: a send error means the test thread stopped receiving
            // (test already finished) — nothing left to do but stop reading.
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    (ChildGuard(child), rx)
}

/// Blocks (bounded) until the first-run admin-password banner appears on the
/// child's stdout, returning the parsed password. No fixed sleep: this waits
/// on the actual stdout stream via the channel, bounded by `STARTUP_DEADLINE`.
fn wait_for_admin_password(rx: &mpsc::Receiver<String>) -> String {
    let deadline = Instant::now() + STARTUP_DEADLINE;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            remaining > Duration::ZERO,
            "admin password banner not observed within {STARTUP_DEADLINE:?}"
        );
        let line = rx
            .recv_timeout(remaining)
            .expect("child stdout closed before the admin-password banner appeared");
        if let Some(password) = line.strip_prefix("  First run detected. Admin password: ") {
            return password.trim().to_string();
        }
    }
}

async fn try_login(
    client: &reqwest::Client,
    base: &str,
    password: &str,
) -> Option<(String, String)> {
    let resp = client
        .post(format!("{base}/api/auth/login"))
        .json(&serde_json::json!({"username": "admin", "password": password}))
        .send()
        .await
        .ok()?;
    if resp.status() != reqwest::StatusCode::OK {
        return None;
    }
    let body: Value = resp.json().await.ok()?;
    Some((
        body["data"]["access_token"].as_str()?.to_string(),
        body["data"]["refresh_token"].as_str()?.to_string(),
    ))
}

/// Bounded-poll login: the server may not be accepting connections yet even
/// after the admin-password banner (the HTTP listener binds slightly later
/// in startup), so retry the login call itself until it succeeds.
async fn wait_for_server_ready(
    client: &reqwest::Client,
    base: &str,
    password: &str,
) -> (String, String) {
    let deadline = Instant::now() + STARTUP_DEADLINE;
    loop {
        if let Some(pair) = try_login(client, base, password).await {
            return pair;
        }
        assert!(
            Instant::now() < deadline,
            "server never became ready to accept logins within {STARTUP_DEADLINE:?}"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn get_config(client: &reqwest::Client, base: &str, token: &str) -> reqwest::Response {
    client
        .get(format!("{base}/api/system/config"))
        .bearer_auth(token)
        .send()
        .await
        .expect("config request sends")
}

/// Bounded-poll: waits until `GET /api/system/config` (using `token`)
/// returns 401 — the observable signal that a JWT-secret rotation has
/// invalidated this specific outstanding bearer.
async fn wait_for_token_invalidated(client: &reqwest::Client, base: &str, token: &str) {
    let deadline = Instant::now() + RELOAD_DEADLINE;
    loop {
        let resp = get_config(client, base, token).await;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "pre-rotation bearer was never invalidated within {RELOAD_DEADLINE:?}"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Bounded-poll: waits until `GET /api/system/config` (using `token`, which
/// must be a POST-rotation bearer) reports `opds_page_size == expected`,
/// then returns the full JSON payload for further assertions.
async fn wait_for_opds_page_size(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    expected: u32,
) -> Value {
    let deadline = Instant::now() + RELOAD_DEADLINE;
    loop {
        let resp = get_config(client, base, token).await;
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "post-rotation bearer must authenticate"
        );
        let body: Value = resp.json().await.expect("config response is JSON");
        if body["paroche"]["opds_page_size"].as_u64() == Some(u64::from(expected)) {
            return body;
        }
        assert!(
            Instant::now() < deadline,
            "opds_page_size never reached {expected} within {RELOAD_DEADLINE:?} (last seen: {body})"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sighup_reload_applies_live_rotates_jwt_and_holds_back_restart_class() {
    let workdir = tempfile::tempdir().expect("create tempdir");
    let download_dir = workdir.path().join("downloads");
    std::fs::create_dir_all(&download_dir).expect("create download dir");

    let db_path_initial = workdir.path().join("harmonia.db");

    let http_port = pick_free_port();
    let mut quic_port = pick_free_port();
    // WHY: two independent ephemeral-port picks could theoretically collide;
    // validation requires them to differ, so re-pick rather than flake.
    while quic_port == http_port {
        quic_port = pick_free_port();
    }

    let fixture = HarmoniaFixture {
        config_path: workdir.path().join("harmonia.toml"),
        download_dir,
        http_port,
        quic_port,
    };
    fixture.write_toml(&db_path_initial, 50, JWT_SECRET_INITIAL);

    let (guard, stdout_rx) = spawn_harmonia(&fixture.config_path);
    let admin_password = wait_for_admin_password(&stdout_rx);

    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{http_port}");

    let (access_pre, refresh_pre) = wait_for_server_ready(&client, &base, &admin_password).await;

    // Sanity: the pre-rotation bearer authenticates and sees the pre-reload
    // page size before any config change.
    let initial = get_config(&client, &base, &access_pre).await;
    assert_eq!(initial.status(), reqwest::StatusCode::OK);
    let initial_body: Value = initial.json().await.expect("config response is JSON");
    assert_eq!(initial_body["paroche"]["opds_page_size"], 50);
    assert_eq!(
        initial_body["restart_pending"]
            .as_array()
            .expect("restart_pending is an array")
            .len(),
        0
    );

    // Rewrite the TOML: one LIVE field (opds_page_size), one auth-LIVE field
    // (jwt_secret rotation — derived from the allowlisted constant at
    // runtime rather than a second hardcoded secret literal, per #529 step 9),
    // and one RESTART-class field (database.db_path).
    let jwt_secret_rotated = format!("{JWT_SECRET_INITIAL}-rotated");
    let db_path_rotated = workdir.path().join("harmonia-rotated.db");
    fixture.write_toml(&db_path_rotated, 999, &jwt_secret_rotated);

    let pid = guard.0.id();
    tokio::task::spawn_blocking(move || {
        Command::new("/bin/kill")
            .arg("-HUP")
            .arg(pid.to_string())
            .status()
    })
    .await
    .expect("spawn_blocking joins")
    .expect("kill -HUP runs");

    // 1. The pre-rotation bearer must be rejected as soon as the reload
    //    lands (immediate JWT invalidation, not the next natural expiry).
    wait_for_token_invalidated(&client, &base, &access_pre).await;

    // 2. Refresh recovers a working token pair without re-login.
    let refresh_resp = client
        .post(format!("{base}/api/auth/refresh"))
        .json(&serde_json::json!({"refresh_token": refresh_pre}))
        .send()
        .await
        .expect("refresh request sends");
    assert_eq!(
        refresh_resp.status(),
        reqwest::StatusCode::OK,
        "refresh must recover a working session across a jwt_secret rotation"
    );
    let refresh_body: Value = refresh_resp.json().await.expect("refresh response is JSON");
    let access_post = refresh_body["data"]["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();

    // 3. The LIVE field is visible via the new bearer, and the RESTART-class
    //    field is reported (held back, not silently dropped).
    let reloaded = wait_for_opds_page_size(&client, &base, &access_post, 999).await;
    let restart_pending = reloaded["restart_pending"]
        .as_array()
        .expect("restart_pending is an array");
    assert!(
        restart_pending
            .iter()
            .any(|v| v.as_str() == Some("database.db_path")),
        "restart_pending must list database.db_path, got: {restart_pending:?}"
    );

    drop(guard);
}
