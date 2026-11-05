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

    await loadCourses();
    await loadOutbox();
  } catch (err) {
    console.error("Erro ao carregar status:", err);
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
          <div>
            <strong>${c.fullname}</strong><br>
            <small>ID #${c.id} • ${c.sections_count || 0} seções • ${c.activities_count || 0} atividades</small>
          </div>
          <button class="btn btn-secondary btn-open-course" data-course-id="${c.id}">Abrir disciplina →</button>
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
  const map = {
    assign: "📝 Tarefa",
    resource: "📄 Arquivo/Slide",
    folder: "📁 Pasta",
    url: "🔗 Link Externo",
    forum: "💬 Fórum",
    quiz: "✅ Questionário",
    page: "📃 Página",
  };
  return map[actType] || `📌 ${actType}`;
}

async function openCourseDetail(courseId) {
  const course = cachedCourses.find((c) => c.id === courseId);
  document.getElementById("courses-master-view").style.display = "none";
  document.getElementById("course-detail-view").style.display = "block";
  document.getElementById("course-detail-short").innerText = course ? `CURSO #${course.id} • ${course.shortname}` : `CURSO #${courseId}`;
  document.getElementById("course-detail-title").innerText = course ? course.fullname : `Disciplina #${courseId}`;

  const sectionsContainer = document.getElementById("course-sections-list");
  sectionsContainer.innerHTML = "<p>Carregando seções e atividades…</p>";

  try {
    const sections = await invoke("get_course_details", { courseId });
    if (!sections || sections.length === 0) {
      sectionsContainer.innerHTML = "<p>Nenhuma seção encontrada para esta disciplina.</p>";
      return;
    }

    sectionsContainer.innerHTML = sections
      .map((sec) => {
        const summaryHtml = sec.summary ? `<p style="color:#6d7890;margin:4px 0 10px;font-size:13px;">${sec.summary}</p>` : "";
        const activitiesHtml =
          !sec.activities || sec.activities.length === 0
            ? `<p style="color:#8994aa;font-size:12px;">Nenhuma atividade nesta seção.</p>`
            : sec.activities
                .map((act) => {
                  const descHtml = act.description
                    ? `<div style="margin-top:6px;padding:8px 10px;background:#f4f6fb;border-radius:7px;font-size:12px;color:#344a78;">${act.description}</div>`
                    : "";
                  const hasLocalFile = act.local_file_path && act.local_file_path.length > 0;
                  const isVideo = (act.content_mime && act.content_mime.startsWith("video/")) || (act.url && /\.(mp4|webm|mkv)(\?|$)/i.test(act.url));

                  let actionsHtml = "";
                  if (act.activity_type === "assign") {
                    actionsHtml += `<button class="btn btn-primary btn-queue-assign" data-assign-id="${act.id}" style="padding:6px 10px;font-size:12px;">⌁ Enfileirar entrega (#${act.id})</button>`;
                  }
                  if (hasLocalFile) {
                    actionsHtml += `<button class="btn btn-secondary btn-open-local" data-target="${act.local_file_path}" style="padding:6px 10px;font-size:12px;">📂 Abrir cópia offline (${act.content_mime || "arquivo"})</button>`;
                  } else if (isVideo) {
                    actionsHtml += `<button class="btn btn-secondary btn-download-media" data-act-id="${act.id}" data-video="true" style="padding:6px 10px;font-size:12px;">🎬 Baixar vídeo offline</button>`;
                  } else if (act.activity_type === "resource" || act.activity_type === "page") {
                    actionsHtml += `<button class="btn btn-secondary btn-download-media" data-act-id="${act.id}" data-video="false" style="padding:6px 10px;font-size:12px;">⬇ Baixar recurso offline</button>`;
                  }
                  if (act.url) {
                    actionsHtml += `<button class="btn btn-ghost btn-open-url" data-target="${act.url}" style="padding:6px 10px;font-size:12px;">🌐 Abrir no Moodle</button>`;
                  }

                  return `
                    <div class="list-item" style="align-items:flex-start;flex-direction:column;gap:8px;margin-top:8px;">
                      <div style="width:100%;display:flex;justify-content:space-between;align-items:center;gap:12px;">
                        <div>
                          <span class="status-badge pending" style="margin-right:6px;">${getActivityBadge(act.activity_type)}</span>
                          <strong>${act.title}</strong> <small>(ID #${act.id})</small>
                        </div>
                        <div style="display:flex;gap:6px;flex-wrap:wrap;">${actionsHtml}</div>
                      </div>
                      ${descHtml}
                    </div>
                  `;
                })
                .join("");

        return `
          <div style="border:1px solid #e8ebf2;border-radius:12px;padding:16px 18px;background:#fff;margin-bottom:12px;">
            <h3 style="font-size:16px;color:#18223a;margin-bottom:6px;">${sec.name}</h3>
            ${summaryHtml}
            <div>${activitiesHtml}</div>
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
        const originalText = btn.innerText;
        btn.disabled = true;
        btn.innerText = "⏳ Baixando...";
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
          btn.innerText = originalText;
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
          <div>
            <strong>Tarefa #${item.assign_id}:</strong> ${item.filename}<br>
            <small>Tentativas: ${item.retry_count} | Hash: ${item.file_hash.substring(0, 16)}...</small>
          </div>
          <div>
            <span class="status-badge ${item.status}">${item.status}</span>
            <button class="btn btn-secondary btn-proof" data-id="${item.id}" style="margin-left: 10px; padding: 6px 10px;">📜 Ver Dossiê</button>
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
  const originalText = btnSync.innerText;
  btnSync.innerText = "⏳ Sincronizando...";

  try {
    const summary = await invoke("sync_moodle");
    if (summary.courses_synced === 0) {
      alert("Sincronização concluída, mas nenhum curso foi encontrado. Verifique seu cookie MoodleSession.");
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
    btnSync.innerText = originalText;
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

document.getElementById("btn-refresh").addEventListener("click", refreshData);
refreshData();
