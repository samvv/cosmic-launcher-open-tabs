const MESSAGES = {
  connecting: {
    title: "Checking for the launcher plugin…",
    detail: "",
  },
  connected: {
    title: "All set: the launcher plugin is connected.",
    detail: "Open COSMIC Launcher and type “tab ” to search your tabs.",
  },
  missing: {
    title: "The launcher plugin isn't installed for this browser.",
    detail: "Follow the setup steps below, then check again.",
  },
  forbidden: {
    title: "The launcher plugin doesn't recognise this extension.",
    detail:
      "The installed plugin was registered for a different extension ID. Update the plugin, then check again.",
  },
  error: {
    title: "The launcher plugin stopped unexpectedly.",
    detail: "",
  },
};

const statusEl = document.getElementById("status");
const titleEl = document.getElementById("status-title");
const detailEl = document.getElementById("status-detail");
const setupEl = document.getElementById("setup");
const retryEl = document.getElementById("retry");

function render(status) {
  const text = MESSAGES[status.state] ?? MESSAGES.error;
  statusEl.dataset.state = status.state;
  titleEl.textContent = text.title;
  let detail = text.detail;
  if (status.state === "connected" && status.hostVersion) {
    detail += ` (plugin version ${status.hostVersion})`;
  } else if (status.state === "error" && status.detail) {
    detail = `${status.detail} Retrying automatically.`;
  }
  detailEl.textContent = detail;
  setupEl.hidden = status.state === "connected";
}

async function request(type) {
  retryEl.disabled = true;
  try {
    render(await chrome.runtime.sendMessage({ type }));
  } finally {
    retryEl.disabled = false;
  }
}

retryEl.addEventListener("click", () => {
  render({ state: "connecting" });
  request("retry");
});

request("getStatus");
