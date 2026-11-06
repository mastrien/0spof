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

// Navegação de Abas
const navItems = document.querySelectorAll(".nav-item");
const tabPanes = document.querySelectorAll(".tab-pane");
const tabTitle = document.getElementById("tab-title");

function switchTab(target) {
  navItems.forEach((b) => b.classList.toggle("active", b.getAttribute("data-tab") === target));
  tabPanes.forEach((p) => p.classList.toggle("active", p.id === `tab-${target}`));

  const titles = {
    dashboard: "Painel de Resiliência",
    courses: "Disciplinas e Prazos Salvos",
    outbox: "Fila de Submissões Pendentes",
    "queue-modal": "Nova Submissão Offline",
    settings: "Conexão Institucional Moodle / SUAP",
  };
  tabTitle.innerText = titles[target] || "0spof";
}

navItems.forEach((btn) => {
  btn.addEventListener("click", () => switchTab(btn.getAttribute("data-tab")));
});

async function refreshData() {
  try {
    const status = await invoke("get_system_status");
    document.getElementById("metric-courses").innerText = status.courses_count;
    document.getElementById("metric-deadlines").innerText = status.deadlines_count;
    document.getElementById("metric-outbox").innerText = status.outbox_count;
    document.getElementById("outbox-count").innerText = status.outbox_count;

    const sessionInfo = await invoke("get_session_info");
    const dot = document.getElementById("session-dot");
    const txt = document.getElementById("session-text");

    if (sessionInfo.is_authenticated) {
      dot.className = "dot online";
      txt.innerText = "Conectado ao Moodle";
      const urlInput = document.getElementById("moodle-url");
      if (urlInput && !urlInput.value && sessionInfo.base_url) {
        urlInput.value = sessionInfo.base_url;
      }
    } else {
      dot.className = "dot offline";
      txt.innerText = "Sessão Desconectada";
    }

    await loadDeadlines();
    await loadCourses();
    await loadOutbox();
  } catch (err) {
    console.error("Erro ao carregar status:", err);
  }
}

async function loadDeadlines() {
  const container = document.getElementById("dashboard-deadlines-list");
  if (!container) return;
  try {
    const deadlines = await invoke("get_upcoming_deadlines");
    if (!deadlines || deadlines.length === 0) {
      container.innerHTML = `<p style="color:var(--muted);font-size:13px;padding:8px 0;">Nenhum prazo pendente no momento. Você está em dia!</p>`;
      return;
    }
    container.innerHTML = deadlines
      .map(
        (d) => `
        <div class="list-item">
          <div class="item-content">
            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:4px;">
              <span class="status-badge pending-assignment">
                <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
                Pendente de envio
              </span>
              <strong>${d.title}</strong>
              <small style="color:var(--muted);">(ID #${d.id})</small>
            </div>
            ${
              d.due_date
                ? `<span class="due-date-tag"><svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg> Entrega até: ${formatDate(d.due_date)}</span>`
                : `<span style="font-size:12px;color:var(--muted);">Sem prazo limite cadastrado</span>`
            }
          </div>
          <div class="item-actions">
            <button class="btn btn-primary btn-queue-assign" data-assign-id="${d.id}" style="height:32px;padding:0 12px;font-size:12px;">
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              Enfileirar entrega (#${d.id})
            </button>
            ${
              d.url
                ? `<button class="btn-discreet btn-open-url" data-target="${d.url}" title="Abrir tarefa no Moodle institucional">
                    <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                    Abrir no Moodle
                  </button>`
                : ""
            }
          </div>
        </div>
      `
      )
      .join("");

    container.querySelectorAll(".btn-queue-assign").forEach((btn) => {
      btn.addEventListener("click", () => {
        const assignId = btn.getAttribute("data-assign-id");
        document.getElementById("assign-id").value = assignId;
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
  } catch (err) {
    container.innerHTML = `<p class="error">Erro ao carregar prazos: ${err}</p>`;
  }
}

let cachedCourses = [];

async function loadCourses() {
  const container = document.getElementById("courses-list");
  try {
    cachedCourses = await invoke("get_courses");
    if (!cachedCourses || cachedCourses.length === 0) {
      container.innerHTML = "<p>Nenhum curso cacheado localmente ainda. Clique em 'Sincronizar Moodle'.</p>";
      return;
    }
    container.innerHTML = cachedCourses
      .map(
        (c) => `
        <div class="list-item course-row" data-course-id="${c.id}" style="cursor:pointer;">
          <div class="item-content">
            <strong>${c.fullname}</strong><br>
            <small>ID #${c.id} • ${c.sections_count || 0} seções • ${c.activities_count || 0} atividades</small>
          </div>
          <div class="item-actions">
            <button class="btn btn-secondary btn-open-course" data-course-id="${c.id}" style="height:32px;padding:0 12px;font-size:12px;">
              Abrir disciplina
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2.2"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>
            </button>
          </div>
        </div>
      `
      )
      .join("");

    container.querySelectorAll(".course-row").forEach((row) => {
      row.addEventListener("click", () => {
        const courseId = parseInt(row.getAttribute("data-course-id"), 10);
        openCourseDetail(courseId);
      });
    });
  } catch (err) {
    container.innerHTML = `<p class="error">Erro ao listar cursos: ${err}</p>`;
  }
}

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

async function openCourseDetail(courseId) {
  const course = cachedCourses.find((c) => c.id === courseId);
  document.getElementById("courses-master-view").style.display = "none";
  document.getElementById("course-detail-view").style.display = "block";
  document.getElementById("course-detail-short").innerText = course ? `CURSO #${course.id} • ${course.shortname}` : `CURSO #${courseId}`;
  document.getElementById("course-detail-title").innerText = course ? course.fullname : "Disciplina";

  const tocNav = document.getElementById("course-toc-nav");
  const sectionsContainer = document.getElementById("course-sections-list");

  tocNav.innerHTML = "<p style='font-size:12px;color:var(--muted);'>Carregando sumário…</p>";
  sectionsContainer.innerHTML = "<p style='font-size:13px;color:var(--muted);'>Carregando seções…</p>";

  try {
    const sections = await invoke("get_course_details", { courseId });
    if (!sections || sections.length === 0) {
      tocNav.innerHTML = "<p style='font-size:12px;color:var(--muted);'>Nenhuma seção.</p>";
      sectionsContainer.innerHTML = "<p>Nenhuma seção cadastrada nesta disciplina.</p>";
      return;
    }

    tocNav.innerHTML = sections
      .map(
        (sec, idx) => `
        <a class="toc-link" data-target-sec="sec-${sec.id}" title="${sec.name}">
          ${idx + 1}. ${sec.name}
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

    sectionsContainer.innerHTML = sections
      .map((sec) => {
        const summaryHtml = sec.summary ? `<div class="section-summary">${sec.summary}</div>` : "";
        const activitiesHtml =
          !sec.activities || sec.activities.length === 0
            ? `<p style="color:#8994aa;font-size:12px;">Nenhuma atividade nesta seção.</p>`
            : sec.activities
                .map((act) => {
                  const descHtml = act.description
                    ? `<div class="activity-description">${act.description}</div>`
                    : "";
                  const hasLocalFile = act.local_file_path && act.local_file_path.length > 0;
                  const isVideo = (act.content_mime && act.content_mime.startsWith("video/")) || (act.url && /\.(mp4|webm|mkv)(\?|$)/i.test(act.url));

                  let actionsHtml = "";
                  if (act.activity_type === "assign") {
                    actionsHtml += `
                      <button class="btn btn-primary btn-queue-assign" data-assign-id="${act.id}" style="height:32px;padding:0 10px;font-size:12px;">
                        <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                        Enfileirar entrega (#${act.id})
                      </button>
                    `;
                  }

                  if (hasLocalFile) {
                    actionsHtml += `
                      <button class="btn btn-secondary btn-open-local" data-target="${act.local_file_path}" style="height:32px;padding:0 10px;font-size:12px;">
                        <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                        Abrir cópia offline
                      </button>
                    `;
                  } else if (isVideo) {
                    actionsHtml += `
                      <button class="btn btn-secondary btn-download-media" data-act-id="${act.id}" data-video="true" style="height:32px;padding:0 10px;font-size:12px;">
                        <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                        Baixar vídeo offline
                      </button>
                    `;
                  } else if (act.activity_type === "resource" || act.activity_type === "page") {
                    actionsHtml += `
                      <button class="btn btn-secondary btn-download-media" data-act-id="${act.id}" data-video="false" style="height:32px;padding:0 10px;font-size:12px;">
                        <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                        Baixar recurso
                      </button>
                    `;
                  }

                  if (act.url) {
                    if (act.activity_type === "url") {
                      actionsHtml += `
                        <button class="btn btn-primary btn-open-url" data-target="${act.url}" style="height:32px;padding:0 10px;font-size:12px;">
                          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                          Acessar link externo
                        </button>
                      `;
                    } else {
                      actionsHtml += `
                        <button class="btn-discreet btn-open-url" data-target="${act.url}" title="Abrir página desta atividade no Moodle institucional">
                          <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                          Abrir no Moodle
                        </button>
                      `;
                    }
                  }

                  let badgeHtml = "";
                  if (act.activity_type === "assign") {
                    if (act.is_submitted) {
                      badgeHtml = `<span class="status-badge submitted"><svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg> Entregue</span>`;
                    } else {
                      badgeHtml = `<span class="status-badge pending-assignment"><svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg> Pendente</span>`;
                    }
                  } else {
                    badgeHtml = `<span class="status-badge pending">${getActivityBadge(act.activity_type)}</span>`;
                  }

                  const dueDateHtml =
                    act.activity_type === "assign" && act.due_date
                      ? `<span class="due-date-tag"><svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg> Prazo: ${formatDate(act.due_date)}</span>`
                      : "";

                  return `
                    <div class="list-item activity-card">
                      <div class="item-content">
                        <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:6px;">
                          ${badgeHtml}
                          <strong>${act.title}</strong> <small style="color:var(--muted);">(ID #${act.id})</small>
                          ${dueDateHtml}
                        </div>
                        ${descHtml}
                      </div>
                      ${actionsHtml ? `<div class="item-actions">${actionsHtml}</div>` : ""}
                    </div>
                  `;
                })
                .join("");

        return `
          <div id="sec-${sec.id}" style="border:1px solid #e8ebf2;border-radius:12px;padding:16px 18px;background:#fff;scroll-margin-top:20px;">
            <h3 style="font-size:16px;color:#18223a;margin-bottom:6px;">${sec.name}</h3>
            ${summaryHtml}
            <div class="section-activities">${activitiesHtml}</div>
          </div>
        `;
      })
      .join("");

    sectionsContainer.querySelectorAll(".btn-queue-assign").forEach((btn) => {
      btn.addEventListener("click", () => {
        const assignId = btn.getAttribute("data-assign-id");
        document.getElementById("assign-id").value = assignId;
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
    sectionsContainer.innerHTML = `<p class="error">Erro ao carregar detalhes do curso: ${err}</p>`;
  }
}

document.getElementById("btn-back-courses").addEventListener("click", () => {
  document.getElementById("course-detail-view").style.display = "none";
  document.getElementById("courses-master-view").style.display = "block";
});

async function loadOutbox() {
  const container = document.getElementById("outbox-list");
  try {
    const items = await invoke("get_outbox_items");
    if (!items || items.length === 0) {
      container.innerHTML = "<p>Nenhuma tarefa na fila de envio.</p>";
      return;
    }
    container.innerHTML = items
      .map(
        (item) => `
        <div class="list-item">
          <div class="item-content">
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;">
              <span class="status-badge ${item.status}">${item.status}</span>
              <strong>Tarefa #${item.assign_id}: ${item.filename}</strong>
            </div>
            <small>Tentativas: ${item.retry_count} | Hash: ${item.file_hash.substring(0, 16)}...</small>
          </div>
          <div class="item-actions">
            <button class="btn btn-secondary btn-proof" data-id="${item.id}" style="height:30px;padding:0 10px;font-size:12px;">
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
              Ver Dossiê
            </button>
          </div>
        </div>
      `
      )
      .join("");

    document.querySelectorAll(".btn-proof").forEach((btn) => {
      btn.addEventListener("click", () => openProof(btn.getAttribute("data-id")));
    });
  } catch (err) {
    container.innerHTML = `<p class="error">Erro ao listar fila: ${err}</p>`;
  }
}

const formEnqueue = document.getElementById("form-enqueue");
const resultBox = document.getElementById("enqueue-result");

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
    formEnqueue.reset();
    document.getElementById("check-tsa").checked = true;
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
    if (summary.courses_synced === 0) {
      alert("Sincronização concluída, mas nenhum curso foi encontrado. Verifique seu login no Moodle.");
    } else {
      alert(
        `Sincronização concluída com sucesso!\nCursos atualizados: ${summary.courses_synced}\nSeções indexadas: ${summary.sections_synced}\nAtividades catalogadas: ${summary.activities_synced}\nArquivos/PDFs cacheados offline: ${summary.resources_cached || 0}`
      );
    }
    await refreshData();
  } catch (err) {
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

async function openProof(outboxId) {
  try {
    const proof = await invoke("get_submission_proof", { outboxId: parseInt(outboxId, 10) });
    proofText.innerText = proof.formatted_report;
    proofModal.classList.remove("hidden");
  } catch (err) {
    alert("Erro ao obter comprovante: " + err);
  }
}

btnCloseModal.addEventListener("click", () => proofModal.classList.add("hidden"));
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

document.getElementById("btn-refresh").addEventListener("click", refreshData);
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
