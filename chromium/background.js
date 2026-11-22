// Background service worker.
//
// Keeps a native messaging connection to the browser-tabs host and pushes the
// full list of open tabs to it whenever something changes. The host caches the
// list and hands it to the COSMIC Launcher plugin on request, and relays the
// plugin's requests to switch to a tab back to us.
//
// An open native messaging port also keeps this service worker alive.

const HOST_NAME = "be.samvervaeck.cosmic_launcher_open_tabs";
const PUSH_DELAY_MS = 100;
const MAX_RETRY_MS = 60_000;
const SETTLE_TIMEOUT_MS = 3_000;
const ICON_SIZE = 32;

// Errors after which retrying is pointless until the user fixes their setup.
const FATAL_ERRORS = {
  "Specified native messaging host not found.": "missing",
  "Access to the specified native messaging host is forbidden.": "forbidden",
};

let port = null;
let retryMs = 1_000;
let retryTimer = null;
let pushTimer = null;

// favIconUrl -> { id, data } for favicons we've already loaded and hashed.
const iconCache = new Map();
// Icon ids the current host has received; reset for every new connection.
let sentIcons = new Set();

// "connecting" | "connected" | "missing" | "forbidden" | "error"
let status = { state: "connecting", detail: null, hostVersion: null };
let statusWaiters = [];

function setStatus(state, extra = {}) {
  status = { state, detail: null, hostVersion: null, ...extra };
  for (const resolve of statusWaiters) resolve(status);
  statusWaiters = [];
}

/// Resolves with the status once the connection attempt has succeeded or failed.
function settledStatus() {
  if (status.state !== "connecting") return Promise.resolve(status);
  return new Promise((resolve) => {
    statusWaiters.push(resolve);
    setTimeout(() => resolve(status), SETTLE_TIMEOUT_MS);
  });
}

function connect() {
  clearTimeout(retryTimer);
  if (port) return;
  setStatus("connecting");
  sentIcons = new Set();
  port = chrome.runtime.connectNative(HOST_NAME);
  port.onMessage.addListener(onHostMessage);
  port.onDisconnect.addListener(() => {
    const message = chrome.runtime.lastError?.message ?? "Disconnected";
    port = null;
    const fatal = FATAL_ERRORS[message];
    if (fatal) {
      // Wait for the user to install the host; the welcome page can retry.
      setStatus(fatal, { detail: message });
      return;
    }
    console.warn("browser-tabs host disconnected:", message);
    setStatus("error", { detail: message });
    retryTimer = setTimeout(connect, retryMs);
    retryMs = Math.min(retryMs * 2, MAX_RETRY_MS);
  });
  pushTabs();
}

async function onHostMessage(message) {
  switch (message.type) {
    case "hello":
      retryMs = 1_000;
      setStatus("connected", { hostVersion: message.version });
      break;
    case "activate":
      try {
        await chrome.tabs.update(message.tabId, { active: true });
        await chrome.windows.update(message.windowId, { focused: true });
      } catch (err) {
        // The tab may have closed since the launcher listed it.
        console.warn("browser-tabs: could not activate tab:", err);
      }
      break;
  }
}

/// Load a tab's favicon from the browser's favicon cache (no network access).
/// Returns { id, data } with a SHA-256 hex id and base64 PNG data, or null.
async function loadIcon(tab) {
  if (!tab.favIconUrl || !tab.url) return null;
  const cached = iconCache.get(tab.favIconUrl);
  if (cached) return cached;

  try {
    const url = new URL(chrome.runtime.getURL("/_favicon/"));
    url.searchParams.set("pageUrl", tab.url);
    url.searchParams.set("size", String(ICON_SIZE));
    const response = await fetch(url);
    if (!response.ok) return null;
    const bytes = new Uint8Array(await response.arrayBuffer());

    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
    const id = Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
    let binary = "";
    for (const b of bytes) binary += String.fromCharCode(b);
    const icon = { id, data: btoa(binary) };

    // While a page loads, the favicon cache may still hold a placeholder,
    // so only remember icons of fully loaded tabs.
    if (tab.status === "complete") iconCache.set(tab.favIconUrl, icon);
    return icon;
  } catch (err) {
    console.warn("browser-tabs: could not load favicon:", err);
    return null;
  }
}

async function pushTabs() {
  if (!port) return;
  const tabs = await chrome.tabs.query({});
  const icons = await Promise.all(tabs.map(loadIcon));
  if (!port) return;

  // Send each icon the host hasn't seen before the tab list that refers to it.
  for (const icon of icons) {
    if (icon && !sentIcons.has(icon.id)) {
      port.postMessage({ type: "icon", id: icon.id, data: icon.data });
      sentIcons.add(icon.id);
    }
  }
  port.postMessage({
    type: "tabs",
    tabs: tabs.map((tab, i) => ({
      id: tab.id,
      windowId: tab.windowId,
      title: tab.title ?? "",
      url: tab.url ?? "",
      active: tab.active,
      iconId: icons[i]?.id,
    })),
  });
}

// Coalesce bursts of tab events (e.g. restoring a session) into one push.
function schedulePush() {
  clearTimeout(pushTimer);
  pushTimer = setTimeout(pushTabs, PUSH_DELAY_MS);
}

chrome.tabs.onCreated.addListener(schedulePush);
chrome.tabs.onRemoved.addListener(schedulePush);
chrome.tabs.onReplaced.addListener(schedulePush);
chrome.tabs.onAttached.addListener(schedulePush);
chrome.tabs.onDetached.addListener(schedulePush);
chrome.tabs.onActivated.addListener(schedulePush);
chrome.tabs.onUpdated.addListener((_id, change) => {
  if ("title" in change || "url" in change || "favIconUrl" in change || change.status === "complete") {
    schedulePush();
  }
});

// Messages from the welcome page.
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type === "getStatus") {
    settledStatus().then(sendResponse);
    return true;
  }
  if (message.type === "retry") {
    retryMs = 1_000;
    connect();
    settledStatus().then(sendResponse);
    return true;
  }
});

// On first install, explain the setup only if the host isn't there yet.
chrome.runtime.onInstalled.addListener(async ({ reason }) => {
  if (reason !== chrome.runtime.OnInstalledReason.INSTALL) return;
  const { state } = await settledStatus();
  if (state !== "connected") {
    chrome.tabs.create({ url: chrome.runtime.getURL("welcome.html") });
  }
});

connect();
