# Moodle: Guia Técnico de Arquitetura, APIs, Plugins e Segurança

Este documento serve como fonte de referência técnica detalhada sobre o ecossistema Moodle (*Modular Object-Oriented Dynamic Learning Environment*), sua arquitetura interna, mecanismos de autenticação, consumo de APIs, funcionamento offline e diretrizes de conformidade (LGPD/Segurança).

---

## 1. Visão Geral e Arquitetura

O Moodle é uma plataforma web em PHP desenhada sob uma arquitetura modular orientada a eventos. O núcleo (*core*) é enxuto em termos de regras de negócio específicas; quase todas as funcionalidades reais existem sob a forma de subsistemas ou plugins.

### 1.1. Camadas do Sistema

```
                  +-----------------------------------------+
                  |   Clientes (Browser, Mobile App, API)   |
                  +-----------------------------------------+
                                       | HTTP(S)
                  +-----------------------------------------+
                  |           Web Server (Nginx/Apache)     |
                  +-----------------------------------------+
                                       |
                  +-----------------------------------------+
                  |                 PHP-FPM                 |
                  |  +-----------------------------------+  |
                  |  |            Moodle Core            |  |
                  |  |  (Router, Session, Access Control)|  |
                  |  +-----------------------------------+  |
                  |  |       Subsystems & Plugins        |  |
                  |  | (mod, theme, auth, enrol, block)  |  |
                  |  +-----------------------------------+  |
                  |  |  MUC (Moodle Universal Cache)     |  |
                  |  +-----------------------------------+  |
                  |  |         Database Abstraction      |  |
                  |  +-----------------------------------+  |
                  +-----------------------------------------+
                          |                     |
         +----------------+--+               +--+----------------+
         |     Database      |               |    Moodledata     |
         | (PGSQL, MySQL)    |               | (Files, Sessions, |
         |                   |               |  Filedir SHA-1)   |
         +-------------------+               +-------------------+
```

### 1.2. Componentes Fundamentais
* **`dirroot` (Web Root):** Contém os scripts PHP acessíveis publicamente (código-fonte, controladores e assets).
* **`dataroot` (`moodledata`):** Diretório crítico localizado **fora** do alcance direto do servidor web (`DocumentRoot`). Armazena:
  * `filedir/`: Armazenamento de arquivos baseado no hash SHA-1 do conteúdo (*content-addressable storage*), evitando arquivos duplicados.
  * Sessões de usuários (caso configurado para disco em vez de Redis/DB).
  * Cache estático e compilado (CSS, JS, templates Mustache).
  * Backups de cursos e logs temporários.
* **Banco de Dados Relacional:**
  * Usa o prefixo padrão `mdl_` em todas as tabelas.
  * Todas as operações passam pela classe global `$DB` (ex: `$DB->get_record()`, `$DB->get_records_sql()`), garantindo compatibilidade entre PostgreSQL, MariaDB/MySQL, MSSQL e Oracle.
* **MUC (Moodle Universal Cache):**
  * Mecanismo de cache em múltiplas camadas (Aplicação, Sessão e Request).
  * Suporta backends como memória local, APCu, Redis e Memcached.
* **Mecanismo de Tarefas Assíncronas (Task API):**
  * **Scheduled Tasks:** Tarefas periódicas (limpeza de sessões, digest de fórum, sync LDAP).
  * **Ad-hoc Tasks:** Filas de processamento assíncrono (envio de notificações pesadas, conversão de PDFs de tarefas, cálculos de notas). Executadas pelo script CLI `admin/cli/cron.php`.

---

## 2. Autenticação, Sessões e Consumo de Dados

Para qualquer aplicação externa ou cliente interagir com o Moodle, entender a camada de acesso é indispensável.

### 2.1. Sessões Tradicionais (Web)
* O Moodle autentica o usuário via formulário de login padrão (`login/index.php`) ou SSO/OAuth2.
* Cria um cookie de sessão chamado `MoodleSession`.
* Cada requisição web valida permissões através do sistema de **Contextos** (`CONTEXT_SYSTEM`, `CONTEXT_COURSECAT`, `CONTEXT_COURSE`, `CONTEXT_MODULE`, `CONTEXT_USER`).

### 2.2. Web Services e REST API Oficial
O Moodle possui uma camada completa de Web Services (`/webservice/rest/server.php`) ativada via configurações de administração:
1. **Protocolos Suportados:** REST (JSON ou XML), SOAP e XML-RPC (legado).
2. **Métodos de Autenticação na API:**
   * **Token Estático (Gerado no Perfil/Admin):** Um token gerado manualmente com permissões atribuídas a um serviço específico.
   * **Token Dinâmico via Login (Moodle Mobile Flow):**
     * Endpoint: `/login/token.php?username=XXX&password=YYY&service=moodle_mobile_app`
     * Retorna um JSON com `{"token": "...", "privatetoken": "..."}`.
     * Este é o mecanismo oficial que o aplicativo móvel do Moodle usa para autenticação direta.
3. **Exemplos de Funções da Web Service API:**
   * `core_course_get_contents`: Retorna todas as seções, módulos, arquivos, descrições e atividades de um curso.
   * `core_enrol_get_users_courses`: Lista todos os cursos nos quais o usuário logado está matriculado.
   * `mod_assign_get_assignments`: Obtém metadados de tarefas, prazos de entrega (*due dates*) e configurações.
   * `mod_assign_get_submission_status`: Detalha se o aluno já enviou a tarefa, nota, feedbacks e arquivos anexados.
   * `mod_assign_save_submission`: Permite o envio/atualização de arquivos ou texto online para uma tarefa.
   * `core_webservice_get_site_info`: Informações da versão do Moodle, usuário logado, funcionalidades habilitadas e permissões.

### 2.3. Web Scraping & Sessão Reversa (Caso a API esteja desativada)
Muitas universidades públicas e privadas desabilitam os Web Services por "segurança" ou descuido de configuração. Nesses casos:
* Uma aplicação cliente precisa simular o fluxo de login via HTTP:
  1. Acessa `/login/index.php` e extrai o token anti-CSRF (`logintoken`).
  2. Envia requisição `POST` com `username`, `password` e `logintoken`.
  3. Captura o cookie `MoodleSession`.
  4. Realiza requisições subsequentes parseando o HTML via DOM/Regex (ou capturando endpoints AJAX internos como `/lib/ajax/service.php` com o parâmetro `sesskey`).

---

## 3. O Modelo Offline do Moodle Mobile Oficial

O aplicativo oficial (Moodle App, baseado em Apache Cordova / Ionic / Angular) possui um dos sistemas offline mais bem documentados do ecossistema:

1. **Armazenamento Local:** Usa SQLite nativo (via plugins Cordova) e IndexedDB para persistir:
   * Dados do site e metadados dos cursos.
   * Conteúdos baixados (PDFs, vídeos, páginas HTML).
   * Fila de ações pendentes (*queue*).
2. **Fila de Sincronização (Outbox):**
   * Quando o aluno realiza uma ação sem conectividade (ex: responder um quiz offline se suportado, postar no fórum, ou salvar rascunho de tarefa), a operação é gravada na tabela local de pendências com um timestamp e os payloads necessários.
   * Um processo em background monitora a rede. Ao detectar retorno do sinal (`navigator.onLine` ou ping bem-sucedido no endpoint do Moodle), os itens da fila são despachados sequencialmente.
   * Em caso de conflito de versão ou erro 500 no servidor, a ação é marcada com erro para revisão manual do aluno.

---

## 4. Ecossistema de Plugins: Funcionamento e Ciclo de Vida

O Moodle possui mais de 40 categorias de plugins. Os mais comuns:

| Tipo | Diretório | Descrição |
| :--- | :--- | :--- |
| **Atividades** | `/mod/<nome>` | Adiciona novos tipos de atividades avaliativas ou recursos (ex: Quiz, Assign, BigBlueButton). |
| **Locais** | `/local/<nome>` | Lógica de negócio personalizada, tarefas agendadas customizadas, extensões de API. |
| **Temas** | `/theme/<nome>` | Modificam a interface visual (CSS, SCSS, Mustache templates, JavaScript AMD/ES6). |
| **Autenticação** | `/auth/<nome>` | Controla o processo de login (ex: LDAP, SAML, Google OAuth). |
| **Matrícula** | `/enrol/<nome>` | Regras para ingresso de alunos em cursos. |
| **Blocos** | `/block/<nome>` | Widgets e painéis laterais na interface do usuário. |

### 4.1. Requisitos para Instalação de Plugins
* **Acesso Administrativo ao Servidor:** Para instalar qualquer plugin em uma instância do Moodle, é **estritamente obrigatório** ter privilégios de **Administrador do Moodle** e/ou acesso de escrita ao sistema de arquivos do servidor (`dirroot`).
* **Processo de Upgrade de Banco:** Toda alteração de plugin aciona a execução do script `version.php` e `db/upgrade.php`, rodando migrações DDL (`XMLDB`) no banco de dados da instituição.

---

## 5. Aspectos de Segurança, LGPD e Compliance

Em ambientes educacionais, o manuseio de dados acadêmicos envolve exigências regulatórias severas:

### 5.1. Classificação de Dados no Moodle
* **Dados Pessoais (Art. 5º, I, LGPD):** Nome, e-mail, foto de perfil, IP de acesso, histórico de navegação nos cursos.
* **Dados Sensíveis / Protegidos:**
  * Avaliações, notas e relatórios de frequência.
  * Tarefas e arquivos acadêmicos enviados.
  * Informações de menores de 18 anos (Art. 14 da LGPD exige tratamento específico, consentimento e melhor interesse do menor).

### 5.2. Papéis e Responsabilidades (LGPD)
* **Controlador (*Data Controller*):** A instituição de ensino. É quem toma as decisões sobre o tratamento dos dados e detém a relação jurídica com o aluno.
* **Operador (*Data Processor*):** Terceiro que trata dados em nome do controlador (ex: servidores de hospedagem contratados).
* **Terceiro Não Autorizado:** Qualquer servidor ou pessoa física externa que espelhe, capture ou centralize dados de alunos e notas sem convênio, contrato ou amparo legal explícito com o Controlador.

### 5.3. Subsistema de Privacidade do Moodle (*Privacy API*)
Desde a versão 3.5 (introduzida para GDPR/LGPD):
* Todo plugin é obrigado a implementar a interface `\core_privacy\local\metadata\null_provider` ou `\core_privacy\local\request\plugin\provider`.
* Permite exportação completa dos dados do usuário em formato legível e direito ao esquecimento (*Data deletion requests*).
