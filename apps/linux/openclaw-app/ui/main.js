import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const TABS = [
  { id: "general", label: "General" },
  { id: "connection", label: "Connection" },
  { id: "permissions", label: "Permissions" },
  { id: "voice", label: "Voice & Talk" },
  { id: "channels", label: "Channels" },
  { id: "skills", label: "Skills" },
  { id: "cron", label: "Cron" },
  { id: "exec", label: "Exec Approvals" },
  { id: "sessions", label: "Sessions" },
  { id: "instances", label: "Instances" },
  { id: "config", label: "Config" },
  { id: "debug", label: "Debug" },
  { id: "about", label: "About" },
];

let activeTab = "general";
let toastTimer;
let pairingRefreshTimer = null;

function stopPairingRefresh() {
  if (pairingRefreshTimer != null) {
    clearInterval(pairingRefreshTimer);
    pairingRefreshTimer = null;
  }
}

function startPairingRefresh(pairingBox) {
  stopPairingRefresh();
  pairingRefreshTimer = setInterval(() => {
    if (activeTab === "connection") {
      void refreshPairingBox(pairingBox);
    }
  }, 15_000);
}

const OPERATOR_TABS = {
  channels: "operator_channels_status",
  skills: "operator_skills_status",
  cron: "operator_cron_list",
  sessions: "operator_sessions_list",
  config: "operator_config_get",
};

const OPERATOR_FILTER_TABS = new Set(["skills", "instances", "sessions"]);

function operatorFilterMatch(haystack, filter) {
  const needle = String(filter ?? "")
    .trim()
    .toLowerCase();
  if (!needle) return true;
  return String(haystack ?? "")
    .toLowerCase()
    .includes(needle);
}

async function copyTextToClipboard(text) {
  await navigator.clipboard.writeText(text);
}

function formatGatewayWsUrl({ host, port, use_tls: useTls }) {
  const h = String(host ?? "127.0.0.1").trim() || "127.0.0.1";
  const p = Number(port) > 0 ? Number(port) : 18789;
  const scheme = useTls ? "wss" : "ws";
  return `${scheme}://${h}:${p}`;
}

function isDirectLanHost(host) {
  const h = String(host ?? "")
    .trim()
    .replace(/^\[|\]$/g, "");
  if (!h || h === "localhost" || h === "127.0.0.1" || h === "::1") return true;
  if (h.endsWith(".local")) return true;
  if (/^192\.168\./.test(h) || /^10\./.test(h) || /^172\.(1[6-9]|2\d|3[01])\./.test(h)) return true;
  if (/^fe80:/i.test(h) || /^fd[0-9a-f]{2}:/i.test(h)) return true;
  return false;
}

function parseJsonSafe(raw) {
  if (typeof raw !== "string") return raw;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function formatGatewayVersionSummary(info) {
  if (!info || typeof info !== "object") return "Gateway version: unknown";
  const version = info.version?.trim();
  const source = info.source ? ` (${info.source})` : "";
  if (version) {
    const health =
      info.operatorConnected && info.operatorHealthOk ? " · operator health OK" : "";
    return `Gateway version: ${version}${source}${health}`;
  }
  if (info.operatorConnected) {
    return info.operatorHealthOk
      ? "Gateway version: unavailable · operator connected (health OK)"
      : "Gateway version: unavailable · operator connected";
  }
  return "Gateway version: unavailable (start gateway or connect operator)";
}

function tunnelStatusLine(settings, active, connectionMessage) {
  const msg = String(connectionMessage ?? "");
  const tunnelErr =
    msg.includes("SSH tunnel") || msg.toLowerCase().includes("tunnel")
      ? msg
      : "";
  if ((settings.mode ?? "local") === "remote" && !settings.remote_direct) {
    if (tunnelErr && !active) {
      return `SSH tunnel: ${tunnelErr}`;
    }
    return active
      ? tunnelErr
        ? `SSH tunnel: active — ${tunnelErr}`
        : "SSH tunnel: active"
      : tunnelErr
        ? `SSH tunnel: stopped — ${tunnelErr}`
        : "SSH tunnel: stopped (use Start SSH tunnel)";
  }
  if (settings.remote_direct) {
    return "SSH tunnel: skipped (remote direct WebSocket)";
  }
  return "";
}

function applyConnectionStatusMessage(message) {
  const text = String(message ?? "").trim();
  const statusEl = document.getElementById("connection-status");
  if (statusEl && text) statusEl.textContent = text;
  const bar = document.getElementById("conn-bar");
  if (bar && text) bar.textContent = text;
}

async function refreshConnBar() {
  const bar = document.getElementById("conn-bar");
  if (!bar) return;
  try {
    const raw = await invoke("get_connection_health");
    const health = parseJsonSafe(raw);
    if (health && typeof health === "object") {
      const op = health.operatorConnected ? "operator ✓" : "operator ✗";
      const node = health.nodeConnected ? "node ✓" : "node ✗";
      const tunnel = health.sshTunnelActive ? " · tunnel ✓" : "";
      const msg = health.message ? ` — ${health.message}` : "";
      bar.textContent = `${op} · ${node}${tunnel}${msg}`;
      return;
    }
    const status = await invoke("get_connection_status");
    bar.textContent = status || "No gateway connection status yet";
  } catch {
    bar.textContent = "Status unavailable";
  }
}

async function ensureOperator() {
  try {
    await invoke("operator_connect");
    return true;
  } catch (err) {
    showToast(String(err), true);
    return false;
  }
}

function summaryList(title, items, format) {
  if (!items?.length) {
    return el("p", { textContent: `${title}: (none)` });
  }
  return el("div", {}, [
    el("strong", { textContent: title }),
    el(
      "ul",
      {},
      items.map((item) => el("li", { textContent: format(item) })),
    ),
  ]);
}

function formatTimestamp(ms) {
  if (ms == null || !Number.isFinite(Number(ms))) return "—";
  try {
    return new Date(Number(ms)).toLocaleString();
  } catch {
    return String(ms);
  }
}

function truncateValue(value, max = 80) {
  const text =
    typeof value === "string"
      ? value
      : value == null
        ? ""
        : JSON.stringify(value);
  if (text.length <= max) return text;
  return `${text.slice(0, max)}…`;
}

function normalizeChannelStatus(channel) {
  if (typeof channel === "string") return channel;
  return String(
    channel?.status ?? channel?.state ?? (channel?.enabled === false ? "off" : "on"),
  );
}

function cronJobStatus(job) {
  if (job.enabled === false || job.disabled === true) return "disabled";
  return String(job.status ?? job.state ?? "enabled");
}

function statusBadgeClass(statusText) {
  const s = String(statusText ?? "").toLowerCase();
  if (/run|on|connected|active|ready|open|enabled/.test(s)) {
    return "status-badge status-badge-on";
  }
  if (/stop|off|disabled|idle|closed|error|fail/.test(s)) {
    return "status-badge status-badge-off";
  }
  return "status-badge status-badge-neutral";
}

function statusBadgeEl(statusText) {
  const label = String(statusText ?? "unknown");
  return el("span", { className: statusBadgeClass(label), textContent: label });
}

function tunnelLineClassName(settings, active, connectionMessage) {
  const msg = String(connectionMessage ?? "").toLowerCase();
  if (msg.includes("reconnecting")) return "status-line status-warn";
  if (
    msg.includes("tunnel") &&
    (msg.includes("failed") || msg.includes("disconnected"))
  ) {
    return "status-line status-error";
  }
  if ((settings.mode ?? "local") === "remote" && !settings.remote_direct && active) {
    return "status-line status-ok";
  }
  return "status-line";
}

let highlightExecId = null;

async function refreshExecPendingBox(box) {
  if (!box) return;
  box.replaceChildren(el("p", { textContent: "Loading…" }));
  try {
    const pending = await invoke("get_pending_exec_approvals");
    const list = Array.isArray(pending) ? pending : [];
    box.replaceChildren();
    if (!list.length) {
      box.append(
        el("p", {
          className: "help",
          textContent: "No pending exec approvals. Gateway CLI prompts appear here while the node is connected.",
        }),
      );
      return;
    }
    for (const item of list) {
      const id = item.id ?? "?";
      const cmd = item.command ?? "command";
      const cwd = item.cwd ? `\ncwd: ${item.cwd}` : "";
      const card = el("div", {
        className: `exec-pending-card${highlightExecId === id ? " exec-pending-highlight" : ""}`,
        id: highlightExecId === id ? "exec-pending-focus" : undefined,
      }, [
        el("pre", { className: "exec-pending-command", textContent: `${cmd}${cwd}` }),
        el("div", { className: "actions" }, [
          el("button", {
            textContent: "Allow once",
            onclick: async () => {
              try {
                await invoke("resolve_exec_approval", { id, decision: "allow-once" });
                highlightExecId = null;
                showToast("Exec allowed");
                await refreshExecPendingBox(box);
              } catch (err) {
                showToast(String(err), true);
              }
            },
          }),
          el("button", {
            textContent: "Deny",
            onclick: async () => {
              try {
                await invoke("resolve_exec_approval", { id, decision: "deny" });
                highlightExecId = null;
                showToast("Exec denied");
                await refreshExecPendingBox(box);
              } catch (err) {
                showToast(String(err), true);
              }
            },
          }),
        ]),
      ]);
      box.append(card);
    }
    if (highlightExecId) {
      document.getElementById("exec-pending-focus")?.scrollIntoView({ block: "nearest" });
    }
  } catch (err) {
    box.replaceChildren(el("p", { className: "help", textContent: String(err) }));
  }
}

function extractPresenceEntries(data) {
  if (Array.isArray(data)) return data;
  if (data?.nodes && Array.isArray(data.nodes)) return data.nodes;
  if (data?.items && Array.isArray(data.items)) return data.items;
  if (data && typeof data === "object") {
    return Object.entries(data)
      .filter(([, v]) => v && typeof v === "object")
      .map(([key, v]) => ({ ...v, _presenceKey: key }));
  }
  return [];
}

function extractSkills(data) {
  const skills = data?.skills ?? data?.items;
  return Array.isArray(skills) ? skills : [];
}

function skillIsEnabled(skill) {
  if (skill.enabled === false || skill.disabled === true) return false;
  if (skill.blockedByAllowlist || skill.blockedByAgentFilter) return false;
  if (skill.eligible === false) return false;
  return true;
}

function renderOperatorSummary(tab, data, filter = "") {
  const wrap = el("div", { className: "operator-summary" });
  if (data == null) {
    wrap.append(el("p", { textContent: "No structured data." }));
    return wrap;
  }
  if (tab === "channels") {
    const channels =
      data.channels ?? data.items ?? (Array.isArray(data) ? data : Object.values(data));
    const list = Array.isArray(channels) ? channels : [];
    if (!list.length) {
      wrap.append(el("p", { textContent: "Channels: (none)" }));
      return wrap;
    }
    wrap.append(el("strong", { textContent: "Channels" }));
    const ul = el("ul");
    for (const c of list) {
      const id = c.id ?? c.channelId ?? c.name ?? "?";
      const status = normalizeChannelStatus(c);
      ul.append(
        el("li", { className: "channel-summary-row" }, [
          el("span", { textContent: id }),
          statusBadgeEl(status),
        ]),
      );
    }
    wrap.append(ul);
    return wrap;
  }
  if (tab === "cron") {
    const jobs = data.jobs ?? data.items ?? (Array.isArray(data) ? data : []);
    const jobList = Array.isArray(jobs) ? jobs : [];
    if (!jobList.length) {
      wrap.append(el("p", { textContent: "Cron jobs: (none)" }));
      return wrap;
    }
    wrap.append(el("strong", { textContent: "Cron jobs" }));
    const ul = el("ul");
    for (const j of jobList) {
      const name = j.name ?? j.id ?? "?";
      ul.append(
        el("li", { className: "cron-summary-row" }, [
          el("span", { textContent: name }),
          statusBadgeEl(cronJobStatus(j)),
        ]),
      );
    }
    wrap.append(ul);
    return wrap;
  }
  if (tab === "instances") {
    const entries = extractPresenceEntries(data).filter((p) => {
      const id =
        p.deviceId ??
        p.instanceId ??
        p.nodeId ??
        p.id ??
        p._presenceKey ??
        p.host ??
        "?";
      const platform = p.platform ?? p.deviceFamily ?? "";
      const host = p.host && p.host !== id ? ` @ ${p.host}` : "";
      const mode = p.mode ? ` (${p.mode})` : "";
      const line = `${id}${platform ? ` — ${platform}` : ""}${host}${mode}`.trim();
      return operatorFilterMatch(`${line} ${id} ${platform} ${p.host ?? ""}`, filter);
    });
    wrap.append(
      summaryList(
        filter.trim() ? `Connected instances (filtered)` : "Connected instances",
        entries,
        (p) => {
          const id =
            p.deviceId ??
            p.instanceId ??
            p.nodeId ??
            p.id ??
            p._presenceKey ??
            p.host ??
            "?";
          const platform = p.platform ?? p.deviceFamily ?? "";
          const host = p.host && p.host !== id ? ` @ ${p.host}` : "";
          const mode = p.mode ? ` (${p.mode})` : "";
          return `${id}${platform ? ` — ${platform}` : ""}${host}${mode}`.trim();
        },
      ),
    );
    return wrap;
  }
  if (tab === "sessions") {
    const sessions = data.sessions ?? data.items ?? (Array.isArray(data) ? data : []);
    const list = (Array.isArray(sessions) ? sessions : []).filter((s) => {
      const key = s.key ?? s.sessionKey ?? s.id ?? "";
      let agent = s.agentId ?? s.agent ?? "";
      if (!agent && typeof key === "string" && key.startsWith("agent:")) {
        agent = key.split(":")[1] ?? "";
      }
      return operatorFilterMatch(`${key} ${agent}`, filter);
    });
    if (!list.length) {
      wrap.append(el("p", { textContent: "Sessions: (none)" }));
      return wrap;
    }
    const table = el("table", { className: "operator-table" });
    table.append(
      el("thead", {}, [
        el("tr", {}, [
          el("th", { textContent: "Session key" }),
          el("th", { textContent: "Agent" }),
          el("th", { textContent: "Updated" }),
        ]),
      ]),
    );
    const tbody = el("tbody");
    for (const s of list.slice(0, 50)) {
      const key = s.key ?? s.sessionKey ?? s.id ?? "?";
      let agent = s.agentId ?? s.agent ?? "";
      if (!agent && typeof key === "string" && key.startsWith("agent:")) {
        agent = key.split(":")[1] ?? "";
      }
      tbody.append(
        el("tr", {}, [
          el("td", { textContent: key, title: key }),
          el("td", { textContent: agent || "—" }),
          el("td", { textContent: formatTimestamp(s.updatedAt ?? s.updated ?? s.ts) }),
        ]),
      );
    }
    table.append(tbody);
    wrap.append(el("strong", { textContent: `Sessions (${list.length})` }), table);
    if (list.length > 50) {
      wrap.append(el("p", { className: "help", textContent: "Showing first 50 rows." }));
    }
    return wrap;
  }
  if (tab === "skills") {
    const skills = extractSkills(data).filter((s) => {
      const label = s.name ?? s.skillKey ?? s.id ?? s.description ?? "";
      return operatorFilterMatch(label, filter);
    });
    const enabled = skills.filter(skillIsEnabled);
    const disabled = skills.length - enabled.length;
    wrap.append(
      el("p", {
        textContent: `${enabled.length} enabled · ${disabled} disabled/unavailable · ${skills.length} total${filter.trim() ? " (filtered)" : ""}`,
      }),
    );
    const top = enabled.slice(0, 8).map((s) => s.name ?? s.skillKey ?? s.id ?? "?");
    if (top.length) {
      wrap.append(el("p", { textContent: `Top enabled: ${top.join(", ")}` }));
    } else if (skills.length) {
      wrap.append(
        el("p", {
          textContent: `Skills: ${skills
            .slice(0, 8)
            .map((s) => s.name ?? s.skillKey ?? "?")
            .join(", ")}`,
        }),
      );
    }
    return wrap;
  }
  if (tab === "config") {
    const cfg = data.config ?? data;
    const keys =
      cfg && typeof cfg === "object" && !Array.isArray(cfg) ? Object.keys(cfg) : [];
    wrap.append(el("p", { textContent: `${keys.length} top-level config keys` }));
    const gateway = cfg?.gateway;
    if (gateway && typeof gateway === "object") {
      const mode =
        gateway.mode ??
        gateway.transport ??
        (gateway.remote ? "remote" : undefined) ??
        "—";
      const bind = gateway.bind ?? gateway.host;
      const port = gateway.port;
      const snippet = [
        mode !== "—" ? `mode: ${truncateValue(mode, 40)}` : null,
        bind != null ? `bind: ${truncateValue(bind, 40)}` : null,
        port != null ? `port: ${port}` : null,
      ]
        .filter(Boolean)
        .join(" · ");
      if (snippet) {
        wrap.append(el("p", { className: "config-snippet", textContent: `Gateway: ${snippet}` }));
      }
      const rawGateway = truncateValue(gateway, 120);
      if (rawGateway) {
        wrap.append(el("pre", { className: "config-snippet-pre", textContent: rawGateway }));
      }
    }
    return wrap;
  }
  wrap.append(el("p", { textContent: "Loaded. Use “View raw JSON” below for full payload." }));
  return wrap;
}

function rawJsonDetails(rawText) {
  const pre = el("pre", { className: "log", textContent: rawText });
  return el("details", { className: "raw-json" }, [
    el("summary", { textContent: "View raw JSON (collapsed)" }),
    pre,
  ]);
}

async function mountOperatorTab(panel, tab, cmd) {
  const summaryHost = el("div");
  const rawHost = el("div", { className: "raw-json-host" });
  let lastRawText = "";
  const copyBtn = el("button", {
    className: "action",
    textContent: "Copy JSON",
    disabled: true,
    onclick: async () => {
      if (!lastRawText) return;
      try {
        await copyTextToClipboard(lastRawText);
        showToast("Copied JSON to clipboard");
      } catch (err) {
        showToast(String(err), true);
      }
    },
  });
  const filterInput = OPERATOR_FILTER_TABS.has(tab)
    ? el("input", {
        type: "search",
        className: "operator-filter",
        placeholder: "Filter list…",
      })
    : null;
  if (filterInput) {
    filterInput.addEventListener("input", () => {
      void loadOperatorTab(tab, cmd, summaryHost, rawHost, filterInput.value, (text) => {
        lastRawText = text;
        copyBtn.disabled = !text;
      });
    });
  }
  const reload = () =>
    loadOperatorTab(tab, cmd, summaryHost, rawHost, filterInput?.value ?? "", (text) => {
      lastRawText = text;
      copyBtn.disabled = !text;
    });
  panel.append(
    el("div", { className: "operator-toolbar" }, [
      el("button", {
        className: "action",
        textContent: "Reload",
        onclick: () => reload(),
      }),
      copyBtn,
      filterInput,
    ].filter(Boolean)),
    summaryHost,
    rawHost,
  );
  await reload();
}

function extractChannelEntries(data) {
  const raw = data?.channels ?? data?.items ?? (Array.isArray(data) ? data : null);
  if (raw && typeof raw === "object" && !Array.isArray(raw)) {
    return Object.entries(raw)
      .filter(([id]) => id.length > 0)
      .map(([id, value]) => ({
        id,
        status:
          value && typeof value === "object"
            ? normalizeChannelStatus(value)
            : String(value ?? "on"),
      }));
  }
  const list = Array.isArray(raw) ? raw : [];
  return list
    .map((c) => {
      if (typeof c === "string") return { id: c, status: "on" };
      const id = c.id ?? c.channelId ?? c.name;
      if (typeof id !== "string" || !id.length) return null;
      return { id, status: normalizeChannelStatus(c) };
    })
    .filter(Boolean);
}

function extractChannelIds(data) {
  return extractChannelEntries(data).map((c) => c.id);
}

function renderChannelActions(channelEntries) {
  const wrap = el("div", { className: "channel-actions" });
  if (!channelEntries.length) {
    wrap.append(el("p", { textContent: "No channels reported by gateway." }));
    return wrap;
  }
  wrap.append(
    el("p", {
      className: "help",
      textContent: "Start/stop channel accounts (plugin must support gateway start/stop).",
    }),
  );
  for (const { id, status } of channelEntries.slice(0, 12)) {
    wrap.append(
      el("div", { className: "channel-action-row" }, [
        el("span", { textContent: id }),
        statusBadgeEl(status),
        el("button", {
          className: "action",
          textContent: "Start",
          onclick: async () => {
            try {
              await invoke("operator_channel_start", { channel: id });
              showToast(`Started ${id}`);
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Stop",
          onclick: async () => {
            try {
              await invoke("operator_channel_stop", { channel: id });
              showToast(`Stopped ${id}`);
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
      ]),
    );
  }
  return wrap;
}

function extractSessions(data) {
  const sessions = data?.sessions ?? data?.items ?? (Array.isArray(data) ? data : []);
  return Array.isArray(sessions) ? sessions : [];
}

function extractSessionKeys(data) {
  return extractSessions(data)
    .map((s) => s.key ?? s.sessionKey ?? s.id)
    .filter((key) => typeof key === "string" && key.length > 0);
}

function renderSessionActions(sessions, filter = "") {
  const wrap = el("div", { className: "session-actions" });
  const list = extractSessions(sessions).filter((s) => {
    const key = s.key ?? s.sessionKey ?? s.id ?? "";
    let agent = s.agentId ?? s.agent ?? "";
    if (!agent && typeof key === "string" && key.startsWith("agent:")) {
      agent = key.split(":")[1] ?? "";
    }
    return operatorFilterMatch(`${key} ${agent}`, filter);
  });
  if (!list.length) {
    wrap.append(el("p", { textContent: "No sessions to open." }));
    return wrap;
  }
  wrap.append(
    el("p", {
      className: "help",
      textContent:
        "Open a session in WebChat, fetch a transcript preview, or load full session metadata.",
    }),
  );
  for (const s of list.slice(0, 15)) {
    const key = s.key ?? s.sessionKey ?? s.id;
    if (!key) continue;
    wrap.append(
      el("div", { className: "channel-action-row session-action-row" }, [
        el("span", { className: "session-key", textContent: key, title: key }),
        el("div", { className: "session-action-buttons" }, [
          el("button", {
            className: "action",
            textContent: "WebChat",
            onclick: () =>
              invoke("open_webchat", { session: String(key) }).catch((err) =>
                showToast(String(err), true),
              ),
          }),
          el("button", {
            className: "action",
            textContent: "Dashboard chat",
            onclick: () =>
              invoke("open_dashboard_chat", { session: String(key) }).catch((err) =>
                showToast(String(err), true),
              ),
          }),
          el("button", {
            className: "action",
            textContent: "Preview",
            onclick: async () => {
              try {
                const out = await invoke("operator_sessions_preview", {
                  sessionKeys: [String(key)],
                  limit: 8,
                });
                const log = document.getElementById("log");
                if (log) log.textContent = typeof out === "string" ? out : JSON.stringify(out, null, 2);
                showToast(`Preview loaded for ${key}`);
              } catch (err) {
                showToast(String(err), true);
              }
            },
          }),
          el("button", {
            className: "action",
            textContent: "Describe",
            onclick: async () => {
              try {
                const out = await invoke("operator_sessions_describe", {
                  sessionKey: String(key),
                  includeDerivedTitles: true,
                  includeLastMessage: true,
                });
                const log = document.getElementById("log");
                if (log) log.textContent = typeof out === "string" ? out : JSON.stringify(out, null, 2);
                showToast(`Describe loaded for ${key}`);
              } catch (err) {
                showToast(String(err), true);
              }
            },
          }),
        ]),
      ]),
    );
  }
  return wrap;
}

function extractCronJobs(data) {
  const jobs = data?.jobs ?? data?.items ?? (Array.isArray(data) ? data : []);
  return Array.isArray(jobs) ? jobs : [];
}

function renderCronRunPanel(jobs) {
  const wrap = el("div", { className: "cron-run-panel" });
  if (!jobs.length) {
    wrap.append(el("p", { textContent: "No cron jobs to run." }));
    return wrap;
  }
  wrap.append(
    el("p", {
      className: "help",
      textContent: "Run a job now (requires operator admin scope on the gateway).",
    }),
  );
  for (const job of jobs.slice(0, 20)) {
    const id = job.id ?? job.jobId;
    if (!id) continue;
    const label = job.name ?? id;
    wrap.append(
      el("button", {
        className: "action",
        textContent: `Run: ${label}`,
        onclick: async () => {
          try {
            const out = await invoke("operator_cron_run", { jobId: String(id) });
            showToast(`Cron run started: ${label}`);
            const log = document.getElementById("log");
            if (log) log.textContent = typeof out === "string" ? out : JSON.stringify(out, null, 2);
          } catch (err) {
            showToast(String(err), true);
          }
        },
      }),
    );
  }
  return wrap;
}

async function loadOperatorTab(tab, cmd, summaryHost, rawHost, filter = "", onRawText) {
  rawHost.replaceChildren(el("p", { className: "help", textContent: "Loading…" }));
  summaryHost.replaceChildren();
  if (!(await ensureOperator())) {
    rawHost.replaceChildren(el("p", { textContent: "Operator not connected." }));
    onRawText?.("");
    return;
  }
  try {
    const raw = await invoke(cmd);
    const data = parseJsonSafe(raw);
    const summary = renderOperatorSummary(tab, data, filter);
    summaryHost.replaceChildren(summary);
    if (tab === "channels") {
      summaryHost.append(renderChannelActions(extractChannelEntries(data)));
    }
    if (tab === "sessions") {
      summaryHost.append(renderSessionActions(data, filter));
    }
    if (tab === "cron") {
      try {
        const statusRaw = await invoke("operator_cron_status");
        const statusPre = el("pre", {
          className: "log",
          textContent: typeof statusRaw === "string" ? statusRaw : JSON.stringify(statusRaw, null, 2),
        });
        summaryHost.append(statusPre, renderCronRunPanel(extractCronJobs(data)));
      } catch (err) {
        summaryHost.append(
          el("p", { className: "help", textContent: `cron.status: ${String(err)}` }),
          renderCronRunPanel(extractCronJobs(data)),
        );
      }
    }
    const rawText = typeof raw === "string" ? raw : JSON.stringify(raw, null, 2);
    rawHost.replaceChildren(rawJsonDetails(rawText));
    onRawText?.(rawText);
  } catch (err) {
    rawHost.replaceChildren(el("pre", { className: "log", textContent: String(err) }));
    onRawText?.("");
    showToast(String(err), true);
  }
}

function pairingPendingList(raw) {
  const data = parseJsonSafe(raw);
  return Array.isArray(data?.pending) ? data.pending : [];
}

async function approveAllPairing(kind, raw, pairingBox) {
  const pending = pairingPendingList(raw);
  if (!pending.length) return;
  const approveCmd = kind === "node" ? "pairing_approve_node" : "pairing_approve_device";
  let ok = 0;
  for (const req of pending) {
    const requestId = req.requestId;
    if (!requestId) continue;
    try {
      await invoke(approveCmd, { requestId });
      ok += 1;
    } catch (err) {
      showToast(String(err), true);
      break;
    }
  }
  if (ok) showToast(`Approved ${ok} pending ${kind} request(s)`);
  await refreshPairingBox(pairingBox);
}

async function rejectAllPairing(kind, raw, pairingBox) {
  const pending = pairingPendingList(raw);
  if (!pending.length) return;
  const rejectCmd = kind === "node" ? "pairing_reject_node" : "pairing_reject_device";
  let ok = 0;
  for (const req of pending) {
    const requestId = req.requestId;
    if (!requestId) continue;
    try {
      await invoke(rejectCmd, { requestId });
      ok += 1;
    } catch (err) {
      showToast(String(err), true);
      break;
    }
  }
  if (ok) showToast(`Rejected ${ok} pending ${kind} request(s)`);
  await refreshPairingBox(pairingBox);
}

async function refreshPairingBox(pairingBox) {
  if (!pairingBox) return;
  pairingBox.replaceChildren(el("p", { textContent: "Loading…" }));
  if (!(await ensureOperator())) {
    pairingBox.replaceChildren(
      el("p", { textContent: "Connect operator on the Connection tab to list pending requests." }),
    );
    return;
  }
  try {
    const devices = await invoke("pairing_list_devices");
    const nodes = await invoke("pairing_list_nodes");
    pairingBox.replaceChildren();
    renderPairingPending("device", devices, pairingBox);
    renderPairingPending("node", nodes, pairingBox);
  } catch (err) {
    pairingBox.replaceChildren(el("pre", { textContent: String(err) }));
  }
}

function renderPairingPending(kind, raw, box) {
  const pending = pairingPendingList(raw);
  if (!pending.length) {
    box.append(el("p", { textContent: `No pending ${kind} pairing requests.` }));
    return;
  }
  box.append(
    el("div", { className: "pairing-batch-row" }, [
      el("strong", { textContent: `${kind} (${pending.length} pending)` }),
      el("button", {
        className: "action",
        textContent: `Approve all ${kind}`,
        onclick: () => approveAllPairing(kind, raw, box),
      }),
      el("button", {
        className: "action",
        textContent: `Reject all ${kind}`,
        onclick: () => rejectAllPairing(kind, raw, box),
      }),
    ]),
  );
  for (const req of pending) {
    const requestId = req.requestId;
    const name = req.displayName ?? req.deviceId ?? req.nodeId ?? requestId;
    const approveCmd = kind === "node" ? "pairing_approve_node" : "pairing_approve_device";
    const rejectCmd = kind === "node" ? "pairing_reject_node" : "pairing_reject_device";
    box.append(
      el("div", { className: "pairing-card" }, [
        el("span", { textContent: name }),
        el("div", { className: "actions" }, [
          el("button", {
            textContent: "Approve",
            onclick: async () => {
              try {
                await invoke(approveCmd, { requestId });
                showToast(`Approved ${name}`);
                await refreshPairingBox(box);
              } catch (err) {
                showToast(String(err), true);
              }
            },
          }),
          el("button", {
            textContent: "Reject",
            onclick: async () => {
              try {
                await invoke(rejectCmd, { requestId });
                showToast(`Rejected ${name}`);
                await refreshPairingBox(box);
              } catch (err) {
                showToast(String(err), true);
              }
            },
          }),
        ]),
      ]),
    );
  }
}

function showToast(message, isError = false) {
  let toast = document.getElementById("toast");
  if (!toast) {
    toast = el("div", { id: "toast", className: "toast" });
    document.body.append(toast);
  }
  toast.textContent = message;
  toast.className = isError ? "toast toast-error" : "toast";
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.textContent = "";
    toast.className = "toast hidden";
  }, 6000);
}

function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  Object.assign(node, props);
  for (const child of children) {
    node.append(child);
  }
  return node;
}

function renderTabs() {
  const nav = document.getElementById("tabs");
  nav.replaceChildren(
    ...TABS.map((tab) =>
      el(
        "button",
        {
          textContent: tab.label,
          className: tab.id === activeTab ? "active" : "",
          onclick: () => {
            activeTab = tab.id;
            if (tab.id !== "connection") {
              stopPairingRefresh();
            }
            renderTabs();
            renderPanel();
          },
        },
      ),
    ),
  );
}

async function renderPanel() {
  const panel = document.getElementById("panel");
  panel.className = "panel";
  panel.replaceChildren();

  if (activeTab === "general") {
    const settings = await invoke("get_connection_settings");
    const gatewayHint = el("p", {
      className: "status-line",
      textContent: "Checking gateway…",
    });
    const refreshGatewayHint = async () => {
      try {
        const out = await invoke("get_gateway_status");
        const running =
          typeof out === "string" &&
          (out.includes('"status":"running"') ||
            out.includes('"status": "running"') ||
            out.includes('"ok":true') ||
            out.includes('"ok": true'));
        gatewayHint.textContent = running
          ? "Gateway is running."
          : "Gateway is not running. Install the service below, or run: openclaw gateway start";
      } catch {
        gatewayHint.textContent =
          "Gateway status unavailable. Install the gateway service or check openclaw is on PATH.";
      }
    };
    await refreshGatewayHint();
    const gatewayVersionLine = el("p", {
      className: "status-line",
      textContent: "Checking gateway version…",
    });
    const refreshGatewayVersion = async () => {
      try {
        const raw = await invoke("get_gateway_version_info");
        const info = parseJsonSafe(raw);
        gatewayVersionLine.textContent = formatGatewayVersionSummary(info);
      } catch {
        gatewayVersionLine.textContent = "Gateway version: unavailable";
      }
    };
    await refreshGatewayVersion();
    const cliPathLine = el("p", { className: "status-line", textContent: "Checking CLI…" });
    const cliInstallStatus = el("p", {
      className: "status-line cli-install-status",
      textContent: "",
      hidden: true,
    });
    try {
      const loc = await invoke("cli_installed_location");
      cliPathLine.textContent = loc
        ? `CLI: ${loc}`
        : "CLI not found on PATH (~/.openclaw/bin). Install below or use the official install script.";
    } catch {
      cliPathLine.textContent = "CLI location unavailable.";
    }
    const autostartToggle = el("input", {
      type: "checkbox",
      id: "gateway_autostart",
      checked: settings.gateway_autostart !== false,
    });
    autostartToggle.addEventListener("change", async () => {
      const enabled = autostartToggle.checked;
      try {
        await invoke("set_gateway_autostart", { enabled });
        showToast(
          enabled
            ? "Gateway will start when you open OpenClaw (if not already running)"
            : "Gateway will not start automatically",
        );
      } catch (err) {
        autostartToggle.checked = !enabled;
        showToast(String(err), true);
      }
    });
    const webchatSessionSelect = el("select", {
      id: "webchat_session_picker",
      className: "webchat-session-picker",
    });
    const refreshWebchatSessions = async () => {
      webchatSessionSelect.replaceChildren(
        el("option", { value: "", textContent: "(default — last session or gateway default)" }),
      );
      const saved = settings.last_webchat_session?.trim() ?? "";
      const keys = [];
      if (saved) keys.push(saved);
      if (await ensureOperator()) {
        try {
          const raw = await invoke("operator_sessions_list");
          for (const key of extractSessionKeys(parseJsonSafe(raw))) {
            if (!keys.includes(key)) keys.push(key);
          }
        } catch {
          /* saved session only when list unavailable */
        }
      }
      for (const key of keys.slice(0, 50)) {
        const opt = el("option", { value: key, textContent: key });
        if (key === saved) opt.selected = true;
        webchatSessionSelect.append(opt);
      }
      if (saved && !keys.length) {
        webchatSessionSelect.append(
          el("option", { value: saved, textContent: `${saved} (saved)`, selected: true }),
        );
      }
    };
    await refreshWebchatSessions();
    const openWebchatWithPicker = () => {
      const session = webchatSessionSelect.value.trim();
      return invoke("open_webchat", { session: session || null }).catch((err) =>
        showToast(String(err), true),
      );
    };
    panel.append(
      gatewayHint,
      gatewayVersionLine,
      cliPathLine,
      cliInstallStatus,
      el("label", { className: "checkbox-row", htmlFor: "gateway_autostart" }, [
        autostartToggle,
        el("span", {
          textContent:
            "Start gateway when OpenClaw launches (local mode; skips start if already running)",
        }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "WebChat session" }),
        el("p", {
          className: "help",
          textContent:
            "Pick a session from sessions.list when the operator is connected, or leave default to restore the last opened session.",
        }),
        el("label", { htmlFor: "webchat_session_picker", textContent: "Session" }),
        webchatSessionSelect,
        el("button", {
          className: "action",
          textContent: "Refresh session list",
          onclick: () => refreshWebchatSessions().catch((err) => showToast(String(err), true)),
        }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Gateway" }),
        el("button", {
          className: "action",
          textContent: "Install gateway service (systemd)",
          onclick: async () => {
            await run("install_gateway_service");
            await refreshGatewayHint();
          },
        }),
        el("button", {
          className: "action",
          textContent: "Start gateway now",
          onclick: async () => {
            try {
              await invoke("ensure_gateway_running_cmd");
              showToast("Gateway started");
              await refreshGatewayHint();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Open Dashboard",
          onclick: () => invoke("open_dashboard"),
        }),
        el("button", {
          className: "action",
          textContent: "Open WebChat",
          onclick: () => openWebchatWithPicker(),
        }),
        el("button", {
          className: "action",
          textContent: "Open chat in Dashboard",
          onclick: () => {
            const session = webchatSessionSelect.value.trim();
            return invoke("open_dashboard_chat", { session: session || null }).catch((err) =>
              showToast(String(err), true),
            );
          },
        }),
        el("button", {
          className: "action",
          textContent: "Refresh gateway version",
          onclick: () => refreshGatewayVersion(),
        }),
        el("button", {
          className: "action",
          textContent: "Install CLI (official script)",
          onclick: async () => {
            cliInstallStatus.hidden = false;
            cliInstallStatus.className = "status-line cli-install-status";
            cliInstallStatus.textContent = "Installing CLI…";
            try {
              const out = await invoke("install_cli");
              cliInstallStatus.className = "status-line cli-install-status";
              cliInstallStatus.textContent = "CLI install finished.";
              showToast("CLI install finished");
              const log = document.getElementById("log");
              if (log) log.textContent = out;
              const loc = await invoke("cli_installed_location");
              cliPathLine.textContent = loc
                ? `CLI: ${loc}`
                : "CLI install finished; restart shell or add ~/.openclaw/bin to PATH.";
            } catch (err) {
              const msg = String(err);
              cliInstallStatus.className = "status-line status-error cli-install-status";
              cliInstallStatus.textContent = msg.length > 400 ? `${msg.slice(0, 400)}…` : msg;
              showToast(msg.split("\n")[0] || "CLI install failed", true);
              const log = document.getElementById("log");
              if (log) log.textContent = msg;
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Install node host service (remote mode)",
          onclick: () => run("install_node_service"),
        }),
      ]),
    );
  }

  if (activeTab === "connection") {
    const currentSettings = await invoke("get_connection_settings");
    const mode = el("select", { id: "mode" }, [
      el("option", { value: "local", textContent: "Local" }),
      el("option", { value: "remote", textContent: "Remote" }),
    ]);
    mode.value = currentSettings.mode ?? "local";
    const host = el("input", { id: "host", value: currentSettings.host ?? "127.0.0.1" });
    const port = el("input", {
      id: "port",
      type: "number",
      value: String(currentSettings.port ?? 18789),
    });
    const token = el("input", { id: "token", value: currentSettings.token ?? "" });
    const sshTarget = el("input", {
      id: "ssh_target",
      value: currentSettings.ssh_target ?? "",
      placeholder: "user@host (gateway.remote.sshTarget)",
    });
    const sshIdentity = el("input", {
      id: "ssh_identity",
      value: currentSettings.ssh_identity ?? "",
      placeholder: "~/.ssh/id_ed25519",
    });
    const remoteDirect = el("input", { type: "checkbox", id: "remote_direct" });
    remoteDirect.checked = Boolean(currentSettings.remote_direct);
    const useTls = el("input", { type: "checkbox", id: "use_tls" });
    useTls.checked = Boolean(currentSettings.use_tls);
    const wsUrlHint = el("p", {
      id: "ws-url-hint",
      className: "help ws-url-hint",
      textContent: "",
    });
    const refreshWsUrlHint = () => {
      const url = formatGatewayWsUrl({
        host: host.value,
        port: port.value,
        use_tls: useTls.checked,
      });
      wsUrlHint.textContent = useTls.checked
        ? `Gateway WebSocket (TLS): ${url}`
        : `Gateway WebSocket: ${url}`;
    };
    host.addEventListener("input", refreshWsUrlHint);
    port.addEventListener("input", refreshWsUrlHint);
    useTls.addEventListener("change", refreshWsUrlHint);
    refreshWsUrlHint();
    const discoveryBox = el("div", { className: "discovery-box" });
    const pairingBox = el("div", { id: "pairing-box", className: "pairing-box" });
    const statusProminent = el("div", {
      id: "connection-status",
      className: "connection-status-prominent",
      textContent: "Loading connection status…",
    });
    const tunnelLine = el("p", { id: "tunnel-status", className: "status-line", textContent: "" });
    const deviceLine = el("p", { className: "status-line", textContent: "Device identity…" });
    try {
      const idRaw = await invoke("get_device_identity");
      const idData = parseJsonSafe(idRaw);
      const deviceId = idData?.deviceId ?? "?";
      deviceLine.textContent = `Node device id: ${deviceId}`;
    } catch {
      deviceLine.textContent = "Device identity unavailable.";
    }
    const savedModeLine = el("p", {
      className: "status-line",
      textContent: `Saved mode: ${currentSettings.mode ?? "local"} · SSH: ${currentSettings.ssh_target || "(none)"} · direct WS: ${currentSettings.remote_direct ? "yes" : "no"} · TLS: ${currentSettings.use_tls ? "yes" : "no"}`,
    });
    const refreshStatus = async () => {
      let settings = currentSettings;
      try {
        settings = await invoke("get_connection_settings");
        savedModeLine.textContent = `Saved mode: ${settings.mode ?? "local"} · SSH: ${settings.ssh_target || "(none)"} · direct WS: ${settings.remote_direct ? "yes" : "no"} · TLS: ${settings.use_tls ? "yes" : "no"}`;
      } catch {
        /* keep initial savedModeLine */
      }
      let connectionMessage = "";
      try {
        connectionMessage = await invoke("get_connection_status");
        statusProminent.textContent =
          connectionMessage || "No gateway connection status yet";
      } catch {
        statusProminent.textContent = "Status unavailable";
      }
      await refreshConnBar();
      try {
        const active = await invoke("get_remote_tunnel_active");
        tunnelLine.textContent = tunnelStatusLine(
          settings,
          active,
          connectionMessage,
        );
        tunnelLine.className = tunnelLineClassName(settings, active, connectionMessage);
      } catch {
        tunnelLine.textContent = tunnelStatusLine(settings, false, connectionMessage);
        tunnelLine.className = "status-line";
      }
    };
    await refreshStatus();
    await refreshPairingBox(pairingBox);
    startPairingRefresh(pairingBox);
    panel.append(
      statusProminent,
      tunnelLine,
      deviceLine,
      savedModeLine,
      el("fieldset", {}, [
        el("legend", { textContent: "Connection" }),
        el("label", { textContent: "Mode" }),
        mode,
        el("label", { textContent: "Host" }),
        host,
        el("label", { textContent: "Port" }),
        port,
        el("label", { textContent: "Gateway token" }),
        token,
        el("label", { textContent: "SSH target (remote)" }),
        sshTarget,
        el("label", { textContent: "SSH identity file (remote)" }),
        sshIdentity,
        el("label", { className: "checkbox-row", htmlFor: "remote_direct" }, [
          remoteDirect,
          el("span", {
            textContent:
              "Connect directly to host/port (no SSH tunnel; gateway.remote.url / direct WS)",
          }),
        ]),
        el("label", { className: "checkbox-row", htmlFor: "use_tls" }, [
          useTls,
          el("span", { textContent: "Use TLS (wss/https) for gateway WebSocket and Control UI" }),
        ]),
        wsUrlHint,
        el("button", {
          className: "action",
          textContent: "Save",
          onclick: async () => {
            try {
              const latest = await invoke("get_connection_settings");
              await invoke("save_connection_settings", {
                settings: {
                  ...latest,
                  mode: mode.value,
                  host: host.value || null,
                  port: Number(port.value),
                  token: token.value || null,
                  ssh_target: sshTarget.value || null,
                  ssh_identity: sshIdentity.value || null,
                  remote_direct: remoteDirect.checked,
                  use_tls: useTls.checked,
                },
              });
              showToast("Connection saved; reconnecting operator and node");
              await run("get_gateway_status");
              await refreshStatus();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Reconnect gateway",
          onclick: async () => {
            try {
              const msg = await invoke("reconnect_gateway_cmd");
              showToast(msg || "Reconnected");
              await refreshStatus();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Test operator connection",
          onclick: async () => {
            try {
              await invoke("operator_connect");
              const health = await invoke("operator_health");
              showToast("Operator connected");
              const log = document.getElementById("log");
              if (log) log.textContent = health;
              await refreshStatus();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Discover gateways (mDNS)",
          onclick: async () => {
            try {
              const raw = await invoke("discover_gateways");
              const list = JSON.parse(raw);
              discoveryBox.replaceChildren(
                ...(Array.isArray(list) && list.length
                  ? [
                      el("p", {
                        className: "help",
                        textContent: `Found ${list.length} gateway(s) (deduped by host:port).`,
                      }),
                      ...list.map((gw) => {
                        const label = gw.name ?? "gateway";
                        const direct = isDirectLanHost(gw.host);
                        const hostPort = `${gw.host}:${gw.port}`;
                        return el("div", { className: "discovery-row" }, [
                          el("span", {
                            textContent: `${label} — ${hostPort}${direct ? " · LAN" : ""}`,
                          }),
                          el("button", {
                            className: "action",
                            textContent: "Use & save",
                            onclick: async () => {
                              host.value = gw.host;
                              port.value = String(gw.port);
                              useTls.checked = Number(gw.port) === 443;
                              refreshWsUrlHint();
                              mode.value = direct ? "local" : "remote";
                              if (direct) remoteDirect.checked = true;
                              try {
                                const latest = await invoke("get_connection_settings");
                                await invoke("save_connection_settings", {
                                  settings: {
                                    ...latest,
                                    mode: mode.value,
                                    host: host.value || null,
                                    port: Number(port.value),
                                    remote_direct: remoteDirect.checked,
                                    use_tls: useTls.checked,
                                  },
                                });
                                showToast(
                                  direct
                                    ? `Saved ${hostPort} (local / direct)`
                                    : `Saved ${hostPort} (remote — set SSH target if needed)`,
                                );
                                await refreshStatus();
                              } catch (err) {
                                showToast(String(err), true);
                              }
                            },
                          }),
                        ]);
                      }),
                    ]
                  : [el("p", { textContent: "No gateways found on the LAN." })]),
              );
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        discoveryBox,
        el("button", {
          className: "action",
          textContent: "Connect operator (WS)",
          onclick: () => run("operator_connect"),
        }),
        el("button", {
          className: "action",
          textContent: "Start SSH tunnel (remote)",
          onclick: async () => {
            await run("start_remote_tunnel");
            await refreshStatus();
          },
        }),
        el("button", {
          className: "action",
          textContent: "Restart SSH tunnel",
          onclick: async () => {
            try {
              await run("stop_remote_tunnel");
              await run("start_remote_tunnel");
              showToast("SSH tunnel restarted");
            } catch (err) {
              showToast(String(err), true);
            }
            await refreshStatus();
          },
        }),
        el("button", {
          className: "action",
          textContent: "Stop SSH tunnel",
          onclick: async () => {
            await run("stop_remote_tunnel");
            await refreshStatus();
          },
        }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Pairing approvals" }),
        el("p", {
          className: "help",
          textContent:
            "Desktop notifications for new pairing requests may appear when the app is in the tray. Clicking a notification cannot reliably open this window on all Linux desktops — open Settings from the tray menu to approve pending requests.",
        }),
        el("button", {
          className: "action",
          textContent: "Refresh pending",
          onclick: () => refreshPairingBox(pairingBox),
        }),
        pairingBox,
      ]),
      el("pre", { id: "log", className: "log" }),
    );
  }

  if (activeTab === "exec") {
    const approvals = await invoke("get_exec_approvals");
    const pendingBox = el("div", { id: "exec-pending-box", className: "exec-pending-box" });
    const socketLine = el("p", { className: "status-line", textContent: "Loading exec socket…" });
    try {
      const socketInfo = await invoke("get_exec_socket_info");
      socketLine.textContent =
        typeof socketInfo === "string"
          ? socketInfo
          : JSON.stringify(socketInfo, null, 2);
    } catch (err) {
      socketLine.textContent = String(err);
    }
    const area = el("textarea", {
      rows: 12,
      value: JSON.stringify(approvals, null, 2),
    });
    panel.append(
      el("fieldset", {}, [
        el("legend", { textContent: "Pending exec prompts" }),
        el("p", {
          className: "help",
          textContent:
            "Gateway CLI exec requests appear here while the node is connected. Desktop notifications also fire; use Allow/Deny below or the system confirm dialog.",
        }),
        el("button", {
          className: "action",
          textContent: "Refresh pending",
          onclick: () => refreshExecPendingBox(pendingBox),
        }),
        pendingBox,
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Exec approvals (~/.openclaw/exec-approvals.json)" }),
        el("p", {
          className: "help",
          textContent:
            "While the node is connected, the gateway CLI can prompt via the Unix socket below.",
        }),
        socketLine,
        area,
        el("button", {
          className: "action",
          textContent: "Save",
          onclick: async () => {
            const parsed = JSON.parse(area.value);
            await invoke("save_exec_approvals", { file: parsed });
            showToast("Exec approvals saved");
          },
        }),
      ]),
    );
    await refreshExecPendingBox(pendingBox);
  }

  if (activeTab === "debug") {
    const bridgeStatusPre = el("pre", {
      className: "log bridge-status",
      textContent: "Click Bridge status to load JSON.",
    });
    const healthLine = el("p", {
      id: "debug-health-line",
      className: "status-line",
      textContent: "Loading connection health…",
    });
    const refreshDebugHealth = async () => {
      try {
        const raw = await invoke("get_connection_health");
        healthLine.textContent = raw;
        const health = parseJsonSafe(raw);
        if (health && typeof health === "object") {
          const op = health.operatorConnected ? "operator ✓" : "operator ✗";
          const node = health.nodeConnected ? "node ✓" : "node ✗";
          const tunnel = health.sshTunnelActive ? " · tunnel ✓" : "";
          const probe = health.operatorHealthOk ? " · health probe OK" : "";
          const msg = health.message ? ` — ${health.message}` : "";
          healthLine.textContent = `${op} · ${node}${tunnel}${probe}${msg}`;
        }
      } catch (err) {
        healthLine.textContent = String(err);
      }
    };
    await refreshDebugHealth();
    panel.append(
      el("fieldset", {}, [
        el("legend", { textContent: "Gateway & operator" }),
        healthLine,
        el("button", {
          className: "action",
          textContent: "Refresh connection health",
          onclick: async () => {
            await refreshDebugHealth();
            await refreshConnBar();
          },
        }),
        el("button", {
          className: "action",
          textContent: "Gateway status (JSON)",
          onclick: () => run("get_gateway_status"),
        }),
        el("button", {
          className: "action",
          textContent: "Operator health",
          onclick: () => run("operator_health"),
        }),
        el("button", {
          className: "action",
          textContent: "Open test canvas (A2UI)",
          onclick: async () => {
            try {
              await invoke("open_test_canvas");
              showToast("Canvas window opened (bundle via pnpm canvas:a2ui:bundle)");
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Restart gateway service",
          onclick: async () => {
            try {
              await invoke("restart_gateway_service_cmd");
              showToast("Gateway restart requested");
              await refreshConnBar();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Stop gateway service",
          onclick: async () => {
            try {
              await invoke("stop_gateway_service_cmd");
              showToast("Gateway stop requested");
              await refreshConnBar();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Reconnect gateway",
          onclick: async () => {
            try {
              const msg = await invoke("reconnect_gateway_cmd");
              showToast(msg || "Reconnected");
              await refreshConnBar();
              await refreshDebugHealth();
              run("operator_health");
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "Connection health (JSON)",
          onclick: async () => {
            try {
              const raw = await invoke("get_connection_health");
              const log = document.getElementById("log");
              if (log) log.textContent = raw;
              await refreshDebugHealth();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("pre", { id: "log", className: "log" }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Automation bridge" }),
        el("p", {
          className: "help",
          textContent: "Peekaboo parity stub — socket path reserved; host not enabled yet.",
        }),
        el("button", {
          className: "action",
          textContent: "Bridge status",
          onclick: async () => {
            try {
              bridgeStatusPre.textContent = await invoke("get_automation_bridge_status");
            } catch (err) {
              bridgeStatusPre.textContent = String(err);
            }
          },
        }),
        bridgeStatusPre,
      ]),
    );
    run("operator_health");
  }

  if (activeTab === "about") {
    const gatewayVersionAbout = el("p", {
      className: "status-line",
      textContent: "Checking gateway version…",
    });
    try {
      const verRaw = await invoke("get_gateway_version_info");
      gatewayVersionAbout.textContent = formatGatewayVersionSummary(parseJsonSafe(verRaw));
    } catch {
      gatewayVersionAbout.textContent = "Gateway version: unavailable";
    }
    const aboutPre = el("pre", { className: "log", textContent: "Loading…" });
    try {
      aboutPre.textContent = await invoke("get_app_build_info");
    } catch (err) {
      aboutPre.textContent = String(err);
    }
    panel.append(
      el("p", {
        textContent: "OpenClaw Linux companion — macOS parity program. See apps/linux/PARITY.md",
      }),
      gatewayVersionAbout,
      aboutPre,
    );
  }

  if (activeTab in OPERATOR_TABS) {
    await mountOperatorTab(panel, activeTab, OPERATOR_TABS[activeTab]);
  }

  if (activeTab === "instances") {
    await mountOperatorTab(panel, "instances", "operator_instances");
  }

  if (activeTab === "voice") {
    const voice = await invoke("get_voice_settings");
    const enabled = el("input", { type: "checkbox", id: "voice_enabled" });
    enabled.checked = Boolean(voice.enabled);
    const talkEnabled = el("input", { type: "checkbox", id: "talk_enabled" });
    talkEnabled.checked = Boolean(voice.talk_enabled);
    const locale = el("input", { id: "voice_locale", value: voice.locale ?? "en-US" });
    const phrases = el("textarea", {
      id: "voice_phrases",
      rows: 4,
      value: (voice.phrases ?? ["open claw"]).join("\n"),
    });
    const voiceDiagPre = el("pre", { className: "log", textContent: "Loading…" });
    const talkConfigPre = el("pre", {
      className: "log",
      textContent: "Enable talk mode and connect operator to load gateway talk.config.",
    });
    const refreshVoiceDiag = async () => {
      try {
        voiceDiagPre.textContent = await invoke("get_voice_diagnostics");
      } catch (err) {
        voiceDiagPre.textContent = String(err);
      }
    };
    await refreshVoiceDiag();
    panel.append(
      el("fieldset", {}, [
        el("legend", { textContent: "Voice wake (not implemented on Linux)" }),
        el("p", {
          className: "help",
          textContent:
            "Saves wake phrases to ~/.openclaw/linux-app-settings.json and syncs voicewake.set when the operator is connected. Continuous wake-word listening is not available on Linux yet.",
        }),
        el("label", { textContent: "Persist wake settings (no local listener)" }),
        enabled,
        el("label", { textContent: "Locale" }),
        locale,
        el("label", { textContent: "Wake phrases (one per line)" }),
        phrases,
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Push-to-talk (implemented)" }),
        el("p", {
          className: "help",
          textContent:
            "Records WAV via pw-record or parecord. Gateway STT is not wired yet — stop returns audioBase64 with an empty transcript.",
        }),
        el("button", {
          className: "action",
          textContent: "PTT start",
          onclick: async () => {
            try {
              await invoke("voice_ptt_start");
              showToast("PTT recording…");
              await refreshVoiceDiag();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "PTT cancel",
          onclick: async () => {
            try {
              await invoke("voice_ptt_cancel");
              showToast("PTT cancelled");
              await refreshVoiceDiag();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "PTT stop",
          onclick: async () => {
            try {
              const result = await invoke("voice_ptt_stop");
              const hasAudio = result?.audioBase64?.length > 0;
              const transcript = result?.transcript?.trim();
              showToast(
                transcript
                  ? `PTT stopped: ${transcript}`
                  : hasAudio
                    ? "PTT stopped (audio captured)"
                    : "PTT stopped",
              );
              await refreshVoiceDiag();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Talk mode (partial — use WebChat)" }),
        el("p", {
          className: "help",
          textContent:
            "Talk mode flags and gateway talk.config are available; managed-room audio is not implemented on Linux. Use WebChat or Dashboard chat for conversation UI.",
        }),
        el("label", { textContent: "Enable talk mode (node caps + gateway)" }),
        talkEnabled,
        el("button", {
          className: "action",
          textContent: "Open WebChat",
          onclick: () => invoke("open_webchat").catch((err) => showToast(String(err), true)),
        }),
        el("button", {
          className: "action",
          textContent: "Open chat in Dashboard",
          onclick: () =>
            invoke("open_dashboard_chat").catch((err) => showToast(String(err), true)),
        }),
        el("button", {
          className: "action",
          textContent: "Load gateway talk.config",
          onclick: async () => {
            try {
              talkConfigPre.textContent = await invoke("operator_talk_config");
            } catch (err) {
              talkConfigPre.textContent = String(err);
            }
          },
        }),
        talkConfigPre,
        el("button", {
          className: "action",
          textContent: "Save voice settings",
          onclick: async () => {
            try {
              await invoke("save_voice_settings", {
                config: {
                  enabled: enabled.checked,
                  talk_enabled: talkEnabled.checked,
                  locale: locale.value,
                  phrases: phrases.value
                    .split("\n")
                    .map((s) => s.trim())
                    .filter(Boolean),
                },
              });
              showToast("Voice settings saved (~/.openclaw/linux-app-settings.json)");
              await refreshVoiceDiag();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Voice diagnostics" }),
        el("button", {
          className: "action",
          textContent: "Refresh voice diagnostics",
          onclick: refreshVoiceDiag,
        }),
        voiceDiagPre,
      ]),
    );
  }

  if (activeTab === "permissions") {
    const settings = await invoke("get_connection_settings");
    const savePermissionFlags = async (patch) => {
      try {
        const current = await invoke("get_connection_settings");
        await invoke("save_connection_settings", {
          settings: { ...current, ...patch },
        });
        showToast("Permission toggles saved; node caps update on next reconnect");
        await refreshCaptureDiag();
      } catch (err) {
        showToast(String(err), true);
      }
    };
    const cameraEnabled = el("input", { type: "checkbox", id: "camera_enabled" });
    cameraEnabled.checked = settings.camera_enabled !== false;
    cameraEnabled.addEventListener("change", () =>
      savePermissionFlags({ camera_enabled: cameraEnabled.checked }),
    );
    const screenEnabled = el("input", { type: "checkbox", id: "screen_enabled" });
    screenEnabled.checked = settings.screen_enabled !== false;
    screenEnabled.addEventListener("change", () =>
      savePermissionFlags({ screen_enabled: screenEnabled.checked }),
    );
    const locationEnabled = el("input", { type: "checkbox", id: "location_enabled" });
    locationEnabled.checked = settings.location_enabled !== false;
    locationEnabled.addEventListener("change", () =>
      savePermissionFlags({ location_enabled: locationEnabled.checked }),
    );
    const diagPre = el("pre", { className: "log", textContent: "Loading…" });
    const refreshCaptureDiag = async () => {
      try {
        diagPre.textContent = await invoke("get_capture_diagnostics");
      } catch (err) {
        diagPre.textContent = String(err);
      }
    };
    panel.append(
      el("p", {
        className: "help",
        textContent:
          "Toggles control which node capabilities are advertised on the next operator/node reconnect. They do not grant OS permissions by themselves.",
      }),
      el("fieldset", {}, [
        el("legend", { textContent: "Node capability advertisement" }),
        el("label", { className: "checkbox-row", htmlFor: "camera_enabled" }, [
          cameraEnabled,
          el("span", { textContent: "Advertise camera.* commands" }),
        ]),
        el("label", { className: "checkbox-row", htmlFor: "screen_enabled" }, [
          screenEnabled,
          el("span", { textContent: "Advertise screen.* commands" }),
        ]),
        el("label", { className: "checkbox-row", htmlFor: "location_enabled" }, [
          locationEnabled,
          el("span", { textContent: "Advertise location.get" }),
        ]),
      ]),
      el("p", {
        textContent:
          "Screen capture uses xdg-desktop-portal (Screenshot) with grim/gnome-screenshot fallbacks. Grant portal access when prompted.",
      }),
      el("button", {
        className: "action",
        textContent: "Refresh capability probe",
        onclick: refreshCaptureDiag,
      }),
      diagPre,
    );
    await refreshCaptureDiag();
  }
}

async function handlePairingRequest(payload) {
  const log = document.getElementById("log");
  const kind = payload?.kind ?? "device";
  const req = payload?.request ?? payload;
  const requestId = req?.requestId;
  if (!requestId) {
    if (log) log.textContent = JSON.stringify(payload, null, 2);
    return;
  }
  const name =
    req.displayName ?? req.deviceId ?? req.nodeId ?? "unknown device";
  const approve = confirm(`Approve ${kind} pairing for ${name}?`);
  try {
    if (approve) {
      await invoke(kind === "node" ? "pairing_approve_node" : "pairing_approve_device", {
        requestId,
      });
    } else {
      await invoke(kind === "node" ? "pairing_reject_node" : "pairing_reject_device", {
        requestId,
      });
    }
  } catch (err) {
    if (log) log.textContent = String(err);
  }
}

async function run(cmd, args = {}) {
  const log = document.getElementById("log");
  try {
    const out = await invoke(cmd, args);
    if (log) log.textContent = typeof out === "string" ? out : JSON.stringify(out, null, 2);
  } catch (err) {
    const message = String(err);
    if (log) log.textContent = message;
    showToast(message, true);
  }
}

listen("tray-open-dashboard", () => invoke("open_dashboard"));
listen("tray-open-settings", () => invoke("open_settings"));
listen("settings-tab", (e) => {
  const tab = String(e.payload ?? "").trim();
  if (TABS.some((t) => t.id === tab)) {
    activeTab = tab;
    renderTabs();
    void renderPanel();
  }
});
listen("deep-link", (e) => {
  const log = document.getElementById("log");
  if (log) log.textContent = `Deep link: ${e.payload}`;
});
listen("canvas-present", (e) => {
  invoke("open_canvas_window", { sessionId: e.payload });
});
listen("pairing-request", async (e) => {
  activeTab = "connection";
  renderTabs();
  await renderPanel();
  const pairingBox = document.getElementById("pairing-box");
  await refreshPairingBox(pairingBox);
  await handlePairingRequest(e.payload);
});
listen("deep-link-agent", async (e) => {
  const payload = e.payload;
  const message =
    typeof payload === "object" && payload?.message != null
      ? String(payload.message)
      : String(payload ?? "");
  const session =
    typeof payload === "object" && payload?.session != null
      ? String(payload.session)
      : "";
  if (message.trim()) {
    showToast(`Agent deep link: ${message.trim().slice(0, 80)}${message.length > 80 ? "…" : ""}`);
  }
  try {
    if (session.trim()) {
      await invoke("open_webchat", { session: session.trim() });
    } else {
      await invoke("open_dashboard");
    }
  } catch (err) {
    showToast(String(err), true);
  }
});
listen("connection-error", (e) => {
  const log = document.getElementById("log");
  const message = String(e.payload ?? "");
  showToast(message, true);
  applyConnectionStatusMessage(message || "Connection error");
  if (log) log.textContent = message;
  if (activeTab === "connection") {
    void renderPanel();
  }
});
listen("gateway-connection-status", async (e) => {
  const message = String(e.payload ?? "");
  applyConnectionStatusMessage(message);
  await refreshConnBar();
  if (activeTab === "connection") {
    const tunnelLine = document.getElementById("tunnel-status");
    if (tunnelLine) {
      try {
        const settings = await invoke("get_connection_settings");
        const active = await invoke("get_remote_tunnel_active");
        tunnelLine.textContent = tunnelStatusLine(settings, active, message);
        tunnelLine.className = tunnelLineClassName(settings, active, message);
      } catch {
        /* panel refresh below is enough */
      }
    }
  }
});
listen("exec-approval-request", async (e) => {
  const payload = e.payload;
  highlightExecId = payload?.id ?? null;
  activeTab = "exec";
  renderTabs();
  await renderPanel();
  try {
    await invoke("open_settings");
  } catch {
    /* window may already be visible */
  }
  const cmd = payload?.command ?? "command";
  const cwd = payload?.cwd ? `\n\ncwd: ${payload.cwd}` : "";
  const ok = confirm(`Allow exec?\n\n${cmd}${cwd}`);
  try {
    await invoke("resolve_exec_approval", {
      id: payload?.id,
      decision: ok ? "allow-once" : "deny",
    });
    highlightExecId = null;
  } catch (err) {
    showToast(String(err), true);
  }
});

renderTabs();
renderPanel();
refreshConnBar();
