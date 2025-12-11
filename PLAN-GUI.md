# PLAN-GUI.md - SKIS GUI (Tauri)

A compact Tauri-based browser/editor for SKIS issues. Follows the ghost project architecture: thin GUI layer over the existing `ski` library.

## Architecture

```
ski/
├── Cargo.toml              # Workspace root (add gui member)
├── src/                    # Existing library + CLI (unchanged)
│   ├── lib.rs
│   ├── main.rs
│   └── ...
└── gui/                    # New: Tauri GUI
    ├── src-tauri/
    │   ├── Cargo.toml      # Depends on ski library
    │   ├── src/
    │   │   ├── main.rs     # Tauri entry point
    │   │   └── lib.rs      # Tauri commands (thin wrapper)
    │   ├── tauri.conf.json
    │   ├── capabilities/
    │   └── icons/
    └── src/
        ├── index.html      # Single-page app
        ├── styles.css      # Compact styling
        └── main.js         # Vanilla JS, Tauri IPC
```

### Design Principles

1. **Minimal core impact**: GUI is a separate workspace member, `ski` library unchanged
2. **Thin Tauri layer**: Commands just marshal data between JS and `ski::db::*`
3. **Vanilla JS**: No framework, keeps bundle small and simple
4. **Compact UI**: Dense layout, no excessive whitespace, 13px base font
5. **Semantic colors**: Reuse CLI colors (bug=red, task=blue, epic=magenta, etc.)

---

## Phase 1: Project Setup

### 1.1 Convert to Workspace

Update root `Cargo.toml`:

```toml
[workspace]
members = [".", "gui/src-tauri"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
ski = { path = "." }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[package]
name = "skis"
version.workspace = true
edition.workspace = true
# ... rest unchanged
```

### 1.2 Create GUI Cargo.toml

`gui/src-tauri/Cargo.toml`:

```toml
[package]
name = "skis-gui"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
ski.workspace = true
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-shell = "2"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

### 1.3 Tauri Configuration

`gui/src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SKIS",
  "version": "0.1.0",
  "identifier": "com.skis.issue-tracker",
  "build": {
    "frontendDist": "../src"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [{
      "title": "SKIS",
      "width": 900,
      "height": 650,
      "resizable": true,
      "minWidth": 600,
      "minHeight": 400
    }]
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

---

## Phase 2: Tauri Backend Commands

### 2.1 Command Structure

`gui/src-tauri/src/lib.rs`:

```rust
use ski::db::{self, SkisDb};
use ski::models::*;
use serde::{Deserialize, Serialize};

// Response wrapper for all commands
#[derive(Serialize)]
struct Response<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

// Issue with labels for display
#[derive(Serialize)]
struct IssueWithLabels {
    #[serde(flatten)]
    issue: Issue,
    labels: Vec<LabelView>,
    linked_issues: Vec<LinkedIssueRef>,
}

#[tauri::command]
fn list_issues(
    state: Option<String>,
    issue_type: Option<String>,
    labels: Vec<String>,
    search: Option<String>,
) -> Response<Vec<Issue>> { ... }

#[tauri::command]
fn get_issue(id: i64) -> Response<IssueWithLabels> { ... }

#[tauri::command]
fn create_issue(
    title: String,
    body: Option<String>,
    issue_type: String,
    labels: Vec<String>,
) -> Response<Issue> { ... }

#[tauri::command]
fn update_issue(
    id: i64,
    title: Option<String>,
    body: Option<String>,
    issue_type: Option<String>,
) -> Response<Issue> { ... }

#[tauri::command]
fn close_issue(id: i64, reason: String, comment: Option<String>) -> Response<Issue> { ... }

#[tauri::command]
fn reopen_issue(id: i64) -> Response<Issue> { ... }

#[tauri::command]
fn delete_issue(id: i64) -> Response<()> { ... }

#[tauri::command]
fn restore_issue(id: i64) -> Response<Issue> { ... }

#[tauri::command]
fn add_comment(issue_id: i64, body: String) -> Response<Comment> { ... }

#[tauri::command]
fn get_comments(issue_id: i64) -> Response<Vec<Comment>> { ... }

#[tauri::command]
fn list_labels() -> Response<Vec<Label>> { ... }

#[tauri::command]
fn create_label(
    name: String,
    description: Option<String>,
    color: Option<String>,
) -> Response<Label> { ... }

#[tauri::command]
fn delete_label(name: String) -> Response<()> { ... }

#[tauri::command]
fn add_label_to_issue(issue_id: i64, label: String) -> Response<()> { ... }

#[tauri::command]
fn remove_label_from_issue(issue_id: i64, label: String) -> Response<()> { ... }

#[tauri::command]
fn link_issues(a: i64, b: i64) -> Response<()> { ... }

#[tauri::command]
fn unlink_issues(a: i64, b: i64) -> Response<()> { ... }

#[tauri::command]
fn select_directory() -> Response<String> { ... }  // File dialog

#[tauri::command]
fn get_current_dir() -> Response<String> { ... }
```

### 2.2 Entry Point

`gui/src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    skis_gui::run();
}
```

Register commands in `lib.rs`:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            list_issues,
            get_issue,
            create_issue,
            update_issue,
            close_issue,
            reopen_issue,
            delete_issue,
            restore_issue,
            add_comment,
            get_comments,
            list_labels,
            create_label,
            delete_label,
            add_label_to_issue,
            remove_label_from_issue,
            link_issues,
            unlink_issues,
            select_directory,
            get_current_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error running SKIS");
}
```

---

## Phase 3: Frontend - HTML Structure

### 3.1 Layout

`gui/src/index.html`:

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>SKIS</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div class="app">
    <!-- Header: directory selector + filters -->
    <header class="header">
      <div class="dir-selector">
        <input type="text" id="dir-path" readonly placeholder="Select directory...">
        <button id="btn-browse">Browse</button>
      </div>
      <div class="filters">
        <select id="filter-state">
          <option value="open">Open</option>
          <option value="closed">Closed</option>
          <option value="all">All</option>
        </select>
        <select id="filter-type">
          <option value="">All Types</option>
          <option value="epic">Epic</option>
          <option value="task">Task</option>
          <option value="bug">Bug</option>
          <option value="request">Request</option>
        </select>
        <input type="text" id="filter-search" placeholder="Search...">
        <button id="btn-new" class="btn-primary">+ New</button>
      </div>
    </header>

    <!-- Main: split view -->
    <main class="main">
      <!-- Left: issue list -->
      <aside class="issue-list">
        <div id="issues"></div>
      </aside>

      <!-- Right: issue detail / editor -->
      <section class="issue-detail">
        <div id="detail"></div>
      </section>
    </main>

    <!-- Footer: labels bar -->
    <footer class="footer">
      <div class="labels-bar">
        <span class="labels-title">Labels:</span>
        <div id="labels"></div>
        <button id="btn-new-label" class="btn-small">+</button>
      </div>
    </footer>
  </div>

  <!-- Modal for create/edit -->
  <dialog id="modal">
    <form id="issue-form">
      <div class="modal-header">
        <h3 id="modal-title">New Issue</h3>
        <button type="button" class="btn-close" id="btn-modal-close">×</button>
      </div>
      <div class="modal-body">
        <input type="text" id="form-title" placeholder="Title" required>
        <textarea id="form-body" placeholder="Description (optional)" rows="6"></textarea>
        <div class="form-row">
          <select id="form-type">
            <option value="task">Task</option>
            <option value="bug">Bug</option>
            <option value="epic">Epic</option>
            <option value="request">Request</option>
          </select>
          <div id="form-labels" class="label-picker"></div>
        </div>
      </div>
      <div class="modal-footer">
        <button type="button" id="btn-cancel">Cancel</button>
        <button type="submit" class="btn-primary" id="btn-save">Save</button>
      </div>
    </form>
  </dialog>

  <script src="main.js"></script>
</body>
</html>
```

---

## Phase 4: Frontend - Compact CSS

### 4.1 Core Styles

`gui/src/styles.css`:

```css
* { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg: #f5f5f5;
  --bg-card: #fff;
  --border: #ddd;
  --text: #333;
  --text-muted: #666;
  --primary: #2563eb;
  --danger: #dc2626;
  --success: #16a34a;
  /* Issue type colors */
  --epic: #a855f7;
  --task: #3b82f6;
  --bug: #ef4444;
  --request: #06b6d4;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  font-size: 13px;
  line-height: 1.4;
  background: var(--bg);
  color: var(--text);
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* Header */
.header {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border);
}

.dir-selector {
  display: flex;
  gap: 0.25rem;
}

.dir-selector input {
  width: 200px;
  padding: 0.3rem 0.5rem;
  font-size: 12px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg);
}

.filters {
  display: flex;
  gap: 0.25rem;
  margin-left: auto;
}

.filters select,
.filters input {
  padding: 0.3rem 0.5rem;
  font-size: 12px;
  border: 1px solid var(--border);
  border-radius: 3px;
}

/* Main split view */
.main {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.issue-list {
  width: 280px;
  border-right: 1px solid var(--border);
  overflow-y: auto;
  background: var(--bg-card);
}

.issue-detail {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem;
}

/* Issue list items */
.issue-item {
  padding: 0.5rem;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}

.issue-item:hover {
  background: var(--bg);
}

.issue-item.selected {
  background: #e0e7ff;
}

.issue-item .issue-id {
  font-size: 11px;
  color: var(--text-muted);
}

.issue-item .issue-title {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.issue-item .issue-meta {
  display: flex;
  gap: 0.5rem;
  font-size: 11px;
  margin-top: 0.25rem;
}

/* Type badges */
.type-badge {
  display: inline-block;
  padding: 0.1rem 0.35rem;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
}

.type-epic { background: var(--epic); color: #fff; }
.type-task { background: var(--task); color: #fff; }
.type-bug { background: var(--bug); color: #fff; }
.type-request { background: var(--request); color: #fff; }

/* State badges */
.state-open { color: var(--success); }
.state-closed { color: var(--danger); }

/* Labels */
.label {
  display: inline-block;
  padding: 0.1rem 0.4rem;
  border-radius: 10px;
  font-size: 10px;
  font-weight: 500;
}

/* Issue detail view */
.detail-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.detail-header h2 {
  font-size: 16px;
  font-weight: 600;
}

.detail-meta {
  display: flex;
  gap: 1rem;
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 0.75rem;
}

.detail-body {
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
}

.detail-labels {
  display: flex;
  gap: 0.25rem;
  flex-wrap: wrap;
  margin: 0.5rem 0;
}

.detail-links {
  margin-top: 0.75rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border);
  font-size: 12px;
}

/* Comments */
.comments {
  margin-top: 1rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.comment {
  padding: 0.5rem;
  margin-bottom: 0.5rem;
  background: var(--bg);
  border-radius: 4px;
  font-size: 12px;
}

.comment-meta {
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 0.25rem;
}

/* Actions bar */
.detail-actions {
  display: flex;
  gap: 0.25rem;
  margin-top: 0.75rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border);
}

/* Footer / labels bar */
.footer {
  padding: 0.35rem 0.5rem;
  background: var(--bg-card);
  border-top: 1px solid var(--border);
}

.labels-bar {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.labels-title {
  font-size: 11px;
  color: var(--text-muted);
}

/* Buttons */
button {
  padding: 0.3rem 0.6rem;
  font-size: 12px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-card);
  cursor: pointer;
}

button:hover {
  background: var(--bg);
}

.btn-primary {
  background: var(--primary);
  border-color: var(--primary);
  color: #fff;
}

.btn-primary:hover {
  background: #1d4ed8;
}

.btn-danger {
  color: var(--danger);
}

.btn-small {
  padding: 0.15rem 0.4rem;
  font-size: 11px;
}

/* Modal */
dialog {
  border: none;
  border-radius: 6px;
  padding: 0;
  box-shadow: 0 4px 20px rgba(0,0,0,0.15);
  max-width: 500px;
  width: 90%;
}

dialog::backdrop {
  background: rgba(0,0,0,0.3);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--border);
}

.modal-header h3 {
  font-size: 14px;
}

.btn-close {
  border: none;
  background: none;
  font-size: 18px;
  cursor: pointer;
  padding: 0;
}

.modal-body {
  padding: 0.75rem;
}

.modal-body input,
.modal-body textarea,
.modal-body select {
  width: 100%;
  padding: 0.4rem 0.5rem;
  font-size: 13px;
  border: 1px solid var(--border);
  border-radius: 3px;
  margin-bottom: 0.5rem;
}

.modal-body textarea {
  resize: vertical;
  font-family: inherit;
}

.form-row {
  display: flex;
  gap: 0.5rem;
}

.form-row select {
  width: auto;
}

.label-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
  flex: 1;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.25rem;
  padding: 0.5rem 0.75rem;
  border-top: 1px solid var(--border);
}

/* Empty states */
.empty {
  padding: 2rem;
  text-align: center;
  color: var(--text-muted);
}
```

---

## Phase 5: Frontend - JavaScript

### 5.1 Core Logic

`gui/src/main.js`:

```javascript
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

// State
let issues = [];
let labels = [];
let selectedIssue = null;
let currentDir = null;

// DOM refs
const $issues = document.getElementById('issues');
const $detail = document.getElementById('detail');
const $labels = document.getElementById('labels');
const $dirPath = document.getElementById('dir-path');
const $filterState = document.getElementById('filter-state');
const $filterType = document.getElementById('filter-type');
const $filterSearch = document.getElementById('filter-search');
const $modal = document.getElementById('modal');

// Initialize
async function init() {
  // Try to get saved directory
  currentDir = localStorage.getItem('skis-dir');
  if (currentDir) {
    $dirPath.value = currentDir;
    await refresh();
  }

  bindEvents();
}

function bindEvents() {
  document.getElementById('btn-browse').onclick = selectDirectory;
  document.getElementById('btn-new').onclick = () => openModal();
  document.getElementById('btn-new-label').onclick = createLabelPrompt;
  document.getElementById('btn-modal-close').onclick = closeModal;
  document.getElementById('btn-cancel').onclick = closeModal;
  document.getElementById('issue-form').onsubmit = saveIssue;

  $filterState.onchange = refresh;
  $filterType.onchange = refresh;
  $filterSearch.oninput = debounce(refresh, 300);
}

async function selectDirectory() {
  const dir = await open({ directory: true });
  if (dir) {
    currentDir = dir;
    $dirPath.value = dir;
    localStorage.setItem('skis-dir', dir);
    await refresh();
  }
}

async function refresh() {
  if (!currentDir) return;

  const state = $filterState.value;
  const type = $filterType.value || null;
  const search = $filterSearch.value || null;

  const res = await invoke('list_issues', {
    state: state === 'all' ? null : state,
    issueType: type,
    labels: [],
    search
  });

  if (res.ok) {
    issues = res.data;
    renderIssueList();
  }

  const labelsRes = await invoke('list_labels');
  if (labelsRes.ok) {
    labels = labelsRes.data;
    renderLabelsBar();
  }
}

function renderIssueList() {
  if (issues.length === 0) {
    $issues.innerHTML = '<div class="empty">No issues found</div>';
    return;
  }

  $issues.innerHTML = issues.map(issue => `
    <div class="issue-item ${selectedIssue?.id === issue.id ? 'selected' : ''}"
         data-id="${issue.id}">
      <div class="issue-id">#${issue.id}</div>
      <div class="issue-title">${esc(issue.title)}</div>
      <div class="issue-meta">
        <span class="type-badge type-${issue.type}">${issue.type}</span>
        <span class="state-${issue.state}">${issue.state}</span>
      </div>
    </div>
  `).join('');

  // Bind click events
  $issues.querySelectorAll('.issue-item').forEach(el => {
    el.onclick = () => selectIssue(parseInt(el.dataset.id));
  });
}

async function selectIssue(id) {
  const res = await invoke('get_issue', { id });
  if (res.ok) {
    selectedIssue = res.data;
    renderIssueDetail();
    renderIssueList(); // Update selection highlight
  }
}

function renderIssueDetail() {
  if (!selectedIssue) {
    $detail.innerHTML = '<div class="empty">Select an issue</div>';
    return;
  }

  const i = selectedIssue;
  const labelsHtml = i.labels.map(l =>
    `<span class="label" style="background: #${l.color}; color: ${textColor(l.color)}">${esc(l.name)}</span>`
  ).join('');

  const linksHtml = i.linked_issues.length > 0
    ? `<div class="detail-links">Linked: ${i.linked_issues.map(l =>
        `<a href="#" data-link="${l.id}">#${l.id} ${esc(l.title)}</a>`
      ).join(', ')}</div>`
    : '';

  $detail.innerHTML = `
    <div class="detail-header">
      <span class="type-badge type-${i.issue.type}">${i.issue.type}</span>
      <h2>#${i.issue.id} ${esc(i.issue.title)}</h2>
    </div>
    <div class="detail-meta">
      <span class="state-${i.issue.state}">${i.issue.state}</span>
      <span>Created ${formatTime(i.issue.created_at)}</span>
      <span>Updated ${formatTime(i.issue.updated_at)}</span>
    </div>
    <div class="detail-labels">${labelsHtml}</div>
    <div class="detail-body">${esc(i.issue.body || '')}</div>
    ${linksHtml}
    <div class="detail-actions">
      <button onclick="editIssue()">Edit</button>
      ${i.issue.state === 'open'
        ? `<button onclick="closeIssue()">Close</button>`
        : `<button onclick="reopenIssue()">Reopen</button>`}
      <button class="btn-danger" onclick="deleteIssue()">Delete</button>
    </div>
    <div class="comments" id="comments"></div>
  `;

  loadComments();
}

async function loadComments() {
  if (!selectedIssue) return;
  const res = await invoke('get_comments', { issueId: selectedIssue.issue.id });
  if (res.ok && res.data.length > 0) {
    document.getElementById('comments').innerHTML = `
      <h4>Comments</h4>
      ${res.data.map(c => `
        <div class="comment">
          <div class="comment-meta">${formatTime(c.created_at)}</div>
          <div>${esc(c.body)}</div>
        </div>
      `).join('')}
    `;
  }
}

function renderLabelsBar() {
  $labels.innerHTML = labels.map(l => `
    <span class="label" style="background: #${l.color}; color: ${textColor(l.color)}">${esc(l.name)}</span>
  `).join('');
}

// Modal functions
function openModal(issue = null) {
  document.getElementById('modal-title').textContent = issue ? 'Edit Issue' : 'New Issue';
  document.getElementById('form-title').value = issue?.title || '';
  document.getElementById('form-body').value = issue?.body || '';
  document.getElementById('form-type').value = issue?.type || 'task';
  $modal.dataset.issueId = issue?.id || '';
  $modal.showModal();
}

function closeModal() {
  $modal.close();
}

async function saveIssue(e) {
  e.preventDefault();
  const id = $modal.dataset.issueId;
  const title = document.getElementById('form-title').value;
  const body = document.getElementById('form-body').value || null;
  const type = document.getElementById('form-type').value;

  let res;
  if (id) {
    res = await invoke('update_issue', { id: parseInt(id), title, body, issueType: type });
  } else {
    res = await invoke('create_issue', { title, body, issueType: type, labels: [] });
  }

  if (res.ok) {
    closeModal();
    await refresh();
    if (res.data) selectIssue(res.data.id);
  } else {
    alert(res.error);
  }
}

// Issue actions
window.editIssue = () => openModal(selectedIssue?.issue);

window.closeIssue = async () => {
  if (!selectedIssue) return;
  const res = await invoke('close_issue', {
    id: selectedIssue.issue.id,
    reason: 'completed',
    comment: null
  });
  if (res.ok) {
    await refresh();
    selectIssue(selectedIssue.issue.id);
  }
};

window.reopenIssue = async () => {
  if (!selectedIssue) return;
  const res = await invoke('reopen_issue', { id: selectedIssue.issue.id });
  if (res.ok) {
    await refresh();
    selectIssue(selectedIssue.issue.id);
  }
};

window.deleteIssue = async () => {
  if (!selectedIssue || !confirm('Delete this issue?')) return;
  const res = await invoke('delete_issue', { id: selectedIssue.issue.id });
  if (res.ok) {
    selectedIssue = null;
    await refresh();
    $detail.innerHTML = '<div class="empty">Select an issue</div>';
  }
};

async function createLabelPrompt() {
  const name = prompt('Label name:');
  if (!name) return;
  const res = await invoke('create_label', { name, description: null, color: null });
  if (res.ok) {
    await refresh();
  } else {
    alert(res.error);
  }
}

// Utilities
function esc(s) {
  const div = document.createElement('div');
  div.textContent = s;
  return div.innerHTML;
}

function formatTime(iso) {
  const d = new Date(iso);
  const now = new Date();
  const diff = (now - d) / 1000;
  if (diff < 60) return 'just now';
  if (diff < 3600) return `${Math.floor(diff/60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff/3600)}h ago`;
  if (diff < 2592000) return `${Math.floor(diff/86400)}d ago`;
  return d.toLocaleDateString();
}

function textColor(hex) {
  // Return black or white based on luminance
  const r = parseInt(hex.slice(0,2), 16);
  const g = parseInt(hex.slice(2,4), 16);
  const b = parseInt(hex.slice(4,6), 16);
  const lum = (0.299*r + 0.587*g + 0.114*b) / 255;
  return lum > 0.5 ? '#000' : '#fff';
}

function debounce(fn, ms) {
  let t;
  return (...args) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...args), ms);
  };
}

// Start
init();
```

---

## Phase 6: Build & Distribution

### 6.1 Development

```bash
cd gui/src-tauri
cargo tauri dev
```

### 6.2 Build Release

```bash
cd gui/src-tauri
cargo tauri build
```

Produces:
- macOS: `target/release/bundle/macos/SKIS.app`
- Windows: `target/release/bundle/msi/SKIS_x.x.x_x64.msi`
- Linux: `target/release/bundle/deb/skis_x.x.x_amd64.deb`

### 6.3 Icons

Generate icons at standard sizes:
- 32x32.png
- 128x128.png
- 128x128@2x.png (256x256)
- icon.icns (macOS)
- icon.ico (Windows)

Use a simple "S" or document icon in the issue type colors.

---

## Implementation Order

1. **Phase 1**: Project setup (workspace, Cargo.toml, tauri.conf.json)
2. **Phase 2**: Backend commands (start with list_issues, get_issue, create_issue)
3. **Phase 3**: HTML structure (static layout)
4. **Phase 4**: CSS styling (get the look right)
5. **Phase 5**: JavaScript (wire up IPC, render logic)
6. **Phase 6**: Polish (icons, build, distribution)

Each phase is independently testable. Start with read-only operations, then add mutations.

---

## Future Enhancements

- Keyboard shortcuts (j/k navigation, n for new, e for edit)
- Drag-and-drop label assignment
- Markdown preview for body
- Comment inline editing
- Dark mode
- Export/import (JSON, CSV)
- Multiple repository tabs
