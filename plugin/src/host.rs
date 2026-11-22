//! Native messaging host, started by the browser when the extension connects.
//!
//! Reads tab lists pushed by the extension over stdin (Chromium native
//! messaging: 4-byte native-endian length prefix + JSON) and serves the most
//! recent list to the launcher plugin over a Unix socket. Activation requests
//! from the plugin are forwarded to the extension over stdout.
//!
//! Stdout is reserved for messages to the browser, so diagnostics go to stderr
//! (Chromium forwards it to its own stderr).

use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::tabs::{icon_dir, icon_path, is_valid_icon_id, socket_dir, SocketRequest, Tab};

const REQUEST_TIMEOUT: Duration = Duration::from_millis(500);

/// Messages sent by the extension.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Message {
    Tabs { tabs: Vec<Tab> },
    /// A favicon as base64 PNG, sent once before the first tab that uses it.
    Icon { id: String, data: String },
}

/// Messages sent to the extension.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Command {
    /// Sent once at startup so the extension knows the host is installed and running.
    Hello { version: &'static str },
    #[serde(rename_all = "camelCase")]
    Activate { tab_id: i64, window_id: i64 },
}

/// Serialized JSON of the latest tab list, ready to send to the plugin.
type Snapshot = Arc<Mutex<Arc<Vec<u8>>>>;

#[tokio::main(flavor = "current_thread")]
pub async fn run() -> io::Result<()> {
    let dir = socket_dir();
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    std::fs::create_dir_all(icon_dir())?;
    let path = dir.join(format!("{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;

    send_to_browser(&Command::Hello {
        version: env!("CARGO_PKG_VERSION"),
    })?;

    let snapshot: Snapshot = Arc::new(Mutex::new(Arc::new(b"[]".to_vec())));

    // Native messaging stdin is blocking and framed; a plain thread is simplest.
    let (eof_tx, eof_rx) = oneshot::channel();
    {
        let snapshot = snapshot.clone();
        std::thread::spawn(move || {
            if let Err(why) = read_messages(&snapshot) {
                eprintln!("browser-tabs host: {why}");
            }
            let _ = eof_tx.send(());
        });
    }

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let serve = async {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            let snapshot = snapshot.clone();
            tokio::spawn(async move {
                if let Err(why) = handle_client(stream, &snapshot).await {
                    eprintln!("browser-tabs host: client: {why}");
                }
            });
        }
    };

    // The browser closes stdin when the extension disconnects or the browser exits.
    tokio::select! {
        _ = serve => {}
        _ = eof_rx => {}
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }

    let _ = std::fs::remove_file(&path);
    Ok(())
}

async fn handle_client(stream: UnixStream, snapshot: &Snapshot) -> io::Result<()> {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    timeout(REQUEST_TIMEOUT, stream.read_line(&mut line))
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;

    match serde_json::from_str(&line)? {
        SocketRequest::List => {
            let data = snapshot.lock().unwrap().clone();
            stream.write_all(&data).await?;
        }
        SocketRequest::Activate { tab_id, window_id } => {
            send_to_browser(&Command::Activate { tab_id, window_id })?;
        }
    }
    stream.shutdown().await
}

/// Write one native messaging frame to the browser.
fn send_to_browser(command: &Command) -> io::Result<()> {
    let body = serde_json::to_vec(command)?;
    let mut frame = (body.len() as u32).to_ne_bytes().to_vec();
    frame.extend_from_slice(&body);
    // One write under the stdout lock keeps concurrent frames from interleaving.
    let mut out = io::stdout().lock();
    out.write_all(&frame)?;
    out.flush()
}

fn read_messages(snapshot: &Snapshot) -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    loop {
        let mut len = [0u8; 4];
        match stdin.read_exact(&mut len) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        }
        let mut body = vec![0u8; u32::from_ne_bytes(len) as usize];
        stdin.read_exact(&mut body)?;

        match serde_json::from_slice::<Message>(&body) {
            Ok(Message::Tabs { tabs }) => {
                let data = serde_json::to_vec(&tabs).expect("tabs serialize");
                *snapshot.lock().unwrap() = Arc::new(data);
            }
            Ok(Message::Icon { id, data }) => {
                if let Err(why) = save_icon(&id, &data) {
                    eprintln!("browser-tabs host: icon {id}: {why}");
                }
            }
            Err(why) => eprintln!("browser-tabs host: bad message: {why}"),
        }
    }
}

/// Store a favicon under its content hash. Files are content-addressed, so an
/// existing file is already correct and other hosts can share it.
fn save_icon(id: &str, data: &str) -> io::Result<()> {
    use base64::Engine;

    if !is_valid_icon_id(id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid icon id"));
    }
    let path = icon_path(id);
    if path.exists() {
        return Ok(());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    // Write then rename so the launcher never loads a half-written file.
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, &path)
}
