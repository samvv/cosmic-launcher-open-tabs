//! COSMIC Launcher (pop-launcher) plugin.
//!
//! The launcher service talks to plugins over stdin/stdout using
//! newline-delimited JSON: it sends [`Request`]s, we reply with [`PluginResponse`]s.
//! On each search we ask every running native host for its tabs.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::StreamExt;
use pop_launcher::{
    async_stdin, async_stdout, json_input_stream, IconSource, PluginResponse, PluginSearchResult,
    Request,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, Stdout};
use tokio::net::UnixStream;
use tokio::time::timeout;

use crate::tabs::{icon_path, is_valid_icon_id, socket_dir, SocketRequest, Tab};

const HOST_TIMEOUT: Duration = Duration::from_millis(250);

#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    let mut plugin = Plugin {
        out: async_stdout(),
        tabs: Vec::new(),
    };

    let mut requests = json_input_stream(async_stdin());
    while let Some(request) = requests.next().await {
        match request {
            Ok(Request::Search(query)) => plugin.search(&query).await,
            Ok(Request::Activate(id)) => plugin.activate(id).await,
            Ok(Request::Exit) => break,
            Ok(_) => {}
            Err(why) => eprintln!("browser-tabs: malformed request: {why}"),
        }
    }
}

struct Plugin {
    out: Stdout,
    /// Tabs from the last search with the host socket they came from, indexed by result id.
    tabs: Vec<(PathBuf, Tab)>,
}

impl Plugin {
    async fn search(&mut self, query: &str) {
        // An optional `tab ` prefix shows only tabs (see `isolate_with` in plugin.ron).
        let mut query = query.trim_start();
        let mut prefixed = false;
        while let Some(rest) = query.strip_prefix("tab ") {
            query = rest.trim_start();
            prefixed = true;
        }
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();

        // We see every query, including the empty one when the launcher opens.
        // Only list all tabs when explicitly asked with `tab `.
        if words.is_empty() && !prefixed {
            self.tabs.clear();
            self.send(PluginResponse::Finished).await;
            return;
        }

        self.tabs = fetch_all_tabs()
            .await
            .into_iter()
            .filter(|(_, tab)| {
                let haystack = format!("{} {}", tab.title, tab.url).to_lowercase();
                words.iter().all(|w| haystack.contains(w.as_str()))
            })
            .collect();

        let results: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(id, (_, tab))| PluginSearchResult {
                id: id as u32,
                name: if tab.title.is_empty() { tab.url.clone() } else { tab.title.clone() },
                description: tab.url.clone(),
                icon: Some(tab_icon(tab)),
                ..Default::default()
            })
            .collect();
        for result in results {
            self.send(PluginResponse::Append(result)).await;
        }
        self.send(PluginResponse::Finished).await;
    }

    async fn activate(&mut self, id: u32) {
        if let Some((socket, tab)) = self.tabs.get(id as usize) {
            let request = SocketRequest::Activate {
                tab_id: tab.id,
                window_id: tab.window_id,
            };
            if let Err(why) = send_request(socket, &request).await {
                eprintln!("browser-tabs: activating tab failed: {why}");
            }
        }
        self.send(PluginResponse::Close).await;
    }

    async fn send(&mut self, response: PluginResponse) {
        let Ok(mut line) = serde_json::to_vec(&response) else {
            return;
        };
        line.push(b'\n');
        let _ = self.out.write_all(&line).await;
        let _ = self.out.flush().await;
    }
}

/// The tab's favicon if the host saved one, otherwise a generic browser icon.
/// COSMIC Launcher accepts an absolute file path in place of an icon name.
fn tab_icon(tab: &Tab) -> IconSource {
    tab.icon_id
        .as_deref()
        .filter(|id| is_valid_icon_id(id))
        .map(icon_path)
        .filter(|path| path.exists())
        .map(|path| IconSource::Name(path.to_string_lossy().into_owned().into()))
        .unwrap_or_else(|| IconSource::Name("web-browser".into()))
}

/// Query every native host socket concurrently and merge their tabs.
async fn fetch_all_tabs() -> Vec<(PathBuf, Tab)> {
    let Ok(entries) = std::fs::read_dir(socket_dir()) else {
        return Vec::new();
    };
    let sockets: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "sock"))
        .collect();

    let lists = futures::future::join_all(sockets.iter().map(|p| fetch_tabs(p))).await;
    sockets
        .into_iter()
        .zip(lists)
        .flat_map(|(socket, tabs)| tabs.into_iter().map(move |tab| (socket.clone(), tab)))
        .collect()
}

async fn fetch_tabs(path: &Path) -> Vec<Tab> {
    match send_request(path, &SocketRequest::List).await {
        Ok(data) => serde_json::from_slice(&data).unwrap_or_default(),
        Err(e) => {
            // Nobody listening: the host died without cleaning up.
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                let _ = std::fs::remove_file(path);
            }
            Vec::new()
        }
    }
}

/// Send one request to a host and return its full reply.
async fn send_request(path: &Path, request: &SocketRequest) -> std::io::Result<Vec<u8>> {
    let exchange = async {
        let mut stream = UnixStream::connect(path).await?;
        let mut line = serde_json::to_vec(request)?;
        line.push(b'\n');
        stream.write_all(&line).await?;
        let mut reply = Vec::new();
        stream.read_to_end(&mut reply).await?;
        Ok(reply)
    };
    timeout(HOST_TIMEOUT, exchange)
        .await
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::TimedOut))?
}
