function getInvoke() {
  if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === "function") {
    return window.__TAURI__.core.invoke;
  }
  if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === "function") {
    return window.__TAURI_INTERNALS__.invoke;
  }
  return null;
}

const invoke = async (cmd, args = {}) => {
  const invoker = getInvoke();
  if (!invoker) {
    throw new Error("Tauri IPC bridge indisponível. Rode dentro da janela do 0spof.");
  }
  return await invoker(cmd, args);
};

function formatDate(dateStr) {
  if (!dateStr) return "";
  try {
    const dt = new Date(dateStr);
    if (isNaN(dt.getTime())) return dateStr;
    return dt.toLocaleString("pt-BR", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch (_) {
    return dateStr;
  }
}

function escapeHtml(str) {
  return String(str ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let val = bytes;
  let idx = 0;
  while (val >= 1024 && idx < units.length - 1) {
    val /= 1024;
    idx++;
  }
  const formatted = idx === 0 ? `${Math.round(val)}` : val.toFixed(val < 10 ? 1 : 0).replace(".", ",");
  return `${formatted} ${units[idx]}`;
}

// ---------------------------------------------------------------------------
// Estado compartilhado da interface
// ---------------------------------------------------------------------------
const DAY_MS = 86400000;

const state = {
  deadlines: [],
  deadlinesError: null,
  outbox: [],
  outboxError: null,
  session: { is_authenticated: false, base_url: null },
  // id da atividade -> atividade + dados da disciplina (montado a partir de get_course_details)
  activities: new Map(),
  syncError: null,
  syncReport: null,
  storage: null,
  deadlineView: "pending",
  selectedOutboxId: null,
  nextDue: null,
};

const pad2 = (n) => String(n).padStart(2, "0");
const startOfDay = (d) => new Date(d.getFullYear(), d.getMonth(), d.getDate());

function daysFromToday(iso) {
  return Math.round((startOfDay(new Date(iso)) - startOfDay(new Date())) / DAY_MS);
}

function weekdayShort(date) {
  return date.toLocaleDateString("pt-BR", { weekday: "short" }).replace(".", "");
}

// "seg, 30/11 às 23:59"
function formatDueLong(iso) {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return String(iso || "");
  return `${weekdayShort(d)}, ${pad2(d.getDate())}/${pad2(d.getMonth() + 1)} às ${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

function formatCountdown(iso) {
  const ms = new Date(iso).getTime() - Date.now();
  if (isNaN(ms)) return { big: "—", small: "" };
  if (ms <= 0) return { big: "Encerrado", small: "prazo vencido" };
  const days = Math.floor(ms / DAY_MS);
  const hours = Math.floor((ms % DAY_MS) / 3600000);
  const mins = Math.floor((ms % 3600000) / 60000);
  if (days > 0) return { big: `${days}d ${pad2(hours)}h`, small: "restantes" };
  if (hours > 0) return { big: `${hours}h ${pad2(mins)}m`, small: "restantes" };
  return { big: `${Math.max(mins, 1)} min`, small: "restantes" };
}

function relativeTime(iso) {
  if (!iso) return "";
  const ms = Date.now() - new Date(iso).getTime();
  if (isNaN(ms)) return "";
  const mins = Math.round(ms / 60000);
  if (mins < 1) return "agora há pouco";
  if (mins < 60) return `há ${mins} min`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `há ${hours} h`;
  const days = Math.round(hours / 24);
  return `há ${days} ${days === 1 ? "dia" : "dias"}`;
}

function dateEyebrow() {
  const txt = new Date().toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "long" });
  return txt.charAt(0).toUpperCase() + txt.slice(1);
}

function hostOf(url) {
  try {
    return new URL(url).host;
  } catch (_) {
    return url || "";
  }
}

function lastSyncIso() {
  let best = null;
  for (const c of cachedCourses) {
    if (c.last_synced_at && (!best || new Date(c.last_synced_at) > new Date(best))) best = c.last_synced_at;
  }
  return best;
}

const ICON_CLOCK = `<svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>`;
const ICON_CHECK = (n = 12) => `<svg viewBox="0 0 24 24" width="${n}" height="${n}" stroke="currentColor" fill="none" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>`;
const ICON_EXTERNAL = `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>`;

// ---------------------------------------------------------------------------
// Navegação de abas
// ---------------------------------------------------------------------------
const navItems = document.querySelectorAll(".nav-item[data-tab]");
const tabPanes = document.querySelectorAll(".tab-pane");
const tabTitle = document.getElementById("tab-title");
const tabEyebrow = document.getElementById("tab-eyebrow");

let currentTab = "dashboard";

const TAB_TITLES = {
  dashboard: "Visão geral",
  courses: "Disciplinas",
  outbox: "Fila de envio",
  settings: "Conexão e dados",
};

function tabEyebrowText(tab) {
  if (tab === "dashboard") return dateEyebrow();
  if (tab === "courses") {
    const n = cachedCourses.length;
    return `Base local · ${n} ${n === 1 ? "disciplina" : "disciplinas"}`;
  }
  if (tab === "outbox") return "Envios protegidos";
  if (tab === "settings") return "Acesso institucional e dados locais";
  return "0spof / workspace";
}

function updateHeading() {
  tabTitle.innerText = TAB_TITLES[currentTab] || "0spof";
  if (tabEyebrow) tabEyebrow.innerText = tabEyebrowText(currentTab);
}

function switchTab(target) {
  // "queue-modal" é o antigo nome da aba de nova submissão: agora o formulário
  // vive na fila de envio (painel lateral) e é aberto por "Nova entrega".
  const focusForm = target === "queue-modal";
  if (focusForm) target = "outbox";

  currentTab = target;
  navItems.forEach((b) => b.classList.toggle("active", b.getAttribute("data-tab") === target));
  tabPanes.forEach((p) => p.classList.toggle("active", p.id === `tab-${target}`));
  updateHeading();

  if (focusForm) {
    const card = document.getElementById("new-delivery-card");
    if (card) card.scrollIntoView({ behavior: "smooth", block: "start" });
    const first = document.getElementById("assign-course");
    if (first && !first.disabled) first.focus({ preventScroll: true });
    else document.getElementById("assign-id")?.focus({ preventScroll: true });
  }
}

navItems.forEach((btn) => {
  btn.addEventListener("click", () => switchTab(btn.getAttribute("data-tab")));
});

document.getElementById("btn-new-delivery")?.addEventListener("click", () => switchTab("queue-modal"));
document.getElementById("link-outbox-all")?.addEventListener("click", () => switchTab("outbox"));

// ---------------------------------------------------------------------------
// Carga de dados
// ---------------------------------------------------------------------------
async function refreshData() {
  try {
    const status = await invoke("get_system_status");
    document.getElementById("metric-courses").innerText = status.active_courses_count;
    const mutedPill = document.getElementById("metric-muted-courses");
    if (mutedPill) {
      if (status.muted_courses_count > 0) {
        mutedPill.innerText = `${status.muted_courses_count} silenciado${status.muted_courses_count > 1 ? "s" : ""}`;
        mutedPill.style.display = "inline-block";
      } else {
        mutedPill.style.display = "none";
      }
    }
    document.getElementById("metric-deadlines").innerText = status.deadlines_count;
    document.getElementById("metric-outbox").innerText = status.outbox_count;
    document.getElementById("outbox-count").innerText = status.outbox_count;
    const navCourses = document.getElementById("nav-courses-count");
    if (navCourses) navCourses.innerText = status.courses_count > 0 ? status.courses_count : "";

    const sessionInfo = await invoke("get_session_info");
    state.session = sessionInfo;
    const dot = document.getElementById("session-dot");
    const txt = document.getElementById("session-text");
    const activeCard = document.getElementById("session-active-card");
    const activeUrl = document.getElementById("session-connected-url");

    if (sessionInfo.is_authenticated) {
      dot.className = "dot online";
      txt.innerText = "Sessão ativa";
      const urlInput = document.getElementById("moodle-url");
      if (urlInput && !urlInput.value && sessionInfo.base_url) {
        urlInput.value = sessionInfo.base_url;
      }
      if (activeCard) activeCard.style.display = "flex";
      if (activeUrl) activeUrl.innerText = `${hostOf(sessionInfo.base_url)} · sessão válida`;
    } else {
      dot.className = "dot offline";
      txt.innerText = "Sessão desconectada";
      if (activeCard) activeCard.style.display = "none";
    }

    await loadCourses();
    await loadActivityIndex();
    await loadDeadlines();
    await loadOutbox();
    await loadSyncAndStorage();
    renderCourses();
    renderDashboard();
    renderOutbox();
    renderConnectionBanner();
    renderSessionDetail();
    populateCourseSelect();
    updateHeading();
  } catch (err) {
    console.error("Erro ao carregar status:", err);
  }
}

// Faixa de aviso: só relata o que o app sabe (sessão, falhas registradas na fila, falha de sync).
// O backend não reenvia sozinho; as tentativas aparecem como "tentativas" por item.
function renderConnectionBanner() {
  const banner = document.getElementById("connection-banner");
  if (!banner) return;
  const open = state.outbox.filter((i) => i.status !== "delivered");
  const failed = open.filter((i) => i.status === "failed");
  let title = "";
  let text = "";
  let action = null;

  if (state.syncError) {
    title = "Falha na última sincronização";
    text = String(state.syncError);
    action = { label: "Conexão e dados", tab: "settings" };
  } else if (failed.length > 0) {
    const latest = failed
      .filter((i) => i.last_attempt_at)
      .sort((a, b) => new Date(b.last_attempt_at) - new Date(a.last_attempt_at))[0];
    title = `${failed.length} ${failed.length === 1 ? "envio falhou" : "envios falharam"}`;
    text = latest
      ? `Última tentativa ${relativeTime(latest.last_attempt_at)}${latest.last_error ? `: ${latest.last_error}` : ""}.`
      : "Revise a fila de envio.";
    action = { label: "Ver fila", tab: "outbox" };
  } else if (!state.session.is_authenticated && open.length > 0) {
    title = "Sem sessão do Moodle";
    text = `${open.length} ${open.length === 1 ? "entrega aguarda" : "entregas aguardam"} na fila. Conecte-se para continuar.`;
    action = { label: "Conectar", tab: "settings" };
  }

  if (!title) {
    banner.style.display = "none";
    return;
  }
  document.getElementById("connection-banner-title").innerText = title;
  document.getElementById("connection-banner-text").innerText = text;
  const btn = document.getElementById("btn-connection-action");
  btn.innerText = action.label;
  btn.onclick = () => switchTab(action.tab);
  banner.style.display = "flex";
}

function renderSessionDetail() {
  const el = document.getElementById("session-detail");
  if (!el) return;
  const host = state.session.is_authenticated ? hostOf(state.session.base_url) : "";
  const last = lastSyncIso();
  const parts = [];
  if (host) parts.push(escapeHtml(host));
  parts.push(last ? `Sincronizado ${escapeHtml(relativeTime(last))}` : "Ainda não sincronizado");
  el.innerHTML = parts.join("<br>");
}

// Índice id da atividade -> atividade com o contexto da disciplina.
async function loadActivityIndex() {
  const idx = new Map();
  await Promise.all(
    cachedCourses.map(async (c) => {
      try {
        const sections = await invoke("get_course_details", { courseId: c.id });
        for (const sec of sections || []) {
          for (const act of sec.activities || []) {
            idx.set(act.id, {
              ...act,
              section_name: sec.name,
              course_id: c.id,
              course_short: c.shortname,
              course_name: c.fullname,
              course_muted: c.is_muted,
            });
          }
        }
      } catch (_) {}
    })
  );
  state.activities = idx;
}

async function loadSyncAndStorage() {
  try {
    state.syncReport = await invoke("get_last_sync_report");
  } catch (_) {
    state.syncReport = null;
  }
  try {
    state.storage = await invoke("get_storage_info");
  } catch (_) {
    state.storage = null;
  }
}

async function loadDeadlines() {
  try {
    state.deadlines = (await invoke("get_upcoming_deadlines")) || [];
    state.deadlinesError = null;
  } catch (err) {
    state.deadlines = [];
    state.deadlinesError = err;
  }
}

// Item mais relevante da fila para uma tarefa (prefere o que ainda não foi entregue).
function outboxItemForAssign(assignId) {
  const items = state.outbox.filter((i) => i.assign_id === assignId);
  if (items.length === 0) return null;
  return items.find((i) => i.status !== "delivered") || items[items.length - 1];
}

// ---------------------------------------------------------------------------
// Visão geral
// ---------------------------------------------------------------------------
function deadlineRowFrom(d) {
  const ctx = state.activities.get(d.id);
  return {
    id: d.id,
    title: d.title,
    type: d.activity_type,
    due_date: d.due_date,
    url: d.url,
    is_submitted: !!d.is_submitted,
    short: ctx ? ctx.course_short : "",
    course: ctx ? ctx.course_name : "",
  };
}

function sentRows() {
  return [...state.activities.values()]
    .filter((a) => a.activity_type === "assign" && a.is_submitted && !a.course_muted)
    .map((a) => ({
      id: a.id,
      title: a.title,
      type: a.activity_type,
      due_date: a.due_date,
      url: a.url,
      is_submitted: true,
      short: a.course_short,
      course: a.course_name,
    }))
    .sort((a, b) => (new Date(b.due_date || 0) - new Date(a.due_date || 0)));
}

const DEADLINE_GROUPS = [
  { key: "overdue", label: "Atrasados" },
  { key: "today", label: "Hoje" },
  { key: "week", label: "Esta semana" },
  { key: "later", label: "Próximas semanas" },
  { key: "nodate", label: "Sem prazo cadastrado" },
  { key: "sent", label: "Enviados" },
];

function groupKeyFor(row) {
  if (row.is_submitted) return "sent";
  if (!row.due_date) return "nodate";
  if (new Date(row.due_date).getTime() < Date.now()) return "overdue";
  const days = daysFromToday(row.due_date);
  if (days <= 0) return "today";
  if (days <= 7) return "week";
  return "later";
}

function rowChip(row) {
  const item = outboxItemForAssign(row.id);
  if (item) {
    if (item.status === "delivered") return `<span class="chip ok">Enviado</span>`;
    if (item.status === "failed") return `<span class="chip risk">Falha no envio</span>`;
    if (item.status === "in_flight") return `<span class="chip warn">Enviando…</span>`;
    return `<span class="chip info">Na fila</span>`;
  }
  if (row.is_submitted) return `<span class="chip ok">Entregue</span>`;
  if (row.type !== "assign") return `<span class="chip mut" title="Atividade respondida apenas no Moodle">Online</span>`;
  const g = groupKeyFor(row);
  if (g === "overdue") return `<span class="chip risk">Atrasada</span>`;
  if (g === "today" || g === "week") return `<span class="chip warn">Sem arquivo</span>`;
  return `<span class="chip mut">Sem arquivo</span>`;
}

function renderDeadlineRow(row) {
  const d = row.due_date ? new Date(row.due_date) : null;
  const tile = d
    ? `<small>${escapeHtml(weekdayShort(d).toUpperCase())}</small><b>${pad2(d.getDate())}</b>`
    : `<small>SEM</small><b>—</b>`;
  const sub = [row.short, row.course].filter(Boolean).join(" · ") || `Atividade #${row.id}`;
  const queued = !!outboxItemForAssign(row.id);
  const canQueue = row.type === "assign" && !row.is_submitted && !queued;
  const actions = [
    canQueue
      ? `<button class="btn btn-primary btn-sm btn-queue-assign" data-assign-id="${row.id}" title="Preparar entrega da tarefa #${row.id}">Enfileirar</button>`
      : "",
    row.url
      ? `<button class="btn-discreet btn-open-url" data-target="${escapeHtml(row.url)}" title="Abrir no Moodle institucional" aria-label="Abrir no Moodle">${ICON_EXTERNAL}</button>`
      : "",
  ].join("");
  return `
    <div class="deadline-row">
      <div class="date-tile">${tile}</div>
      <div class="row-main">
        <div class="row-title">${escapeHtml(row.title)}</div>
        <div class="row-sub">${escapeHtml(sub)}</div>
      </div>
      ${rowChip(row)}
      <div class="row-actions">${actions}</div>
    </div>`;
}

function renderDeadlines() {
  const container = document.getElementById("dashboard-deadlines-list");
  if (!container) return;
  if (state.deadlinesError) {
    container.innerHTML = `<p class="error">Erro ao carregar prazos: ${escapeHtml(state.deadlinesError)}</p>`;
    return;
  }
  const pending = state.deadlines.map(deadlineRowFrom);
  const sent = sentRows();
  let rows;
  if (state.deadlineView === "sent") rows = sent;
  else if (state.deadlineView === "all") rows = pending.concat(sent);
  else rows = pending;

  if (rows.length === 0) {
    container.innerHTML = `<p class="muted-line">${
      state.deadlineView === "sent"
        ? "Nenhuma entrega enviada ainda."
        : "Nenhum prazo pendente no momento. Você está em dia!"
    }</p>`;
    return;
  }

  container.innerHTML = DEADLINE_GROUPS.map((g) => {
    const inGroup = rows.filter((r) => groupKeyFor(r) === g.key);
    if (inGroup.length === 0) return "";
    return `<div class="deadline-group"><p class="grp">${g.label}</p>${inGroup.map(renderDeadlineRow).join("")}</div>`;
  }).join("");

  container.querySelectorAll(".btn-queue-assign").forEach((btn) => {
    btn.addEventListener("click", () => {
      setAssignTarget(btn.getAttribute("data-assign-id"));
      switchTab("queue-modal");
    });
  });

  container.querySelectorAll(".btn-open-url").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const target = btn.getAttribute("data-target");
      try {
        await invoke("open_target", { target });
      } catch (err) {
        alert("Erro ao abrir: " + err);
      }
    });
  });
}

document.querySelectorAll("[data-deadline-view]").forEach((btn) => {
  btn.addEventListener("click", () => {
    state.deadlineView = btn.getAttribute("data-deadline-view");
    document
      .querySelectorAll("[data-deadline-view]")
      .forEach((b) => b.classList.toggle("active", b === btn));
    renderDeadlines();
  });
});

function pickNextDelivery() {
  const now = Date.now();
  const future = state.deadlines.filter((d) => d.due_date && new Date(d.due_date).getTime() >= now);
  return future.find((d) => d.activity_type === "assign") || future[0] || null;
}

function renderNextDelivery() {
  const card = document.getElementById("next-delivery-card");
  if (!card) return;
  const next = pickNextDelivery();
  state.nextDue = next ? next.due_date : null;

  if (!next) {
    card.className = "next-card is-empty";
    card.innerHTML = `
      <div class="next-main">
        <p class="eyebrow">Próxima entrega</p>
        <h2>Nenhuma entrega à vista</h2>
        <p class="next-due">Quando houver prazos nas disciplinas sincronizadas, a mais próxima aparece aqui com contagem regressiva.</p>
      </div>`;
    return;
  }

  const ctx = state.activities.get(next.id);
  const item = outboxItemForAssign(next.id);
  const chips = [];
  if (item) {
    chips.push(`${escapeHtml(item.filename)} anexado`);
    if (item.has_rfc3161_token) chips.push("Selo RFC 3161 pronto");
  } else if (next.activity_type === "assign") {
    chips.push("Nenhum arquivo preparado");
  } else {
    chips.push("Respondida apenas no Moodle");
  }
  const dueLine = `Vence ${escapeHtml(formatDueLong(next.due_date))}${ctx ? ` · ${escapeHtml(ctx.course_name)}` : ""}`;
  let actions = "";
  if (item) {
    actions += `<button class="btn btn-on-dark" type="button" id="btn-next-queue">Ver na fila</button>`;
  } else if (next.activity_type === "assign") {
    actions += `<button class="btn btn-on-dark btn-queue-assign" type="button" data-assign-id="${next.id}">Preparar entrega</button>`;
  }
  if (next.url) {
    actions += `<button class="btn btn-ghost-dark btn-open-url" type="button" data-target="${escapeHtml(next.url)}">Abrir no Moodle</button>`;
  }
  const cd = formatCountdown(next.due_date);

  card.className = "next-card";
  card.innerHTML = `
    <div class="next-main">
      <p class="eyebrow">Próxima entrega${ctx ? ` · ${escapeHtml(ctx.course_short)}` : ""}</p>
      <h2>${escapeHtml(next.title)}</h2>
      <p class="next-due">${dueLine}</p>
      <div class="next-chips">${chips.map((c) => `<span class="chip on-dark">${c}</span>`).join("")}</div>
      <div class="next-actions">${actions}</div>
    </div>
    <div class="next-count">
      <div class="count-big" id="next-countdown">${cd.big}</div>
      <div class="count-small" id="next-countdown-label">${cd.small}</div>
    </div>`;

  card.querySelector("#btn-next-queue")?.addEventListener("click", () => switchTab("outbox"));
  card.querySelector(".btn-queue-assign")?.addEventListener("click", (e) => {
    setAssignTarget(e.currentTarget.getAttribute("data-assign-id"));
    switchTab("queue-modal");
  });
  card.querySelector(".btn-open-url")?.addEventListener("click", async (e) => {
    try {
      await invoke("open_target", { target: e.currentTarget.getAttribute("data-target") });
    } catch (err) {
      alert("Erro ao abrir: " + err);
    }
  });
}

function tickCountdown() {
  if (!state.nextDue) return;
  const big = document.getElementById("next-countdown");
  const small = document.getElementById("next-countdown-label");
  if (!big) return;
  const cd = formatCountdown(state.nextDue);
  big.innerText = cd.big;
  if (small) small.innerText = cd.small;
}
setInterval(tickCountdown, 30000);

function renderProtect() {
  const card = document.getElementById("protect-card");
  const title = document.getElementById("protect-title");
  const sub = document.getElementById("protect-sub");
  const failed = state.outbox.filter((i) => i.status === "failed").length;
  let tone = "ok";
  if (cachedCourses.length === 0) {
    tone = "empty";
    title.innerText = "Nada salvo ainda";
    sub.innerText = "Sincronize o Moodle para ter acesso offline.";
  } else if (failed > 0) {
    tone = "warn";
    title.innerText = "Atenção na fila de envio";
    sub.innerText = `${failed} ${failed === 1 ? "envio falhou" : "envios falharam"}. Revise na fila.`;
  } else {
    title.innerText = "Tudo protegido";
    sub.innerText = "Você pode ficar offline agora.";
  }
  card.setAttribute("data-tone", tone);

  const sess = document.getElementById("protect-session");
  sess.innerText = state.session.is_authenticated ? "Ativa" : "Desconectada";
  sess.className = state.session.is_authenticated ? "tone-ok" : "tone-warn";
  const last = lastSyncIso();
  document.getElementById("protect-lastsync").innerText = last ? relativeTime(last) : "Nunca";
  const storageEl = document.getElementById("protect-storage");
  if (storageEl) storageEl.innerText = state.storage ? formatBytes(state.storage.total_bytes) : "—";
  renderSyncReport(last);
}

const SYNC_CHANGE_LABELS = {
  new_resource: "Novo material",
  new_activity: "Nova atividade",
  deadline_changed: "Prazo alterado",
  file_cached: "Arquivo baixado",
};

function renderSyncReport(lastIso) {
  const line = document.getElementById("last-sync-line");
  const list = document.getElementById("last-sync-changes");
  const report = state.syncReport;
  const when = report ? report.synced_at : lastIso;
  if (line) {
    if (!when) line.innerText = "Ainda não sincronizado.";
    else if (!report) line.innerText = `${formatDate(when)} · ${relativeTime(when)}`;
    else if (report.changes_total === 0) line.innerText = `${formatDate(when)} · nenhuma novidade`;
    else line.innerText = `${formatDate(when)} · ${report.changes_total} ${report.changes_total === 1 ? "novidade" : "novidades"}`;
  }
  if (!list) return;
  const items = report ? report.changes : [];
  const extra = report ? report.changes_total - items.length : 0;
  list.innerHTML =
    items
      .map(
        (c) =>
          `<li><b>${escapeHtml(SYNC_CHANGE_LABELS[c.kind] || c.kind)}</b> em <b>${escapeHtml(c.course_short)}</b> — ${escapeHtml(c.title)}${c.detail ? ` <span class="sync-detail">(${escapeHtml(c.detail)})</span>` : ""}</li>`
      )
      .join("") + (extra > 0 ? `<li class="sync-more">+ ${extra} ${extra === 1 ? "outra novidade" : "outras novidades"}</li>` : "");
}

// "2 dias e 13 h", "5 h 10 min" ou "42 min" a partir de segundos.
function formatMargin(totalSecs) {
  const secs = Math.abs(totalSecs);
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (days > 0) return `${days} ${days === 1 ? "dia" : "dias"} e ${hours} h`;
  if (hours > 0) return `${hours} h ${mins} min`;
  return `${mins} min`;
}

// Etapas da linha do tempo, com os horários reais expostos pelo backend.
function outboxSteps(item) {
  const delivered = item.status === "delivered";
  const failed = item.status === "failed";
  const inFlight = item.status === "in_flight";
  const tries = item.retry_count || 0;
  const when = (iso) => (iso ? formatDate(iso) : "");
  const join = (...parts) => parts.filter(Boolean).join(" · ");
  return [
    { state: "done", title: "Arquivo registrado", sub: join(when(item.created_at), "hash SHA-256 guardado") },
    item.has_rfc3161_token
      ? { state: "done", title: "Selo RFC 3161 emitido", sub: join(when(item.timestamped_at || item.created_at), "TSA") }
      : { state: "todo", title: "Sem selo de tempo", sub: "Não solicitado ou autoridade indisponível" },
    delivered
      ? { state: "done", title: "Enviado ao Moodle", sub: when(item.last_attempt_at) }
      : failed
        ? {
            state: "fail",
            title: "Falha no envio",
            sub: join(`${tries} ${tries === 1 ? "tentativa" : "tentativas"}`, when(item.last_attempt_at), item.last_error),
          }
        : inFlight
          ? { state: "active", title: "Enviando ao Moodle", sub: "Transferência em andamento" }
          : { state: "active", title: "Enviando ao Moodle", sub: "Aguardando conexão" },
    delivered
      ? { state: "done", title: "Confirmado", sub: when(item.updated_at) }
      : { state: "todo", title: "Confirmado", sub: "" },
  ];
}

function marginText(item) {
  if (item.margin_seconds === null || item.margin_seconds === undefined) return "";
  return item.margin_seconds >= 0
    ? `Selado antes do prazo · ${formatMargin(item.margin_seconds)} de folga`
    : `Selado ${formatMargin(item.margin_seconds)} depois do prazo`;
}

function outboxStatusText(item) {
  if (item.status === "delivered") return "Confirmado pelo Moodle";
  if (item.status === "failed") {
    const n = item.retry_count || 0;
    return `Falha no envio · ${n} ${n === 1 ? "tentativa" : "tentativas"}`;
  }
  if (item.status === "in_flight") return "Enviando ao Moodle…";
  return "Aguardando conexão com o Moodle";
}

function outboxChip(item) {
  if (item.status === "delivered") return `<span class="chip ok">Confirmado pelo Moodle</span>`;
  if (item.status === "failed") return `<span class="chip risk">Falhou</span>`;
  if (item.status === "in_flight") return `<span class="chip warn">Enviando…</span>`;
  return `<span class="chip info">${item.has_rfc3161_token ? "Selo pronto · aguardando rede" : "Aguardando rede"}</span>`;
}

function renderMiniOutbox() {
  const box = document.getElementById("dashboard-outbox-list");
  if (!box) return;
  const open = state.outbox.filter((i) => i.status !== "delivered");
  if (open.length === 0) {
    box.innerHTML = `<p class="muted-line">Nenhum envio na fila.</p>`;
    return;
  }
  box.innerHTML = open
    .slice(0, 3)
    .map((item) => {
      const bars = outboxSteps(item)
        .map((s) => `<span class="bar-seg ${s.state}"></span>`)
        .join("");
      return `
        <div class="mini-item">
          <div class="mini-name">${escapeHtml(item.filename)}</div>
          <div class="mini-sub">${escapeHtml(outboxStatusText(item))}</div>
          <div class="mini-bars">${bars}</div>
        </div>`;
    })
    .join("");
}

function renderDashboard() {
  renderNextDelivery();
  renderProtect();
  renderDeadlines();
  renderMiniOutbox();
}

let cachedCourses = [];

// Filtros e ordenação da lista de disciplinas (lembrados neste computador).
const COURSE_PREFS_KEY = "0spof.courses.prefs";
const courseView = { query: "", filter: "all", sort: "name" };

try {
  Object.assign(courseView, JSON.parse(localStorage.getItem(COURSE_PREFS_KEY) || "{}"), { query: "" });
} catch (_) {}

function saveCourseView() {
  try {
    localStorage.setItem(COURSE_PREFS_KEY, JSON.stringify({ filter: courseView.filter, sort: courseView.sort }));
  } catch (_) {}
}

const normalizeText = (s) => (s || "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
const timeOrNull = (iso) => (iso ? new Date(iso).getTime() : null);
const byName = (a, b) => a.fullname.localeCompare(b.fullname, "pt-BR", { sensitivity: "base", numeric: true });

// Ordena valores ausentes por último, desempatando pelo nome.
function compareNullable(a, b, va, vb, dir) {
  if (va === vb) return byName(a, b);
  if (va === null) return 1;
  if (vb === null) return -1;
  return (va - vb) * dir;
}

const COURSE_SORTS = {
  name: byName,
  deadline: (a, b) => compareNullable(a, b, timeOrNull(a.next_deadline), timeOrNull(b.next_deadline), 1),
  pending: (a, b) => (b.pending_deadlines || 0) - (a.pending_deadlines || 0) || byName(a, b),
  activities: (a, b) => (b.activities_count || 0) - (a.activities_count || 0) || byName(a, b),
  synced: (a, b) => compareNullable(a, b, timeOrNull(a.last_synced_at), timeOrNull(b.last_synced_at), -1),
};

const COURSE_FILTERS = {
  all: () => true,
  active: (c) => !c.is_muted,
  muted: (c) => c.is_muted,
  pending: (c) => (c.pending_deadlines || 0) > 0,
};

function courseDeadlineChip(c) {
  if (c.is_muted) return `<span class="chip mut" title="Prazos desta disciplina estão ocultos na visão geral">Silenciada</span>`;
  if (!c.next_deadline) return `<span class="chip mut">Sem prazos</span>`;
  const days = daysFromToday(c.next_deadline);
  if (days <= 0) return `<span class="chip warn">Vence hoje</span>`;
  if (days === 1) return `<span class="chip warn">Amanhã</span>`;
  if (days <= 3) return `<span class="chip warn">Vence em ${days} dias</span>`;
  return `<span class="chip info">Em ${days} dias</span>`;
}

// Progresso de tarefas entregues, a partir do índice de atividades.
function courseTaskProgress(courseId) {
  let total = 0;
  let done = 0;
  for (const a of state.activities.values()) {
    if (a.course_id === courseId && a.activity_type === "assign") {
      total++;
      if (a.is_submitted) done++;
    }
  }
  return { total, done };
}

function syncCourseToolbar() {
  document.querySelectorAll(".filter-chip").forEach((chip) => {
    const on = chip.getAttribute("data-filter") === courseView.filter;
    chip.classList.toggle("active", on);
    chip.setAttribute("aria-checked", on ? "true" : "false");
  });
  const sortSel = document.getElementById("courses-sort");
  if (sortSel) sortSel.value = courseView.sort;
}

function renderCourses() {
  const container = document.getElementById("courses-list");
  const countEl = document.getElementById("courses-count");
  if (!cachedCourses || cachedCourses.length === 0) {
    container.innerHTML = "<p class='empty-state'>Nenhum curso cacheado localmente ainda. Clique em 'Sincronizar Moodle'.</p>";
    if (countEl) countEl.innerText = "";
    return;
  }
  const q = normalizeText(courseView.query.trim());
  const filterFn = COURSE_FILTERS[courseView.filter] || COURSE_FILTERS.all;
  const visible = cachedCourses
    .filter(filterFn)
    .filter((c) => !q || normalizeText(`${c.fullname} ${c.shortname}`).includes(q))
    .sort(COURSE_SORTS[courseView.sort] || COURSE_SORTS.name);
  if (countEl) {
    countEl.innerText = visible.length === cachedCourses.length
      ? ""
      : `${visible.length} de ${cachedCourses.length} disciplinas`;
  }
  if (visible.length === 0) {
    container.innerHTML = "<p class='empty-state'>Nenhuma disciplina corresponde aos filtros.</p>";
    return;
  }
  container.innerHTML = visible
    .map((c) => {
      const muteBtn = c.is_muted
        ? `<button class="btn btn-secondary btn-toggle-mute" data-course-id="${c.id}" data-muted="false" title="Reativar acompanhamento de prazos">
             <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>
             Reativar prazos
           </button>`
        : `<button class="btn btn-ghost btn-toggle-mute" data-course-id="${c.id}" data-muted="true" title="Silenciar prazos deste curso na visão geral">
             <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M13.73 21a2 2 0 0 1-3.46 0"/><path d="M18.63 13A17.89 17.89 0 0 1 18 8"/><path d="M6.26 6.26A5.86 5.86 0 0 0 6 8c0 7-3 9-3 9h14"/><path d="M18 8a6 6 0 0 0-9.33-5"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
             Silenciar
           </button>`;
      const prog = courseTaskProgress(c.id);
      const pct = prog.total > 0 ? Math.round((prog.done / prog.total) * 100) : 0;
      const progressHtml =
        prog.total > 0
          ? `<div class="progress"><div class="bar"><i style="width:${pct}%"></i></div><p>${prog.done} de ${prog.total} ${prog.total === 1 ? "tarefa entregue" : "tarefas entregues"}</p></div>`
          : `<div class="progress"><p>Nenhuma tarefa com entrega nesta disciplina</p></div>`;
      const pending = c.pending_deadlines || 0;

      return `
        <article class="course-card course-row ${c.is_muted ? "course-muted-row" : ""}" data-course-id="${c.id}" tabindex="0">
          <div class="cc-head">
            <div class="cc-title">
              <p class="eyebrow">${escapeHtml(c.shortname)}</p>
              <h3>${escapeHtml(c.fullname)}</h3>
            </div>
            ${courseDeadlineChip(c)}
          </div>
          <div class="stat-row">
            <span><b>${c.sections_count || 0}</b>seções</span>
            <span><b>${c.activities_count || 0}</b>atividades</span>
            <span><b>${pending}</b>${pending === 1 ? "prazo" : "prazos"}</span>
          </div>
          ${progressHtml}
          <div class="item-actions cc-actions">
            ${muteBtn}
            <button class="btn btn-ghost btn-export-course" data-course-id="${c.id}" title="Selecionar materiais e exportar pacote (.zip)">
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg>
              Exportar
            </button>
            <button class="btn btn-secondary btn-open-course" data-course-id="${c.id}">
              Abrir
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2.2"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>
            </button>
          </div>
        </article>
      `;
    })
    .join("");

  container.querySelectorAll(".btn-toggle-mute").forEach((btn) => {
    btn.addEventListener("click", async (e) => {
      e.stopPropagation();
      const courseId = parseInt(btn.getAttribute("data-course-id"), 10);
      const isMuted = btn.getAttribute("data-muted") === "true";
      try {
        await invoke("set_course_muted", { courseId, isMuted });
        await refreshData();
      } catch (err) {
        alert("Erro ao alterar silenciamento do curso: " + err);
      }
    });
  });

  container.querySelectorAll(".btn-export-course").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const courseId = parseInt(btn.getAttribute("data-course-id"), 10);
      openCourseExportModal(courseId);
    });
  });

  container.querySelectorAll(".course-row").forEach((row) => {
    const open = () => {
      const courseId = parseInt(row.getAttribute("data-course-id"), 10);
      openCourseDetail(courseId);
    };
    row.addEventListener("click", open);
    row.addEventListener("keydown", (e) => {
      if (e.target === row && (e.key === "Enter" || e.key === " ")) {
        e.preventDefault();
        open();
      }
    });
  });
}

async function loadCourses() {
  const container = document.getElementById("courses-list");
  try {
    cachedCourses = await invoke("get_courses");
    syncCourseToolbar();
    renderCourses();
  } catch (err) {
    container.innerHTML = `<p class="error">Erro ao listar cursos: ${err}</p>`;
  }
}

(function setupCourseToolbar() {
  const search = document.getElementById("courses-search");
  if (!search) return;
  search.addEventListener("input", () => {
    courseView.query = search.value;
    renderCourses();
  });
  document.querySelectorAll(".filter-chip").forEach((chip) => {
    chip.addEventListener("click", () => {
      courseView.filter = chip.getAttribute("data-filter");
      saveCourseView();
      syncCourseToolbar();
      renderCourses();
    });
  });
  document.getElementById("courses-sort").addEventListener("change", (e) => {
    courseView.sort = e.target.value;
    saveCourseView();
    renderCourses();
  });
  syncCourseToolbar();
})();

function getActivityBadge(actType) {
  const icons = {
    assign: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg> <span>Tarefa</span>`,
    resource: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg> <span>Arquivo</span>`,
    folder: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg> <span>Pasta</span>`,
    url: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg> <span>Link Externo</span>`,
    forum: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg> <span>Fórum</span>`,
    quiz: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><polyline points="9 11 12 14 22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/></svg> <span>Questionário</span>`,
    page: `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg> <span>Página</span>`,
  };
  return icons[actType] || `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><circle cx="12" cy="12" r="10"/></svg> <span>${actType}</span>`;
}

// Ícone/rótulo do tipo de recurso na lista de atividades da disciplina.
function activityKind(act, hasLocalFile, isVideo) {
  const mime = act.content_mime || "";
  const path = (act.local_file_path || act.url || "").toLowerCase();
  if (mime === "application/pdf" || /\.pdf(\?|$)/.test(path)) return { label: "PDF", cls: "k-pdf" };
  if (isVideo) return { label: "VÍD", cls: "k-vid" };
  if (mime.startsWith("image/") || /\.(png|jpe?g|gif|svg|webp)(\?|$)/.test(path)) return { label: "IMG", cls: "k-img" };
  const byType = {
    assign: { label: "TAR", cls: "k-task" },
    page: { label: "HTML", cls: "k-html" },
    resource: { label: "ARQ", cls: "k-other" },
    folder: { label: "PST", cls: "k-other" },
    url: { label: "LNK", cls: "k-html" },
    forum: { label: "FÓR", cls: "k-other" },
    quiz: { label: "QST", cls: "k-task" },
  };
  return byType[act.activity_type] || { label: "•", cls: "k-other" };
}

async function openCourseDetail(courseId) {
  const course = cachedCourses.find((c) => c.id === courseId);
  document.getElementById("courses-master-view").style.display = "none";
  document.getElementById("course-detail-view").style.display = "block";
  document.getElementById("course-detail-short").innerText = course ? course.shortname : `#${courseId}`;
  document.getElementById("course-detail-title").innerText = course ? course.fullname : "Disciplina";

  const btnDetailExport = document.getElementById("btn-detail-export-course");
  if (btnDetailExport) {
    btnDetailExport.onclick = () => openCourseExportModal(courseId);
  }

  const btnDetailMute = document.getElementById("btn-detail-toggle-mute");
  if (btnDetailMute && course) {
    btnDetailMute.className = "btn btn-secondary";
    if (course.is_muted) {
      btnDetailMute.innerHTML = `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg> <span>Reativar prazos</span>`;
      btnDetailMute.title = "Reativar prazos desta disciplina na visão geral";
    } else {
      btnDetailMute.innerHTML = `<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M13.73 21a2 2 0 0 1-3.46 0"/><path d="M18.63 13A17.89 17.89 0 0 1 18 8"/><path d="M6.26 6.26A5.86 5.86 0 0 0 6 8c0 7-3 9-3 9h14"/><path d="M18 8a6 6 0 0 0-9.33-5"/><line x1="1" y1="1" x2="23" y2="23"/></svg> <span>Silenciar</span>`;
      btnDetailMute.title = "Silenciar prazos desta disciplina na visão geral";
    }
    btnDetailMute.onclick = async () => {
      try {
        await invoke("set_course_muted", { courseId: course.id, isMuted: !course.is_muted });
        course.is_muted = !course.is_muted;
        await refreshData();
        await openCourseDetail(courseId);
      } catch (err) {
        alert("Erro ao alterar silenciamento: " + err);
      }
    };
  }

  const tocNav = document.getElementById("course-toc-nav");
  const tocFoot = document.getElementById("course-toc-foot");
  const sectionsContainer = document.getElementById("course-sections-list");

  tocNav.innerHTML = "<p class='muted-line'>Carregando sumário…</p>";
  if (tocFoot) tocFoot.innerText = "";
  sectionsContainer.innerHTML = "<p class='muted-line'>Carregando seções…</p>";

  try {
    const sections = await invoke("get_course_details", { courseId });
    if (!sections || sections.length === 0) {
      tocNav.innerHTML = "<p class='muted-line'>Nenhuma seção.</p>";
      sectionsContainer.innerHTML = "<p class='muted-line'>Nenhuma seção cadastrada nesta disciplina.</p>";
      return;
    }

    const totalActivities = sections.reduce((n, s) => n + (s.activities ? s.activities.length : 0), 0);
    if (tocFoot) tocFoot.innerText = `${sections.length} seções · ${totalActivities} atividades`;

    tocNav.innerHTML = sections
      .map(
        (sec, idx) => `
        <a class="toc-link" data-target-sec="sec-${sec.id}" title="${escapeHtml(sec.name)}">
          <span class="toc-num">${pad2(idx)}</span><span class="toc-name">${escapeHtml(sec.name)}</span>
        </a>
      `
      )
      .join("");

    tocNav.querySelectorAll(".toc-link").forEach((link) => {
      link.addEventListener("click", (e) => {
        e.preventDefault();
        const targetId = link.getAttribute("data-target-sec");
        const targetEl = document.getElementById(targetId);
        if (targetEl) {
          targetEl.scrollIntoView({ behavior: "smooth", block: "start" });
          tocNav.querySelectorAll(".toc-link").forEach((l) => l.classList.remove("active"));
          link.classList.add("active");
        }
      });
    });

    // Próxima tarefa pendente da disciplina (faixa de destaque no topo).
    const upcoming = sections
      .flatMap((s) => s.activities || [])
      .filter((a) => a.activity_type === "assign" && !a.is_submitted && a.due_date)
      .sort((a, b) => new Date(a.due_date) - new Date(b.due_date))[0];
    let bannerHtml = "";
    if (upcoming) {
      const qItem = outboxItemForAssign(upcoming.id);
      bannerHtml = `
        <section class="card task-banner">
          <span class="task-banner-icon">${ICON_CLOCK}</span>
          <div class="task-banner-main">
            <p class="eyebrow">Tarefa · vence ${escapeHtml(formatDueLong(upcoming.due_date))}</p>
            <h2>${escapeHtml(upcoming.title)}</h2>
          </div>
          ${qItem ? `<span class="chip info">Na fila</span>` : ""}
          <button class="btn btn-primary btn-queue-assign" data-assign-id="${upcoming.id}" type="button">${qItem ? "Nova versão" : "Preparar entrega"}</button>
        </section>`;
    }

    sectionsContainer.innerHTML =
      bannerHtml +
      sections
        .map((sec, secIdx) => {
          const summaryHtml = sec.summary ? `<div class="section-summary">${sec.summary}</div>` : "";
          const activitiesHtml =
            !sec.activities || sec.activities.length === 0
              ? `<p class="muted-line">Nenhuma atividade nesta seção.</p>`
              : sec.activities
                  .map((act) => {
                    const descHtml = act.description
                      ? `<div class="activity-description">${act.description}</div>`
                      : "";
                    const hasLocalFile = act.local_file_path && act.local_file_path.length > 0;
                    const isVideo =
                      (act.content_mime && act.content_mime.startsWith("video/")) ||
                      (act.url && /\.(mp4|webm|mkv)(\?|$)/i.test(act.url));

                    let actionsHtml = "";
                    if (act.activity_type === "assign") {
                      actionsHtml += `
                        <button class="btn btn-primary btn-queue-assign" data-assign-id="${act.id}">
                          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                          Enfileirar entrega (#${act.id})
                        </button>
                      `;
                    }

                    if (hasLocalFile) {
                      actionsHtml += `
                        <button class="btn btn-secondary btn-open-local" data-target="${escapeHtml(act.local_file_path)}">
                          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                          Abrir cópia offline
                        </button>
                      `;
                    } else if (isVideo) {
                      actionsHtml += `
                        <button class="btn btn-secondary btn-download-media" data-act-id="${act.id}" data-video="true">
                          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                          Baixar vídeo offline
                        </button>
                      `;
                    } else if (act.activity_type === "resource" || act.activity_type === "page") {
                      actionsHtml += `
                        <button class="btn btn-secondary btn-download-media" data-act-id="${act.id}" data-video="false">
                          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                          Baixar recurso
                        </button>
                      `;
                    }

                    if (act.url) {
                      if (act.activity_type === "url") {
                        actionsHtml += `
                          <button class="btn btn-primary btn-open-url" data-target="${escapeHtml(act.url)}">
                            <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                            Acessar link externo
                          </button>
                        `;
                      } else {
                        actionsHtml += `
                          <button class="btn-discreet btn-open-url" data-target="${escapeHtml(act.url)}" title="Abrir página desta atividade no Moodle institucional">
                            <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                            Abrir no Moodle
                          </button>
                        `;
                      }
                    }

                    let badgeHtml = "";
                    if (act.activity_type === "assign") {
                      const qItem = outboxItemForAssign(act.id);
                      if (act.is_submitted) {
                        badgeHtml = `<span class="chip ok">Entregue</span>`;
                      } else if (qItem) {
                        badgeHtml = `<span class="chip info">Na fila</span>`;
                      } else {
                        badgeHtml = `<span class="chip warn">Pendente</span>`;
                      }
                    } else if (hasLocalFile) {
                      badgeHtml = `<span class="chip ok">Offline</span>`;
                    } else {
                      badgeHtml = `<span class="chip mut">${getActivityBadge(act.activity_type)}</span>`;
                    }

                    const dueDateHtml =
                      act.activity_type === "assign" && act.due_date
                        ? `<span class="due-date-tag"><svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg> Prazo: ${formatDate(act.due_date)}</span>`
                        : "";

                    const kind = activityKind(act, hasLocalFile, isVideo);

                    return `
                      <div class="list-item activity-card">
                        <div class="file-ico ${kind.cls}">${kind.label}</div>
                        <div class="item-content">
                          <div class="activity-line">
                            <strong>${escapeHtml(act.title)}</strong>
                            <small>#${act.id}</small>
                            ${dueDateHtml}
                          </div>
                          ${descHtml}
                          ${actionsHtml ? `<div class="item-actions">${actionsHtml}</div>` : ""}
                        </div>
                        <div class="activity-state">${badgeHtml}</div>
                      </div>
                    `;
                  })
                  .join("");

          const n = sec.activities ? sec.activities.length : 0;
          return `
            <section id="sec-${sec.id}" class="card section-card">
              <div class="section-card-head">
                <div class="section-card-title"><span class="sec-num">${pad2(secIdx)}</span><h3>${escapeHtml(sec.name)}</h3></div>
                <span class="sec-count">${n} ${n === 1 ? "atividade" : "atividades"}</span>
              </div>
              ${summaryHtml}
              <div class="section-activities">${activitiesHtml}</div>
            </section>
          `;
        })
        .join("");

    sectionsContainer.querySelectorAll(".btn-queue-assign").forEach((btn) => {
      btn.addEventListener("click", () => {
        setAssignTarget(btn.getAttribute("data-assign-id"));
        switchTab("queue-modal");
      });
    });

    sectionsContainer.querySelectorAll(".btn-open-local, .btn-open-url").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const target = btn.getAttribute("data-target");
        try {
          await invoke("open_target", { target });
        } catch (err) {
          alert("Erro ao abrir: " + err);
        }
      });
    });

    sectionsContainer.querySelectorAll(".btn-download-media").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const activityId = parseInt(btn.getAttribute("data-act-id"), 10);
        const allowVideo = btn.getAttribute("data-video") === "true";
        const originalText = btn.innerHTML;
        btn.disabled = true;
        btn.innerText = "Baixando...";
        try {
          const savedPath = await invoke("download_activity_media", { activityId, allowVideo });
          if (savedPath) {
            await openCourseDetail(courseId);
          } else {
            alert("Este recurso é um vídeo. Use o botão 'Baixar vídeo offline' para confirmar.");
            await openCourseDetail(courseId);
          }
        } catch (err) {
          alert("Erro ao baixar recurso: " + err);
          btn.disabled = false;
          btn.innerHTML = originalText;
        }
      });
    });
  } catch (err) {
    sectionsContainer.innerHTML = `<p class="error">Erro ao carregar detalhes do curso: ${escapeHtml(err)}</p>`;
  }
}

document.getElementById("btn-back-courses").addEventListener("click", () => {
  document.getElementById("course-detail-view").style.display = "none";
  document.getElementById("courses-master-view").style.display = "block";
});

async function loadOutbox() {
  try {
    state.outbox = (await invoke("get_outbox_items")) || [];
    state.outboxError = null;
  } catch (err) {
    state.outbox = [];
    state.outboxError = err;
  }
}

function fileKind(filename) {
  const ext = (String(filename).split(".").pop() || "").toLowerCase();
  if (ext === "pdf") return { label: "PDF", cls: "k-pdf" };
  if (["zip", "rar", "7z", "tar", "gz"].includes(ext)) return { label: "ZIP", cls: "k-zip" };
  if (["doc", "docx", "odt"].includes(ext)) return { label: "DOC", cls: "k-doc" };
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext)) return { label: "IMG", cls: "k-img" };
  return { label: (ext || "ARQ").slice(0, 3).toUpperCase(), cls: "k-other" };
}

function renderTimeline(steps) {
  return steps
    .map((s, i) => {
      const last = i === steps.length - 1;
      const dotInner = s.state === "done" ? ICON_CHECK(12) : s.state === "fail" ? "!" : "";
      return `
        <div class="step step-${s.state}">
          <div class="step-rail">
            <span class="step-dot">${dotInner}</span>
            ${last ? "" : `<span class="step-line ${s.state === "done" ? "done" : ""}"></span>`}
          </div>
          <div class="step-body">
            <div class="step-title">${escapeHtml(s.title)}</div>
            ${s.sub ? `<div class="step-sub">${escapeHtml(s.sub)}</div>` : ""}
          </div>
        </div>`;
    })
    .join("");
}

function renderOutbox() {
  const container = document.getElementById("outbox-list");
  const detail = document.getElementById("outbox-detail");
  const chip = document.getElementById("outbox-session-chip");
  if (!container) return;

  if (chip) {
    if (state.session.is_authenticated) {
      chip.className = "chip ok";
      chip.innerText = "Sessão do Moodle ativa";
    } else {
      chip.className = "chip warn";
      chip.innerText = "Sessão desconectada · conecte-se para enviar";
    }
  }

  if (state.outboxError) {
    container.innerHTML = `<p class="error">Erro ao listar fila: ${escapeHtml(state.outboxError)}</p>`;
    if (detail) detail.style.display = "none";
    return;
  }
  // Itens a enviar primeiro; entregues por último (mais recentes antes).
  const items = [...state.outbox].sort((a, b) => {
    const da = a.status === "delivered" ? 1 : 0;
    const db = b.status === "delivered" ? 1 : 0;
    return da - db || b.id - a.id;
  });
  if (items.length === 0) {
    container.innerHTML = "<p class='empty-state'>Nenhuma tarefa na fila de envio.</p>";
    if (detail) detail.style.display = "none";
    return;
  }

  if (!items.some((i) => i.id === state.selectedOutboxId)) state.selectedOutboxId = items[0].id;

  container.innerHTML = items
    .map((item) => {
      const kind = fileKind(item.filename);
      const ctx = state.activities.get(item.assign_id);
      const context = ctx
        ? `${ctx.course_short} · ${ctx.title}${ctx.due_date ? ` · vence ${formatDueLong(ctx.due_date)}` : ""}`
        : `Tarefa #${item.assign_id}`;
      const icon =
        item.status === "delivered"
          ? `<div class="file-ico k-ok">${ICON_CHECK(18)}</div>`
          : `<div class="file-ico ${kind.cls}">${escapeHtml(kind.label)}</div>`;
      return `
        <button type="button" class="outbox-item list-item ${item.id === state.selectedOutboxId ? "selected" : ""}" data-id="${item.id}">
          ${icon}
          <div class="item-content">
            <div class="row-title">${escapeHtml(item.filename)}</div>
            <div class="row-sub">${escapeHtml(context)}${item.retry_count > 0 ? ` · ${item.retry_count} ${item.retry_count === 1 ? "tentativa" : "tentativas"}` : ""}</div>
          </div>
          ${outboxChip(item)}
        </button>`;
    })
    .join("");

  container.querySelectorAll(".outbox-item").forEach((el) => {
    el.addEventListener("click", () => {
      state.selectedOutboxId = parseInt(el.getAttribute("data-id"), 10);
      renderOutbox();
    });
  });

  const sel = items.find((i) => i.id === state.selectedOutboxId);
  if (detail && sel) {
    detail.style.display = "block";
    detail.innerHTML = `
      <div class="receipt-head">
        <div>
          <p class="eyebrow">Comprovante</p>
          <h2>${escapeHtml(sel.filename)}</h2>
        </div>
        <div class="receipt-actions">
          <button class="btn btn-secondary btn-proof" data-id="${sel.id}" type="button">Ver dossiê</button>
          <button class="btn btn-secondary btn-export-proof" data-id="${sel.id}" type="button">Exportar comprovante</button>
        </div>
      </div>
      <div class="receipt-body">
        <div class="timeline">${renderTimeline(outboxSteps(sel))}</div>
        <div class="receipt-facts">
          <div>
            <div class="fact-label">Hash SHA-256</div>
            <code class="hash-block">${escapeHtml(sel.file_hash)}</code>
          </div>
          <div>
            <div class="fact-label">Selo de tempo</div>
            <div class="fact-value">${sel.has_rfc3161_token ? `Carimbo RFC 3161${sel.timestamped_at ? ` emitido em ${escapeHtml(formatDate(sel.timestamped_at))}` : " anexado ao arquivo"}` : "Nenhum carimbo anexado"}</div>
          </div>
          ${sel.due_date ? `<div>
            <div class="fact-label">Prazo da tarefa</div>
            <div class="fact-value">${escapeHtml(formatDueLong(sel.due_date))}${marginText(sel) ? `<br>${escapeHtml(marginText(sel))}` : ""}</div>
          </div>` : ""}
          <div>
            <div class="fact-label">Tarefa no Moodle</div>
            <div class="fact-value">#${sel.assign_id}${state.activities.get(sel.assign_id) ? ` · ${escapeHtml(state.activities.get(sel.assign_id).title)}` : ""}</div>
          </div>
        </div>
      </div>`;
    detail.querySelectorAll(".btn-proof").forEach((btn) => {
      btn.addEventListener("click", () => openProof(btn.getAttribute("data-id")));
    });
    detail.querySelectorAll(".btn-export-proof").forEach((btn) => {
      btn.addEventListener("click", () => exportProof(parseInt(btn.getAttribute("data-id"), 10)));
    });
  }
}

// ---------------------------------------------------------------------------
// Formulário "Nova entrega": disciplina -> tarefa -> ID preenchido sozinho.
// O campo de ID continua disponível para tarefas ainda não sincronizadas.
// ---------------------------------------------------------------------------
const formEnqueue = document.getElementById("form-enqueue");
const resultBox = document.getElementById("enqueue-result");
const assignCourseSel = document.getElementById("assign-course");
const assignTaskSel = document.getElementById("assign-task");
const assignIdInput = document.getElementById("assign-id");
const assignIdHint = document.getElementById("assign-id-hint");
const ASSIGN_HINT_DEFAULT = "Preenchido automaticamente ao escolher a tarefa; ou digite o ID.";

function assignTasksOf(courseId) {
  return [...state.activities.values()]
    .filter((a) => a.course_id === courseId && a.activity_type === "assign")
    .sort((a, b) => {
      if (a.is_submitted !== b.is_submitted) return a.is_submitted ? 1 : -1;
      const ta = a.due_date ? new Date(a.due_date).getTime() : Infinity;
      const tb = b.due_date ? new Date(b.due_date).getTime() : Infinity;
      return ta - tb;
    });
}

function populateCourseSelect() {
  if (!assignCourseSel) return;
  const prev = assignCourseSel.value;
  const withTasks = cachedCourses
    .filter((c) => assignTasksOf(c.id).length > 0)
    .sort((a, b) => a.fullname.localeCompare(b.fullname, "pt-BR"));
  assignCourseSel.innerHTML =
    `<option value="">${withTasks.length ? "Selecione a disciplina…" : "Nenhuma tarefa sincronizada"}</option>` +
    withTasks.map((c) => `<option value="${c.id}">${escapeHtml(c.shortname)} · ${escapeHtml(c.fullname)}</option>`).join("");
  assignCourseSel.disabled = withTasks.length === 0;
  if (prev && withTasks.some((c) => String(c.id) === prev)) assignCourseSel.value = prev;
  populateTaskSelect();
}

function populateTaskSelect() {
  if (!assignTaskSel) return;
  const courseId = parseInt(assignCourseSel.value, 10);
  const prev = assignTaskSel.value;
  if (!courseId) {
    assignTaskSel.innerHTML = `<option value="">Escolha a disciplina primeiro</option>`;
    assignTaskSel.disabled = true;
    syncAssignIdField();
    return;
  }
  const tasks = assignTasksOf(courseId);
  assignTaskSel.innerHTML =
    `<option value="">Selecione a tarefa…</option>` +
    tasks
      .map((t) => {
        const tags = [t.is_submitted ? "entregue" : "", t.due_date ? `vence ${formatDueLong(t.due_date)}` : ""].filter(Boolean);
        return `<option value="${t.id}">${escapeHtml(t.title)}${tags.length ? ` (${escapeHtml(tags.join(" · "))})` : ""}</option>`;
      })
      .join("");
  assignTaskSel.disabled = false;
  if (prev && tasks.some((t) => String(t.id) === prev)) assignTaskSel.value = prev;
  syncAssignIdField();
}

// Quando há tarefa escolhida o ID vem dela (somente leitura); senão fica livre.
function syncAssignIdField() {
  if (!assignIdInput) return;
  const taskId = parseInt(assignTaskSel.value, 10);
  if (taskId) {
    assignIdInput.value = String(taskId);
    assignIdInput.readOnly = true;
    if (assignIdHint) assignIdHint.innerText = `ID no Moodle: ${taskId} · preenchido automaticamente`;
  } else {
    if (assignIdInput.readOnly) assignIdInput.value = "";
    assignIdInput.readOnly = false;
    if (assignIdHint) assignIdHint.innerText = ASSIGN_HINT_DEFAULT;
  }
}

function setAssignTarget(assignId) {
  const id = parseInt(assignId, 10);
  const act = state.activities.get(id);
  populateCourseSelect();
  if (act && act.activity_type === "assign") {
    assignCourseSel.value = String(act.course_id);
    populateTaskSelect();
    assignTaskSel.value = String(act.id);
    syncAssignIdField();
  } else {
    assignCourseSel.value = "";
    populateTaskSelect();
    assignIdInput.value = String(id);
  }
}

function resetEnqueueForm() {
  formEnqueue.reset();
  document.getElementById("check-tsa").checked = true;
  if (filePicked) filePicked.hidden = true;
  populateCourseSelect();
  assignIdInput.readOnly = false;
  if (assignIdHint) assignIdHint.innerText = ASSIGN_HINT_DEFAULT;
}

// Seleção do arquivo: diálogo nativo do Tauri, arrastar e soltar ou caminho digitado.
const filePathInput = document.getElementById("file-path");
const filePicked = document.getElementById("file-picked");
const fileDrop = document.getElementById("file-drop");

async function showPickedFile() {
  const path = filePathInput.value.trim();
  if (!path) {
    filePicked.hidden = true;
    return;
  }
  filePicked.hidden = false;
  try {
    const info = await invoke("file_info", { path });
    filePicked.className = "file-picked";
    filePicked.innerText = `${info.name} · ${formatBytes(info.size_bytes)}`;
  } catch (err) {
    filePicked.className = "file-picked bad";
    filePicked.innerText = String(err);
  }
}

function setPickedFile(path) {
  filePathInput.value = path;
  return showPickedFile();
}

async function pickFile() {
  const dialog = window.__TAURI__ && window.__TAURI__.dialog;
  if (!dialog || typeof dialog.open !== "function") {
    filePathInput.focus();
    return;
  }
  try {
    const selected = await dialog.open({ multiple: false, directory: false });
    if (selected) await setPickedFile(typeof selected === "string" ? selected : selected.path);
  } catch (err) {
    alert("Erro ao abrir o seletor de arquivos: " + err);
  }
}

document.getElementById("btn-pick-file")?.addEventListener("click", pickFile);
filePathInput?.addEventListener("change", showPickedFile);

if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.listen === "function") {
  const ev = window.__TAURI__.event;
  ev.listen("tauri://drag-enter", () => {
    switchTab("queue-modal");
    fileDrop?.classList.add("dragging");
  });
  ev.listen("tauri://drag-leave", () => fileDrop?.classList.remove("dragging"));
  ev.listen("tauri://drag-drop", (e) => {
    fileDrop?.classList.remove("dragging");
    const paths = e && e.payload && e.payload.paths;
    if (paths && paths.length > 0) {
      switchTab("queue-modal");
      setPickedFile(paths[0]);
    }
  });
}

assignCourseSel?.addEventListener("change", populateTaskSelect);
assignTaskSel?.addEventListener("change", syncAssignIdField);

formEnqueue.addEventListener("submit", async (e) => {
  e.preventDefault();
  resultBox.className = "result-message";
  resultBox.style.display = "none";

  const assignId = parseInt(document.getElementById("assign-id").value, 10);
  const filePath = document.getElementById("file-path").value;
  const checkTsa = document.getElementById("check-tsa").checked;

  try {
    const res = await invoke("enqueue_submission", { assignId, filePath, fetchTsa: checkTsa });
    resultBox.className = "result-message success";
    resultBox.innerText = `Tarefa enfileirada com sucesso! ID: ${res.outbox_id} | SHA-256: ${res.file_hash}`;
    resultBox.style.display = "block";
    resetEnqueueForm();
    state.selectedOutboxId = res.outbox_id;
    await refreshData();
  } catch (err) {
    resultBox.className = "result-message error";
    resultBox.innerText = `Erro ao enfileirar: ${err}`;
    resultBox.style.display = "block";
  }
});

const formSession = document.getElementById("form-session");
const sessionResult = document.getElementById("session-result");

formSession.addEventListener("submit", async (e) => {
  e.preventDefault();
  sessionResult.className = "result-message";
  sessionResult.style.display = "none";

  const baseUrl = document.getElementById("moodle-url").value;
  const sessionCookie = document.getElementById("moodle-cookie").value;

  try {
    const res = await invoke("save_session", { baseUrl, sessionCookie });
    sessionResult.className = "result-message success";
    sessionResult.innerText = `Sessão para ${res.base_url} salva com sucesso!`;
    sessionResult.style.display = "block";
    await refreshData();
  } catch (err) {
    sessionResult.className = "result-message error";
    sessionResult.innerText = `Erro ao salvar sessão: ${err}`;
    sessionResult.style.display = "block";
  }
});

const btnSync = document.getElementById("btn-sync");
btnSync.addEventListener("click", async () => {
  btnSync.disabled = true;
  const originalText = btnSync.innerHTML;
  btnSync.innerText = "Sincronizando...";

  try {
    const summary = await invoke("sync_moodle");
    state.syncError = null;
    if (summary.courses_synced === 0) {
      alert("Sincronização concluída, mas nenhum curso foi encontrado. Verifique seu login no Moodle.");
    } else {
      alert(
        `Sincronização concluída com sucesso!\nCursos atualizados: ${summary.courses_synced}\nSeções indexadas: ${summary.sections_synced}\nAtividades catalogadas: ${summary.activities_synced}\nArquivos/PDFs cacheados offline: ${summary.resources_cached || 0}`
      );
    }
    await refreshData();
  } catch (err) {
    state.syncError = err;
    renderConnectionBanner();
    alert("Erro na sincronização: " + err);
  } finally {
    btnSync.disabled = false;
    btnSync.innerHTML = originalText;
  }
});

const proofModal = document.getElementById("proof-modal");
const proofText = document.getElementById("proof-text");
const btnCloseModal = document.getElementById("btn-close-modal");
const btnCopyProof = document.getElementById("btn-copy-proof");

// Salva o comprovante como arquivo de texto (o mesmo relatório do dossiê).
function downloadTextFile(filename, text) {
  const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

async function exportProof(outboxId) {
  try {
    const proof = await invoke("get_submission_proof", { outboxId });
    const base = String(proof.filename).replace(/\.[^.]*$/, "").replace(/[^\w.-]+/g, "_");
    downloadTextFile(`comprovante-0spof-${base}-${proof.assign_id}.txt`, proof.formatted_report);
  } catch (err) {
    alert("Erro ao exportar comprovante: " + err);
  }
}

let currentProofId = null;

async function openProof(outboxId) {
  try {
    const proof = await invoke("get_submission_proof", { outboxId: parseInt(outboxId, 10) });
    proofText.innerText = proof.formatted_report;
    currentProofId = parseInt(outboxId, 10);
    proofModal.classList.remove("hidden");
  } catch (err) {
    alert("Erro ao obter comprovante: " + err);
  }
}

btnCloseModal.addEventListener("click", () => proofModal.classList.add("hidden"));
document.getElementById("btn-save-proof")?.addEventListener("click", () => {
  if (currentProofId !== null) exportProof(currentProofId);
});
btnCopyProof.addEventListener("click", () => {
  navigator.clipboard.writeText(proofText.innerText);
  alert("Comprovante copiado para a área de transferência!");
});

// Login Automático Nativo via Webview
const btnAutoLogin = document.getElementById("btn-auto-login");
const autoLoginSpinner = document.getElementById("auto-login-spinner");

if (btnAutoLogin) {
  btnAutoLogin.addEventListener("click", async () => {
    const urlInput = document.getElementById("moodle-url");
    const baseUrl = urlInput && urlInput.value.trim() ? urlInput.value.trim() : "https://moodle2.jcr.ifsp.edu.br";
    btnAutoLogin.disabled = true;
    if (autoLoginSpinner) autoLoginSpinner.style.display = "inline";

    try {
      await invoke("open_moodle_login_window", { baseUrl });
    } catch (err) {
      alert("Erro ao abrir janela de login institucional: " + err);
      btnAutoLogin.disabled = false;
      if (autoLoginSpinner) autoLoginSpinner.style.display = "none";
    }
  });
}

// Escuta evento de sessão conectada emitido pelo backend nativo
if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.listen === "function") {
  window.__TAURI__.event.listen("session_connected", async () => {
    if (btnAutoLogin) btnAutoLogin.disabled = false;
    if (autoLoginSpinner) autoLoginSpinner.style.display = "none";
    const sessionRes = document.getElementById("session-result");
    if (sessionRes) {
      sessionRes.className = "result-message success";
      sessionRes.innerText = "Conexão institucional estabelecida com sucesso via login oficial!";
      sessionRes.style.display = "block";
    }
    await refreshData();
  });
}

window.addEventListener("focus", async () => {
  if (btnAutoLogin) btnAutoLogin.disabled = false;
  if (autoLoginSpinner) autoLoginSpinner.style.display = "none";
  await refreshData();
});

// ---------------------------------------------------------------------------
// Busca global (Ctrl K): disciplinas, atividades e recursos, no banco local.
// ---------------------------------------------------------------------------
const searchInput = document.getElementById("global-search-input");
const searchResults = document.getElementById("global-search-results");
const SEARCH_KIND_LABELS = { course: "Disciplina", activity: "Atividade", resource: "Material" };
let searchHits = [];
let searchActive = -1;
let searchTimer = null;
let searchSeq = 0;

function closeSearch() {
  if (!searchResults) return;
  searchResults.hidden = true;
  searchInput.setAttribute("aria-expanded", "false");
  searchActive = -1;
}

function highlightSearchActive() {
  searchResults.querySelectorAll(".search-hit").forEach((el, i) => {
    el.classList.toggle("active", i === searchActive);
    if (i === searchActive) el.scrollIntoView({ block: "nearest" });
  });
}

function renderSearchResults(query) {
  if (searchHits.length === 0) {
    searchResults.innerHTML = `<p class="search-empty">Nada encontrado para "${escapeHtml(query)}".</p>`;
  } else {
    searchResults.innerHTML = searchHits
      .map(
        (h, i) => `
        <button type="button" class="search-hit" role="option" data-index="${i}">
          <span class="chip ${h.kind === "course" ? "info" : h.kind === "resource" ? "ok" : "warn"}">${SEARCH_KIND_LABELS[h.kind] || h.kind}</span>
          <span class="search-hit-main"><b>${escapeHtml(h.title)}</b><small>${escapeHtml(h.subtitle)}</small></span>
        </button>`
      )
      .join("");
    searchResults.querySelectorAll(".search-hit").forEach((el) => {
      el.addEventListener("mousedown", (e) => e.preventDefault());
      el.addEventListener("click", () => openSearchHit(searchHits[parseInt(el.getAttribute("data-index"), 10)]));
    });
  }
  searchResults.hidden = false;
  searchInput.setAttribute("aria-expanded", "true");
  searchActive = searchHits.length ? 0 : -1;
  highlightSearchActive();
}

async function runSearch() {
  const query = searchInput.value.trim();
  if (!query) {
    closeSearch();
    return;
  }
  const seq = ++searchSeq;
  try {
    const hits = await invoke("search_local", { query, limit: 12 });
    if (seq !== searchSeq) return; // resposta antiga
    searchHits = hits || [];
    renderSearchResults(query);
  } catch (err) {
    console.error("Erro na busca:", err);
  }
}

function openSearchHit(hit) {
  if (!hit) return;
  closeSearch();
  searchInput.value = "";
  switchTab("courses");
  openCourseDetail(hit.course_id);
}

if (searchInput) {
  searchInput.addEventListener("input", () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(runSearch, 120);
  });
  searchInput.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (searchResults.hidden || searchHits.length === 0) return;
      e.preventDefault();
      const n = searchHits.length;
      searchActive = (searchActive + (e.key === "ArrowDown" ? 1 : -1) + n) % n;
      highlightSearchActive();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (!searchResults.hidden && searchHits[searchActive]) openSearchHit(searchHits[searchActive]);
    } else if (e.key === "Escape") {
      searchInput.value = "";
      closeSearch();
      searchInput.blur();
    }
  });
  searchInput.addEventListener("focus", () => {
    if (searchInput.value.trim()) runSearch();
  });
  searchInput.addEventListener("blur", () => setTimeout(closeSearch, 120));
  document.addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      searchInput.focus();
      searchInput.select();
    }
  });
}

document.getElementById("btn-refresh").addEventListener("click", refreshData);
updateHeading();
refreshData();

// ScrollSpy para destacar dinamicamente a seção ativa no sumário
const mainScrollEl = document.querySelector(".main-content");
if (mainScrollEl) {
  mainScrollEl.addEventListener("scroll", () => {
    const sections = document.querySelectorAll("[id^='sec-']");
    let currentId = "";
    sections.forEach((sec) => {
      const rect = sec.getBoundingClientRect();
      if (rect.top <= 200) {
        currentId = sec.id;
      }
    });

    const tocLinks = document.querySelectorAll("#course-toc-nav .toc-link");
    tocLinks.forEach((link) => {
      if (currentId && link.getAttribute("data-target-sec") === currentId) {
        link.classList.add("active");
      } else {
        link.classList.remove("active");
      }
    });
  });

  // Intercepta todos os links normais da página para abrir no navegador padrão externamente
  mainScrollEl.addEventListener("click", async (e) => {
    const link = e.target.closest("a");
    if (
      link &&
      link.href &&
      !link.classList.contains("toc-link") &&
      !link.classList.contains("nav-item") &&
      !link.getAttribute("href")?.startsWith("#")
    ) {
      e.preventDefault();
      try {
        await invoke("open_target", { target: link.href });
      } catch (err) {
        console.error("Erro ao abrir link externo:", err);
      }
    }
  });
}

// Logout (Apagar Cookie)
const btnLogout = document.getElementById("btn-logout");
if (btnLogout) {
  btnLogout.addEventListener("click", async () => {
    if (!confirm("Deseja realmente deslogar e apagar a sessão atual?")) return;
    try {
      await invoke("logout");
      const moodleCookie = document.getElementById("moodle-cookie");
      if (moodleCookie) moodleCookie.value = "";
      const sessionResult = document.getElementById("session-result");
      if (sessionResult) {
        sessionResult.className = "result-message";
        sessionResult.style.display = "none";
      }
      await refreshData();
      alert("Sessão desconectada e cookie apagado com sucesso.");
    } catch (err) {
      alert("Erro ao deslogar: " + err);
    }
  });
}

// Exportar dados locais (JSON)
const btnExportData = document.getElementById("btn-export-data");
if (btnExportData) {
  btnExportData.addEventListener("click", async () => {
    try {
      const jsonContent = await invoke("export_data");
      const blob = new Blob([jsonContent], { type: "application/json;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      const dateStr = new Date().toISOString().slice(0, 10);
      a.href = url;
      a.download = `0spof-backup-${dateStr}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      const resBox = document.getElementById("data-management-result");
      if (resBox) {
        resBox.className = "result-message success";
        resBox.innerText = "Exportação de dados em JSON concluída com sucesso!";
        resBox.style.display = "block";
      }
    } catch (err) {
      alert("Erro ao exportar dados: " + err);
    }
  });
}

// Importar dados locais (.json) ou pacote de disciplina (.zip)
const btnImportData = document.getElementById("btn-import-data");
const btnImportCourse = document.getElementById("btn-import-course");
const fileImportInput = document.getElementById("file-import-input");

function triggerImportPicker() {
  if (!fileImportInput) return;
  fileImportInput.value = "";
  fileImportInput.click();
}

if (btnImportData && fileImportInput) {
  btnImportData.addEventListener("click", triggerImportPicker);
}
if (btnImportCourse && fileImportInput) {
  btnImportCourse.addEventListener("click", triggerImportPicker);
}

if (fileImportInput) {
  fileImportInput.addEventListener("change", async (e) => {
    const file = e.target.files && e.target.files[0];
    if (!file) return;

    try {
      const buf = await file.arrayBuffer();
      const summary = await invoke("import_package", new Uint8Array(buf));
      const filesText = summary.files_imported > 0 ? `, Arquivos offline: ${summary.files_imported}` : "";
      const resBox = document.getElementById("data-management-result");
      if (resBox) {
        resBox.className = "result-message success";
        resBox.innerText = `Importação concluída com sucesso! Cursos: ${summary.courses_imported}, Seções: ${summary.sections_imported}, Atividades: ${summary.activities_imported}, Fila: ${summary.outbox_items_imported}${filesText}`;
        resBox.style.display = "block";
      }
      await refreshData();
      alert(
        `Importação concluída com sucesso!\nCursos importados: ${summary.courses_imported}\nSeções: ${summary.sections_imported}\nAtividades: ${summary.activities_imported}\nArquivos offline restaurados: ${summary.files_imported || 0}\nItens na fila: ${summary.outbox_items_imported}`
      );
    } catch (err) {
      alert("Erro ao importar arquivo: " + err);
    }
  });
}

// Exportação seletiva por disciplina (.zip compartilhável)
const courseExportModal = document.getElementById("course-export-modal");
const exportModalTitle = document.getElementById("export-modal-course-title");
const exportModalSearch = document.getElementById("export-modal-search");
const exportModalSections = document.getElementById("export-modal-sections");
const exportToggleActivities = document.getElementById("export-toggle-activities");
const exportIncludeFiles = document.getElementById("export-include-files");
const exportModalSummary = document.getElementById("export-modal-summary");
const btnConfirmCourseExport = document.getElementById("btn-confirm-course-export");

const courseExportState = {
  preview: null,
  selectedIds: new Set(),
  query: "",
  includeFiles: true,
};

function allPreviewItems() {
  if (!courseExportState.preview) return [];
  return courseExportState.preview.sections.flatMap((s) => s.items || []);
}

function itemMatchesExportSearch(item, q) {
  if (!q) return true;
  const hay = normalizeText(
    `${item.title} ${item.activity_type} ${item.file_name || ""} ${item.is_pdf ? "pdf" : ""} ${
      item.is_material ? "material" : "atividade"
    }`
  );
  return hay.includes(q);
}

function syncCourseExportSelectionUi() {
  if (!courseExportState.preview || !exportModalSections) return;

  exportModalSections.querySelectorAll(".export-item-checkbox").forEach((cb) => {
    const id = parseInt(cb.getAttribute("data-item-id"), 10);
    const checked = courseExportState.selectedIds.has(id);
    cb.checked = checked;
    const row = cb.closest(".export-item-row");
    if (row) row.classList.toggle("unchecked", !checked);
  });

  exportModalSections.querySelectorAll(".export-section-checkbox").forEach((secCb) => {
    const secId = parseInt(secCb.getAttribute("data-section-id"), 10);
    const sec = courseExportState.preview.sections.find((s) => s.id === secId);
    const items = sec ? sec.items || [] : [];
    const selectedInSec = items.filter((it) => courseExportState.selectedIds.has(it.id)).length;
    secCb.checked = items.length > 0 && selectedInSec === items.length;
    secCb.indeterminate = selectedInSec > 0 && selectedInSec < items.length;
  });

  const allItems = allPreviewItems();
  const nonMaterialItems = allItems.filter((it) => !it.is_material);
  const selectedActivities = nonMaterialItems.filter((it) => courseExportState.selectedIds.has(it.id)).length;
  if (exportToggleActivities) {
    exportToggleActivities.disabled = nonMaterialItems.length === 0;
    exportToggleActivities.checked =
      nonMaterialItems.length > 0 && selectedActivities === nonMaterialItems.length;
    exportToggleActivities.indeterminate =
      selectedActivities > 0 && selectedActivities < nonMaterialItems.length;
  }

  const chosen = allItems.filter((it) => courseExportState.selectedIds.has(it.id));
  const pdfCount = chosen.filter((it) => it.is_pdf).length;
  const totalBytes = courseExportState.includeFiles
    ? chosen.reduce((acc, it) => acc + (it.has_local_file ? it.file_size_bytes || 0 : 0), 0)
    : 0;
  const pdfPart = pdfCount > 0 ? ` · ${pdfCount} PDF${pdfCount > 1 ? "s" : ""}` : "";
  const sizePart = totalBytes > 0 ? ` · ${formatBytes(totalBytes)}` : "";

  if (exportModalSummary) {
    exportModalSummary.innerText = `${chosen.length} de ${allItems.length} itens selecionados${pdfPart}${sizePart}`;
  }
  if (btnConfirmCourseExport) {
    btnConfirmCourseExport.disabled = chosen.length === 0;
  }
}

function renderCourseExportSections() {
  if (!exportModalSections) return;
  const preview = courseExportState.preview;
  if (!preview || !preview.sections || preview.sections.length === 0) {
    exportModalSections.innerHTML = `<p style="color:var(--muted);padding:12px 0;">Nenhum item encontrado nesta disciplina para exportação.</p>`;
    syncCourseExportSelectionUi();
    return;
  }

  const q = normalizeText(courseExportState.query.trim());
  const visibleSections = preview.sections
    .map((sec) => ({
      sec,
      items: (sec.items || []).filter((it) => itemMatchesExportSearch(it, q)),
    }))
    .filter((entry) => entry.items.length > 0);

  if (visibleSections.length === 0) {
    exportModalSections.innerHTML = `<p style="color:var(--muted);padding:12px 0;">Nenhum item corresponde ao filtro "${escapeHtml(courseExportState.query)}".</p>`;
    syncCourseExportSelectionUi();
    return;
  }

  exportModalSections.innerHTML = visibleSections
    .map(({ sec, items }) => {
      const itemsHtml = items
        .map((item) => {
          const checked = courseExportState.selectedIds.has(item.id);
          const pdfBadge = item.is_pdf ? `<span class="badge-pdf">PDF</span>` : "";
          const kindClass = item.is_material ? "material" : "activity";
          const fileTag = item.has_local_file
            ? `<span class="badge-file-offline">${escapeHtml(item.file_name || "Arquivo offline")} · ${formatBytes(item.file_size_bytes || 0)}</span>`
            : `<span class="badge-link-only">${item.url ? "Apenas link" : "Somente registro"}</span>`;
          const dueTag =
            !item.is_material && item.due_date
              ? `<span class="badge-link-only">Prazo: ${escapeHtml(formatDate(item.due_date))}</span>`
              : "";

          return `
            <label class="export-item-row ${checked ? "" : "unchecked"}">
              <input type="checkbox" class="export-item-checkbox" data-item-id="${item.id}" ${checked ? "checked" : ""}>
              <div class="export-item-info">
                <div class="export-item-top">
                  ${pdfBadge}
                  <span class="export-type-pill ${kindClass}">${getActivityBadge(item.activity_type)}</span>
                  <strong>${escapeHtml(item.title)}</strong>
                </div>
                <div class="export-item-meta">
                  ${fileTag}
                  ${dueTag}
                </div>
              </div>
            </label>
          `;
        })
        .join("");

      return `
        <div class="export-section-block">
          <label class="export-section-header">
            <input type="checkbox" class="export-section-checkbox" data-section-id="${sec.id}">
            <strong>${escapeHtml(sec.name)}</strong>
            <span class="export-section-count">${items.length} ${items.length === 1 ? "item" : "itens"}</span>
          </label>
          <div class="export-section-items">${itemsHtml}</div>
        </div>
      `;
    })
    .join("");

  exportModalSections.querySelectorAll(".export-item-checkbox").forEach((cb) => {
    cb.addEventListener("change", () => {
      const id = parseInt(cb.getAttribute("data-item-id"), 10);
      if (cb.checked) {
        courseExportState.selectedIds.add(id);
      } else {
        courseExportState.selectedIds.delete(id);
      }
      syncCourseExportSelectionUi();
    });
  });

  exportModalSections.querySelectorAll(".export-section-checkbox").forEach((secCb) => {
    secCb.addEventListener("change", () => {
      const secId = parseInt(secCb.getAttribute("data-section-id"), 10);
      const sec = courseExportState.preview.sections.find((s) => s.id === secId);
      if (!sec) return;
      const qNorm = normalizeText(courseExportState.query.trim());
      const targetItems = qNorm
        ? (sec.items || []).filter((it) => itemMatchesExportSearch(it, qNorm))
        : sec.items || [];
      for (const it of targetItems) {
        if (secCb.checked) {
          courseExportState.selectedIds.add(it.id);
        } else {
          courseExportState.selectedIds.delete(it.id);
        }
      }
      syncCourseExportSelectionUi();
    });
  });

  syncCourseExportSelectionUi();
}

function closeCourseExportModal() {
  if (courseExportModal) courseExportModal.classList.add("hidden");
}

async function openCourseExportModal(courseId) {
  if (!courseExportModal) return;
  courseExportState.preview = null;
  courseExportState.selectedIds = new Set();
  courseExportState.query = "";
  courseExportState.includeFiles = true;
  if (exportModalSearch) exportModalSearch.value = "";
  if (exportIncludeFiles) exportIncludeFiles.checked = true;
  if (exportModalTitle) exportModalTitle.innerText = "Carregando disciplina…";
  if (exportModalSections) {
    exportModalSections.innerHTML = `<p style="color:var(--muted);padding:12px 0;">Carregando itens da disciplina…</p>`;
  }
  if (btnConfirmCourseExport) btnConfirmCourseExport.disabled = true;
  courseExportModal.classList.remove("hidden");

  try {
    const preview = await invoke("get_course_export_preview", { courseId });
    courseExportState.preview = preview;
    courseExportState.selectedIds = new Set(
      preview.sections
        .flatMap((s) => s.items || [])
        .filter((it) => it.selected_by_default)
        .map((it) => it.id)
    );
    if (exportModalTitle) exportModalTitle.innerText = preview.fullname;
    renderCourseExportSections();
  } catch (err) {
    closeCourseExportModal();
    alert("Erro ao preparar exportação da disciplina: " + err);
  }
}

if (courseExportModal) {
  document.getElementById("btn-close-export-modal")?.addEventListener("click", closeCourseExportModal);
  document.getElementById("btn-cancel-course-export")?.addEventListener("click", closeCourseExportModal);
  courseExportModal.addEventListener("click", (e) => {
    if (e.target === courseExportModal) closeCourseExportModal();
  });

  if (exportModalSearch) {
    exportModalSearch.addEventListener("input", () => {
      courseExportState.query = exportModalSearch.value;
      renderCourseExportSections();
    });
  }

  document.querySelectorAll(".btn-export-preset").forEach((btn) => {
    btn.addEventListener("click", () => {
      const preset = btn.getAttribute("data-preset");
      const items = allPreviewItems();
      courseExportState.selectedIds.clear();
      if (preset === "materials") {
        items.filter((it) => it.is_material).forEach((it) => courseExportState.selectedIds.add(it.id));
      } else if (preset === "pdfs") {
        items.filter((it) => it.is_pdf).forEach((it) => courseExportState.selectedIds.add(it.id));
      } else if (preset === "all") {
        items.forEach((it) => courseExportState.selectedIds.add(it.id));
      }
      syncCourseExportSelectionUi();
    });
  });

  if (exportToggleActivities) {
    exportToggleActivities.addEventListener("change", () => {
      const nonMaterials = allPreviewItems().filter((it) => !it.is_material);
      for (const it of nonMaterials) {
        if (exportToggleActivities.checked) {
          courseExportState.selectedIds.add(it.id);
        } else {
          courseExportState.selectedIds.delete(it.id);
        }
      }
      syncCourseExportSelectionUi();
    });
  }

  if (exportIncludeFiles) {
    exportIncludeFiles.addEventListener("change", () => {
      courseExportState.includeFiles = exportIncludeFiles.checked;
      syncCourseExportSelectionUi();
    });
  }

  if (btnConfirmCourseExport) {
    btnConfirmCourseExport.addEventListener("click", async () => {
      const preview = courseExportState.preview;
      if (!preview || courseExportState.selectedIds.size === 0) return;

      const originalLabel = btnConfirmCourseExport.innerText;
      btnConfirmCourseExport.disabled = true;
      btnConfirmCourseExport.innerText = "Gerando pacote…";
      try {
        const buf = await invoke("export_course_package", {
          selection: {
            course_id: preview.course_id,
            activity_ids: Array.from(courseExportState.selectedIds),
            include_section_ids: [],
            include_files: courseExportState.includeFiles,
          },
        });
        const blob = new Blob([buf], { type: "application/zip" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = preview.suggested_filename || `0spof-disciplina-${preview.course_id}.zip`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        closeCourseExportModal();
      } catch (err) {
        alert("Erro ao exportar disciplina: " + err);
      } finally {
        btnConfirmCourseExport.innerText = originalLabel;
        syncCourseExportSelectionUi();
      }
    });
  }
}

// Apagar todos os dados
const btnWipeData = document.getElementById("btn-wipe-data");
if (btnWipeData) {
  btnWipeData.addEventListener("click", async () => {
    const confirmed = confirm(
      "Atenção: todos os cursos, seções, atividades e itens na fila de envio serão apagados permanentemente do banco local.\n\nDeseja realmente apagar todos os dados?"
    );
    if (!confirmed) return;

    try {
      await invoke("wipe_all_data");
      const resBox = document.getElementById("data-management-result");
      if (resBox) {
        resBox.className = "result-message success";
        resBox.innerText = "Todos os dados locais foram apagados com sucesso.";
        resBox.style.display = "block";
      }
      await refreshData();
      alert("Todos os dados locais foram apagados com sucesso.");
    } catch (err) {
      alert("Erro ao apagar dados: " + err);
    }
  });
}


// Sincronização com o celular (servidor local + QR code)
let syncInfo = null;

function renderSyncInfo(info) {
  syncInfo = info;
  const qr = document.getElementById("sync-qr");
  const status = document.getElementById("sync-status");
  const addr = document.getElementById("sync-address");
  const toggle = document.getElementById("btn-sync-toggle");
  if (!qr || !status) return;
  qr.innerHTML = info.running && info.qr_svg ? info.qr_svg : "";
  status.innerText = info.running ? `Servidor ativo em "${info.device_name}"` : "Servidor desligado";
  addr.innerText = info.running ? `${info.host}:${info.port}` : "Ligue o servidor para gerar o QR code.";
  toggle.querySelector("span").innerText = info.running ? "Desligar" : "Ligar";
}

async function loadSyncInfo() {
  try {
    renderSyncInfo(await invoke("get_sync_info"));
  } catch (err) {
    console.error("sync info:", err);
  }
}

const btnSyncToggle = document.getElementById("btn-sync-toggle");
if (btnSyncToggle) {
  btnSyncToggle.addEventListener("click", async () => {
    try {
      renderSyncInfo(await invoke(syncInfo && syncInfo.running ? "stop_sync_server" : "start_sync_server"));
    } catch (err) {
      alert("Erro no servidor de sincronização: " + err);
    }
  });
  document.getElementById("btn-sync-rotate").addEventListener("click", async () => {
    try {
      renderSyncInfo(await invoke("rotate_sync_token"));
    } catch (err) {
      alert("Erro ao gerar novo código: " + err);
    }
  });
  document.getElementById("btn-sync-copy").addEventListener("click", () => {
    if (syncInfo && syncInfo.pairing_code) navigator.clipboard.writeText(syncInfo.pairing_code);
  });
  loadSyncInfo();
}

// Verificação leve de atualizações (GitHub Releases)
let latestReleaseUrl = "https://github.com/mastrien/0spof/releases";

async function loadAppVersion() {
  const versionEl = document.getElementById("app-version");
  if (!versionEl) return;
  try {
    const version = await invoke("get_app_version");
    versionEl.innerText = `• ${version}`;
  } catch (_) {}
}

async function checkForUpdates({ silent = true } = {}) {
  const banner = document.getElementById("update-banner");
  const titleEl = document.getElementById("update-banner-title");
  const textEl = document.getElementById("update-banner-text");
  const resBox = document.getElementById("data-management-result");

  try {
    const info = await invoke("check_for_updates");
    latestReleaseUrl = info.release_url || latestReleaseUrl;

    if (info.update_available) {
      if (titleEl) {
        titleEl.innerText = `Nova versão disponível: ${info.latest_version}`;
      }
      if (textEl) {
        textEl.innerText = `(versão atual: ${info.current_version})`;
      }
      if (banner) {
        banner.style.display = "flex";
      }
      if (!silent && resBox) {
        resBox.className = "result-message success";
        resBox.innerText = `Nova versão disponível: ${info.latest_version} (atual: ${info.current_version}).`;
        resBox.style.display = "block";
      }
    } else if (!silent && resBox) {
      resBox.className = "result-message success";
      resBox.innerText = `Você já está usando a versão mais recente (${info.current_version}).`;
      resBox.style.display = "block";
    }
  } catch (err) {
    if (!silent && resBox) {
      resBox.className = "result-message error";
      resBox.innerText = `Não foi possível verificar atualizações: ${err}`;
      resBox.style.display = "block";
    }
  }
}

const btnDownloadUpdate = document.getElementById("btn-download-update");
if (btnDownloadUpdate) {
  btnDownloadUpdate.addEventListener("click", async () => {
    const labelEl = btnDownloadUpdate.querySelector("span");
    const originalText = labelEl ? labelEl.innerText : "Baixar atualização";
    btnDownloadUpdate.disabled = true;
    if (labelEl) labelEl.innerText = "Instalando atualização...";

    try {
      const installed = await invoke("install_update");
      if (!installed) {
        await invoke("open_target", { target: latestReleaseUrl });
      }
    } catch (_) {
      try {
        await invoke("open_target", { target: latestReleaseUrl });
      } catch (openErr) {
        alert("Erro ao abrir página de atualização: " + openErr);
      }
    } finally {
      btnDownloadUpdate.disabled = false;
      if (labelEl) labelEl.innerText = originalText;
    }
  });
}

const btnDismissUpdate = document.getElementById("btn-dismiss-update");
if (btnDismissUpdate) {
  btnDismissUpdate.addEventListener("click", () => {
    const banner = document.getElementById("update-banner");
    if (banner) banner.style.display = "none";
  });
}

const btnCheckUpdates = document.getElementById("btn-check-updates");
if (btnCheckUpdates) {
  btnCheckUpdates.addEventListener("click", async () => {
    btnCheckUpdates.disabled = true;
    try {
      await checkForUpdates({ silent: false });
    } finally {
      btnCheckUpdates.disabled = false;
    }
  });
}

loadAppVersion();
checkForUpdates({ silent: true });