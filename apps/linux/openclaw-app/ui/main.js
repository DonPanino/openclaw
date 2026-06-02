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

const OPERATOR_TABS = {
  channels: "operator_channels_status",
  skills: "operator_skills_status",
  cron: "operator_cron_list",
  sessions: "operator_sessions_list",
  config: "operator_config_get",
};

function parseJsonSafe(raw) {
  if (typeof raw !== "string") return raw;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

async function refreshConnBar() {
  const bar = document.getElementById("conn-bar");
  if (!bar) return;
  try {
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

function renderOperatorSummary(tab, data) {
  const wrap = el("div", { className: "operator-summary" });
  if (!data || typeof data !== "object") {
    wrap.append(el("p", { textContent: "No structured data." }));
    return wrap;
  }
  if (tab === "channels") {
    const channels =
      data.channels ?? data.items ?? (Array.isArray(data) ? data : Object.values(data));
    const list = Array.isArray(channels) ? channels : [];
    wrap.append(
      summaryList(
        "Channels",
        list,
        (c) =>
          `${c.id ?? c.channelId ?? c.name ?? "?"} — ${c.status ?? c.state ?? c.enabled === false ? "off" : "on"}`,
      ),
    );
    return wrap;
  }
  if (tab === "cron") {
    const jobs = data.jobs ?? data.items ?? (Array.isArray(data) ? data : []);
    wrap.append(
      summaryList(
        "Cron jobs",
        Array.isArray(jobs) ? jobs : [],
        (j) => `${j.name ?? j.id ?? "?"} — ${j.enabled === false ? "disabled" : "enabled"}`,
      ),
    );
    return wrap;
  }
  if (tab === "instances") {
    const nodes = data.nodes ?? data.items ?? (Array.isArray(data) ? data : []);
    wrap.append(
      summaryList(
        "Nodes",
        Array.isArray(nodes) ? nodes : [],
        (n) => `${n.displayName ?? n.nodeId ?? n.id ?? "?"} — ${n.status ?? n.connected ?? ""}`.trim(),
      ),
    );
    return wrap;
  }
  if (tab === "sessions") {
    const sessions = data.sessions ?? data.items ?? (Array.isArray(data) ? data : []);
    wrap.append(
      summaryList(
        "Sessions",
        Array.isArray(sessions) ? sessions : [],
        (s) => `${s.key ?? s.sessionKey ?? s.id ?? "?"} — ${s.label ?? s.title ?? ""}`.trim(),
      ),
    );
    return wrap;
  }
  if (tab === "skills") {
    const skills = data.skills ?? data.items ?? (Array.isArray(data) ? data : []);
    wrap.append(
      summaryList(
        "Skills",
        Array.isArray(skills) ? skills : [],
        (s) => `${s.name ?? s.id ?? "?"} — ${s.status ?? s.state ?? ""}`.trim(),
      ),
    );
    return wrap;
  }
  wrap.append(el("p", { textContent: "Loaded. See raw JSON below." }));
  return wrap;
}

async function mountOperatorTab(panel, tab, cmd) {
  const summaryHost = el("div");
  const log = el("pre", { className: "log", textContent: "Loading…" });
  panel.append(
    el("button", {
      className: "action",
      textContent: "Reload",
      onclick: () => loadOperatorTab(panel, tab, cmd, summaryHost, log),
    }),
    summaryHost,
    log,
  );
  await loadOperatorTab(panel, tab, cmd, summaryHost, log);
}

async function loadOperatorTab(panel, tab, cmd, summaryHost, log) {
  log.textContent = "Connecting operator…";
  summaryHost.replaceChildren();
  if (!(await ensureOperator())) {
    log.textContent = "Operator not connected.";
    return;
  }
  try {
    const raw = await invoke(cmd);
    const data = parseJsonSafe(raw);
    summaryHost.replaceChildren(renderOperatorSummary(tab, data));
    log.textContent = typeof raw === "string" ? raw : JSON.stringify(raw, null, 2);
  } catch (err) {
    log.textContent = String(err);
    showToast(String(err), true);
  }
}

function renderPairingPending(kind, raw, box) {
  const data = parseJsonSafe(raw);
  const pending = data?.pending ?? [];
  if (!pending.length) {
    box.append(el("p", { textContent: `No pending ${kind} pairing requests.` }));
    return;
  }
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
              await invoke(approveCmd, { requestId });
              showToast(`Approved ${name}`);
            },
          }),
          el("button", {
            textContent: "Reject",
            onclick: async () => {
              await invoke(rejectCmd, { requestId });
              showToast(`Rejected ${name}`);
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
    panel.append(
      gatewayHint,
      el("label", { className: "checkbox-row", htmlFor: "gateway_autostart" }, [
        autostartToggle,
        el("span", {
          textContent:
            "Start gateway when OpenClaw launches (local mode; skips start if already running)",
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
          onclick: () => invoke("open_webchat").catch((err) => showToast(String(err), true)),
        }),
        el("button", {
          className: "action",
          textContent: "Install CLI (npm global)",
          onclick: () => run("install_cli"),
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
    const discoveryBox = el("div", { className: "discovery-box" });
    const pairingBox = el("div", { id: "pairing-box", className: "pairing-box" });
    const statusLine = el("p", { id: "connection-status", className: "status-line", textContent: "" });
    const refreshStatus = async () => {
      try {
        const status = await invoke("get_connection_status");
        statusLine.textContent = status ? `Status: ${status}` : "";
      } catch {
        statusLine.textContent = "";
      }
    };
    await refreshStatus();
    panel.append(
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
        statusLine,
        el("button", {
          className: "action",
          textContent: "Save",
          onclick: async () => {
            try {
              await invoke("save_connection_settings", {
                settings: {
                  ...currentSettings,
                  mode: mode.value,
                  host: host.value || null,
                  port: Number(port.value),
                  token: token.value || null,
                  ssh_target: sshTarget.value || null,
                  ssh_identity: sshIdentity.value || null,
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
          textContent: "Discover gateways (mDNS)",
          onclick: async () => {
            try {
              const raw = await invoke("discover_gateways");
              const list = JSON.parse(raw);
              discoveryBox.replaceChildren(
                ...(Array.isArray(list) && list.length
                  ? list.map((gw) =>
                      el("div", { className: "discovery-row" }, [
                        el("span", {
                          textContent: `${gw.name ?? "gateway"} — ${gw.host}:${gw.port}`,
                        }),
                        el("button", {
                          className: "action",
                          textContent: "Use",
                          onclick: async () => {
                            host.value = gw.host;
                            port.value = String(gw.port);
                            mode.value = "remote";
                            showToast(`Set host to ${gw.host}:${gw.port} (remote mode)`);
                          },
                        }),
                      ]),
                    )
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
          onclick: () => run("start_remote_tunnel"),
        }),
        el("button", {
          className: "action",
          textContent: "Stop SSH tunnel",
          onclick: () => run("stop_remote_tunnel"),
        }),
      ]),
      el("fieldset", {}, [
        el("legend", { textContent: "Pairing approvals" }),
        el("button", {
          className: "action",
          textContent: "Refresh pending",
          onclick: async () => {
            pairingBox.replaceChildren(el("p", { textContent: "Loading…" }));
            if (!(await ensureOperator())) return;
            try {
              const devices = await invoke("pairing_list_devices");
              const nodes = await invoke("pairing_list_nodes");
              pairingBox.replaceChildren();
              renderPairingPending("device", devices, pairingBox);
              renderPairingPending("node", nodes, pairingBox);
            } catch (err) {
              pairingBox.replaceChildren(el("pre", { textContent: String(err) }));
            }
          },
        }),
        pairingBox,
      ]),
      el("pre", { id: "log", className: "log" }),
    );
  }

  if (activeTab === "exec") {
    const approvals = await invoke("get_exec_approvals");
    const area = el("textarea", {
      rows: 12,
      value: JSON.stringify(approvals, null, 2),
    });
    panel.append(
      el("fieldset", {}, [
        el("legend", { textContent: "Exec approvals (~/.openclaw/exec-approvals.json)" }),
        area,
        el("button", {
          className: "action",
          textContent: "Save",
          onclick: async () => {
            const parsed = JSON.parse(area.value);
            await invoke("save_exec_approvals", { file: parsed });
          },
        }),
      ]),
    );
  }

  if (activeTab === "debug") {
    panel.append(
      el("fieldset", {}, [
        el("legend", { textContent: "Gateway & operator" }),
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
        el("pre", { id: "log", className: "log" }),
      ]),
    );
    run("operator_health");
  }

  if (activeTab === "about") {
    panel.append(
      el("p", {
        textContent: "OpenClaw Linux companion — macOS parity program. See apps/linux/PARITY.md",
      }),
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
        el("legend", { textContent: "Voice & Talk" }),
        el("label", { textContent: "Enable voice wake (stub)" }),
        enabled,
        el("label", { textContent: "Enable talk mode (stub)" }),
        talkEnabled,
        el("label", { textContent: "Locale" }),
        locale,
        el("label", { textContent: "Wake phrases (one per line)" }),
        phrases,
        el("button", {
          className: "action",
          textContent: "PTT start (stub)",
          onclick: async () => {
            try {
              await invoke("voice_ptt_start");
              showToast("PTT recording (stub — no audio yet)");
              await refreshVoiceDiag();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
        el("button", {
          className: "action",
          textContent: "PTT stop (stub)",
          onclick: async () => {
            try {
              await invoke("voice_ptt_stop");
              showToast("PTT stopped");
              await refreshVoiceDiag();
            } catch (err) {
              showToast(String(err), true);
            }
          },
        }),
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
listen("deep-link", (e) => {
  const log = document.getElementById("log");
  if (log) log.textContent = `Deep link: ${e.payload}`;
});
listen("canvas-present", (e) => {
  invoke("open_canvas_window", { sessionId: e.payload });
});
listen("pairing-request", (e) => {
  handlePairingRequest(e.payload);
});
listen("connection-error", (e) => {
  const log = document.getElementById("log");
  const status = document.getElementById("connection-status");
  const message = String(e.payload ?? "");
  showToast(message, true);
  if (status) status.textContent = message ? `Status: ${message}` : "";
  if (log) log.textContent = message;
  const bar = document.getElementById("conn-bar");
  if (bar) bar.textContent = message;
});
listen("gateway-connection-status", (e) => {
  const bar = document.getElementById("conn-bar");
  if (bar) bar.textContent = String(e.payload ?? "");
});
listen("exec-approval-request", async (e) => {
  const payload = e.payload;
  const cmd = payload?.command ?? "command";
  const cwd = payload?.cwd ? `\n\ncwd: ${payload.cwd}` : "";
  const ok = confirm(`Allow exec?\n\n${cmd}${cwd}`);
  try {
    await invoke("resolve_exec_approval", {
      id: payload?.id,
      decision: ok ? "allow-once" : "deny",
    });
  } catch (err) {
    showToast(String(err), true);
  }
});

renderTabs();
renderPanel();
refreshConnBar();
