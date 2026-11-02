const invoke = (window.__TAURI__ && window.__TAURI__.core)
  ? window.__TAURI__.core.invoke
  : async (cmd, args) => {
      console.warn("Tauri runtime não detectado. Executando em mock local para o comando:", cmd, args);
      if (cmd === "get_system_status") return { courses_count: 0, deadlines_count: 0, outbox_count: 0 };
      if (cmd === "get_courses") return [];
      if (cmd === "get_outbox_items") return [];
      if (cmd === "get_session_info") return { is_authenticated: false, base_url: null };
      if (cmd === "save_session") return { is_authenticated: true, base_url: args.baseUrl };
      if (cmd === "sync_moodle") return { courses_synced: 1, sections_synced: 2, activities_synced: 3 };
      return { message: "Mock response" };
    };

// Gerenciamento de Abas
const navItems = document.querySelectorAll(".nav-item");
const tabPanes = document.querySelectorAll(".tab-pane");
const tabTitle = document.getElementById("tab-title");

navItems.forEach((btn) => {
  btn.addEventListener("click", () => {
    const target = btn.getAttribute("data-tab");
    navItems.forEach((b) => b.classList.remove("active"));
    tabPanes.forEach((p) => p.classList.remove("active"));

    btn.classList.add("active");
    const pane = document.getElementById(`tab-${target}`);
    if (pane) pane.classList.add("active");

    const titles = {
      dashboard: "Painel de Resiliência",
      courses: "Disciplinas e Prazos Salvos",
      outbox: "Fila de Submissões Pendentes",
      "queue-modal": "Nova Submissão Offline",
      settings: "Conexão Institucional Moodle / SUAP",
    };
    tabTitle.innerText = titles[target] || "0spof";
  });
});

// Atualização de Métricas e Dados
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

async function loadCourses() {
  const container = document.getElementById("courses-list");
  try {
    const courses = await invoke("get_courses");
    if (!courses || courses.length === 0) {
      container.innerHTML = "<p>Nenhum curso cacheado localmente ainda.</p>";
      return;
    }
    container.innerHTML = courses
      .map(
        (c) => `
        <div class="list-item">
          <div>
            <strong>[${c.shortname}]</strong> ${c.fullname}
          </div>
        </div>
      `
      )
      .join("");
  } catch (err) {
    container.innerHTML = `<p class="error">Erro ao listar cursos: ${err}</p>`;
  }
}

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

// Enfileiramento de Submissão
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
    const res = await invoke("enqueue_submission", {
      assignId,
      filePath,
      fetchTsa: checkTsa,
    });

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

// Configuração de Sessão Moodle
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

// Sincronização Online do Moodle
const btnSync = document.getElementById("btn-sync");
btnSync.addEventListener("click", async () => {
  btnSync.disabled = true;
  const originalText = btnSync.innerText;
  btnSync.innerText = "⏳ Sincronizando...";

  try {
    const summary = await invoke("sync_moodle");
    alert(`Sincronização concluída!\nCursos atualizados: ${summary.courses_synced}\nSeções indexadas: ${summary.sections_synced}\nAtividades catalogadas: ${summary.activities_synced}`);
    await refreshData();
  } catch (err) {
    alert("Erro na sincronização: " + err);
  } finally {
    btnSync.disabled = false;
    btnSync.innerText = originalText;
  }
});

// Modal de Comprovante Forense
const proofModal = document.getElementById("proof-modal");
const proofText = document.getElementById("proof-text");
const btnCloseModal = document.getElementById("btn-close-modal");
const btnCopyProof = document.getElementById("btn-copy-proof");

async function openProof(outboxId) {
  try {
    const proof = await invoke("get_submission_proof", { id: parseInt(outboxId, 10) });
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

// Inicialização
refreshData();
