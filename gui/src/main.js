// SKIS GUI - Main JavaScript
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

// ============ State ============

let currentIssue = null;
let issues = [];
let labels = [];
let sortOrder = 'desc';
let editingIssueId = null;
let homeDir = '';

// ============ DOM Elements ============

// Directory
const directoryPath = document.getElementById('directory-path');
const btnBrowse = document.getElementById('btn-browse');
const btnInit = document.getElementById('btn-init');

// Filters
const searchInput = document.getElementById('search-input');
const filterState = document.getElementById('filter-state');
const filterType = document.getElementById('filter-type');
const filterLabel = document.getElementById('filter-label');

// Issue list
const issueList = document.getElementById('issue-list');
const issueCount = document.getElementById('issue-count');
const emptyState = document.getElementById('empty-state');
const sortBy = document.getElementById('sort-by');
const btnSortOrder = document.getElementById('btn-sort-order');

// Issue detail
const detailEmpty = document.getElementById('detail-empty');
const detailContent = document.getElementById('detail-content');
const detailId = document.getElementById('detail-id');
const detailTitle = document.getElementById('detail-title');
const detailType = document.getElementById('detail-type');
const detailState = document.getElementById('detail-state');
const detailTimestamps = document.getElementById('detail-timestamps');
const detailLabels = document.getElementById('detail-labels');
const detailBody = document.getElementById('detail-body');
const linkedIssues = document.getElementById('linked-issues');
const linkIssueId = document.getElementById('link-issue-id');
const btnLink = document.getElementById('btn-link');
const commentsList = document.getElementById('comments-list');
const commentInput = document.getElementById('comment-input');
const btnAddComment = document.getElementById('btn-add-comment');

// Detail actions
const btnEdit = document.getElementById('btn-edit');
const btnClose = document.getElementById('btn-close');
const btnReopen = document.getElementById('btn-reopen');
const btnDelete = document.getElementById('btn-delete');
const btnNewIssue = document.getElementById('btn-new-issue');

// Labels bar
const labelsList = document.getElementById('labels-list');
const btnManageLabels = document.getElementById('btn-manage-labels');

// Issue modal
const modalOverlay = document.getElementById('modal-overlay');
const modalTitle = document.getElementById('modal-title');
const issueForm = document.getElementById('issue-form');
const inputTitle = document.getElementById('input-title');
const inputType = document.getElementById('input-type');
const inputLabels = document.getElementById('input-labels');
const inputBody = document.getElementById('input-body');
const btnModalClose = document.getElementById('btn-modal-close');
const btnModalCancel = document.getElementById('btn-modal-cancel');
const btnModalSave = document.getElementById('btn-modal-save');

// Close modal
const closeModalOverlay = document.getElementById('close-modal-overlay');
const closeReason = document.getElementById('close-reason');
const closeComment = document.getElementById('close-comment');
const btnConfirmClose = document.getElementById('btn-confirm-close');

// Labels modal
const labelsModalOverlay = document.getElementById('labels-modal-overlay');
const existingLabels = document.getElementById('existing-labels');
const newLabelName = document.getElementById('new-label-name');
const newLabelColor = document.getElementById('new-label-color');
const newLabelDesc = document.getElementById('new-label-desc');
const btnCreateLabel = document.getElementById('btn-create-label');

// ============ Initialization ============

async function init() {
  try {
    const result = await invoke('get_home_dir');
    if (result.ok) {
      homeDir = result.data;
    }
  } catch (e) {
    console.error('Could not get home dir:', e);
  }

  // Check for saved directory
  const savedDir = localStorage.getItem('skis_directory');
  if (savedDir) {
    await selectDirectory(savedDir);
  }

  setupEventListeners();
}

function setupEventListeners() {
  // Directory
  btnBrowse.addEventListener('click', browseDirectory);
  btnInit.addEventListener('click', initRepository);

  // Filters
  searchInput.addEventListener('input', debounce(loadIssues, 300));
  filterState.addEventListener('change', loadIssues);
  filterType.addEventListener('change', loadIssues);
  filterLabel.addEventListener('change', loadIssues);
  sortBy.addEventListener('change', loadIssues);
  btnSortOrder.addEventListener('click', toggleSortOrder);

  // New issue
  btnNewIssue.addEventListener('click', () => openIssueModal());

  // Detail actions
  btnEdit.addEventListener('click', () => openIssueModal(currentIssue));
  btnClose.addEventListener('click', showCloseModal);
  btnReopen.addEventListener('click', reopenIssue);
  btnDelete.addEventListener('click', deleteIssue);
  btnLink.addEventListener('click', linkIssue);
  btnAddComment.addEventListener('click', addComment);

  // Issue modal
  btnModalClose.addEventListener('click', closeIssueModal);
  btnModalCancel.addEventListener('click', closeIssueModal);
  issueForm.addEventListener('submit', saveIssue);
  modalOverlay.addEventListener('click', e => {
    if (e.target === modalOverlay) closeIssueModal();
  });

  // Close modal
  closeModalOverlay.querySelectorAll('.btn-close-modal').forEach(btn => {
    btn.addEventListener('click', () => closeModalOverlay.style.display = 'none');
  });
  btnConfirmClose.addEventListener('click', closeIssue);
  closeModalOverlay.addEventListener('click', e => {
    if (e.target === closeModalOverlay) closeModalOverlay.style.display = 'none';
  });

  // Labels modal
  btnManageLabels.addEventListener('click', openLabelsModal);
  labelsModalOverlay.querySelectorAll('.btn-close-labels-modal').forEach(btn => {
    btn.addEventListener('click', closeLabelsModal);
  });
  btnCreateLabel.addEventListener('click', createLabel);
  labelsModalOverlay.addEventListener('click', e => {
    if (e.target === labelsModalOverlay) closeLabelsModal();
  });

  // Keyboard shortcuts
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape') {
      if (modalOverlay.style.display !== 'none') closeIssueModal();
      if (closeModalOverlay.style.display !== 'none') closeModalOverlay.style.display = 'none';
      if (labelsModalOverlay.style.display !== 'none') closeLabelsModal();
    }
    // Ctrl+N or Cmd+N for new issue
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      openIssueModal();
    }
  });
}

// ============ Directory Management ============

async function browseDirectory() {
  try {
    const selected = await open({
      directory: true,
      multiple: false
    });
    if (selected) {
      await selectDirectory(selected);
    }
  } catch (err) {
    console.error('Error opening directory dialog:', err);
  }
}

async function selectDirectory(path) {
  try {
    const result = await invoke('select_directory', { path });
    if (result.ok) {
      directoryPath.value = shortenPath(path);
      localStorage.setItem('skis_directory', path);

      if (result.data.initialized) {
        btnInit.style.display = 'none';
        await loadIssues();
        await loadLabels();
      } else {
        btnInit.style.display = 'inline-block';
        showEmptyState('Directory not initialized. Click ⚡ to initialize SKIS.');
      }
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function initRepository() {
  try {
    const result = await invoke('init_repository');
    if (result.ok) {
      btnInit.style.display = 'none';
      await loadIssues();
      await loadLabels();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

// ============ Issue List ============

async function loadIssues() {
  const filter = {
    state: filterState.value || null,
    issue_type: filterType.value || null,
    labels: filterLabel.value ? [filterLabel.value] : null,
    sort_by: sortBy.value,
    sort_order: sortOrder,
    search: searchInput.value || null
  };

  try {
    const result = await invoke('list_issues', { filter });
    if (result.ok) {
      issues = result.data;
      renderIssueList();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

function renderIssueList() {
  if (issues.length === 0) {
    emptyState.style.display = 'flex';
    emptyState.innerHTML = '<p>No issues found</p>';
    issueList.innerHTML = '';
    issueCount.textContent = '0 issues';
    return;
  }

  emptyState.style.display = 'none';
  issueCount.textContent = `${issues.length} issue${issues.length !== 1 ? 's' : ''}`;

  issueList.innerHTML = issues.map(issue => `
    <div class="issue-item ${currentIssue && currentIssue.id === issue.id ? 'selected' : ''}"
         data-id="${issue.id}">
      <div class="issue-item-header">
        <span class="issue-item-id">#${issue.id}</span>
        <span class="badge badge-type ${issue.issue_type}">${issue.issue_type}</span>
        <span class="badge badge-state ${issue.state}">${issue.state}</span>
      </div>
      <div class="issue-item-title">${escapeHtml(issue.title)}</div>
      ${issue.labels.length > 0 ? `
        <div class="issue-item-labels">
          ${issue.labels.map(l => renderLabelPill(l, true)).join('')}
        </div>
      ` : ''}
    </div>
  `).join('');

  // Add click handlers
  issueList.querySelectorAll('.issue-item').forEach(el => {
    el.addEventListener('click', () => {
      const id = parseInt(el.dataset.id);
      selectIssue(id);
    });
  });
}

async function selectIssue(id) {
  const issue = issues.find(i => i.id === id);

  if (issue) {
    currentIssue = issue;
    renderIssueList(); // Update selection
  }

  // Always load the detail - even if not in current filtered list
  await loadIssueDetail(id);
}

// ============ Issue Detail ============

async function loadIssueDetail(id) {
  try {
    const result = await invoke('get_issue', { id });
    if (result.ok) {
      currentIssue = result.data;
      renderIssueDetail();
      await loadComments(id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

function renderIssueDetail() {
  if (!currentIssue) {
    detailEmpty.style.display = 'flex';
    detailContent.style.display = 'none';
    return;
  }

  detailEmpty.style.display = 'none';
  detailContent.style.display = 'block';

  detailId.textContent = `#${currentIssue.id}`;
  detailTitle.textContent = currentIssue.title;
  detailType.textContent = currentIssue.issue_type;
  detailType.className = `badge badge-type ${currentIssue.issue_type}`;
  detailState.textContent = currentIssue.state;
  detailState.className = `badge badge-state ${currentIssue.state}`;
  detailTimestamps.textContent = formatTimestamps(currentIssue);
  detailBody.textContent = currentIssue.body || '';

  // Labels
  detailLabels.innerHTML = currentIssue.labels.map(l => renderLabelPill(l)).join('');

  // Linked issues
  if (currentIssue.linked_issues.length > 0) {
    linkedIssues.innerHTML = currentIssue.linked_issues.map(li => `
      <span class="linked-issue" data-id="${li.id}">
        #${li.id} ${escapeHtml(li.title.substring(0, 30))}${li.title.length > 30 ? '...' : ''}
        <button class="btn-icon unlink-btn" data-id="${li.id}" title="Unlink">×</button>
      </span>
    `).join('');

    // Add click handlers
    linkedIssues.querySelectorAll('.linked-issue').forEach(el => {
      el.addEventListener('click', e => {
        if (!e.target.classList.contains('unlink-btn')) {
          selectIssue(parseInt(el.dataset.id));
        }
      });
    });

    linkedIssues.querySelectorAll('.unlink-btn').forEach(btn => {
      btn.addEventListener('click', e => {
        e.stopPropagation();
        unlinkIssue(parseInt(btn.dataset.id));
      });
    });
  } else {
    linkedIssues.innerHTML = '<span style="color: var(--color-text-muted); font-size: 0.8rem;">No linked issues</span>';
  }

  // Show/hide close/reopen buttons
  if (currentIssue.state === 'open') {
    btnClose.style.display = 'inline-block';
    btnReopen.style.display = 'none';
  } else {
    btnClose.style.display = 'none';
    btnReopen.style.display = 'inline-block';
  }
}

async function loadComments(issueId) {
  try {
    const result = await invoke('get_comments', { issueId });
    if (result.ok) {
      renderComments(result.data);
    }
  } catch (err) {
    console.error('Error loading comments:', err);
  }
}

function renderComments(comments) {
  if (comments.length === 0) {
    commentsList.innerHTML = '<p style="color: var(--color-text-muted); font-size: 0.8rem;">No comments yet</p>';
    return;
  }

  commentsList.innerHTML = comments.map(c => `
    <div class="comment-item">
      <div class="comment-meta">${formatDateTime(c.created_at)}</div>
      <div class="comment-body">${escapeHtml(c.body)}</div>
    </div>
  `).join('');
}

// ============ Issue Actions ============

function openIssueModal(issue = null) {
  editingIssueId = issue ? issue.id : null;
  modalTitle.textContent = issue ? 'Edit Issue' : 'New Issue';
  btnModalSave.textContent = issue ? 'Save Changes' : 'Create Issue';

  inputTitle.value = issue ? issue.title : '';
  inputType.value = issue ? issue.issue_type : 'task';
  inputBody.value = issue ? (issue.body || '') : '';

  // Populate labels
  inputLabels.innerHTML = labels.map(l => `
    <option value="${escapeHtml(l.name)}" ${issue && issue.labels.some(il => il.name === l.name) ? 'selected' : ''}>
      ${escapeHtml(l.name)}
    </option>
  `).join('');

  modalOverlay.style.display = 'flex';
  inputTitle.focus();
}

function closeIssueModal() {
  modalOverlay.style.display = 'none';
  editingIssueId = null;
  issueForm.reset();
}

async function saveIssue(e) {
  e.preventDefault();

  const selectedLabels = Array.from(inputLabels.selectedOptions).map(o => o.value);

  if (editingIssueId) {
    // Update existing issue
    try {
      const result = await invoke('update_issue', {
        id: editingIssueId,
        params: {
          title: inputTitle.value,
          body: inputBody.value || null,
          issue_type: inputType.value
        }
      });
      if (result.ok) {
        closeIssueModal();
        await loadIssues();
        await loadIssueDetail(editingIssueId);
      } else {
        showError(result.error);
      }
    } catch (err) {
      showError(err);
    }
  } else {
    // Create new issue
    try {
      const result = await invoke('create_issue', {
        params: {
          title: inputTitle.value,
          body: inputBody.value || null,
          issue_type: inputType.value,
          labels: selectedLabels
        }
      });
      if (result.ok) {
        closeIssueModal();
        await loadIssues();
        selectIssue(result.data.id);
      } else {
        showError(result.error);
      }
    } catch (err) {
      showError(err);
    }
  }
}

function showCloseModal() {
  closeReason.value = 'completed';
  closeComment.value = '';
  closeModalOverlay.style.display = 'flex';
}

async function closeIssue() {
  if (!currentIssue) return;

  try {
    const result = await invoke('close_issue', {
      id: currentIssue.id,
      reason: closeReason.value,
      comment: closeComment.value || null
    });
    if (result.ok) {
      closeModalOverlay.style.display = 'none';
      await loadIssues();
      await loadIssueDetail(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function reopenIssue() {
  if (!currentIssue) return;

  try {
    const result = await invoke('reopen_issue', { id: currentIssue.id });
    if (result.ok) {
      await loadIssues();
      await loadIssueDetail(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function deleteIssue() {
  if (!currentIssue) return;
  if (!confirm(`Delete issue #${currentIssue.id}? This can be undone.`)) return;

  try {
    const result = await invoke('delete_issue', { id: currentIssue.id });
    if (result.ok) {
      currentIssue = null;
      renderIssueDetail();
      await loadIssues();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function linkIssue() {
  if (!currentIssue) return;
  const targetId = parseInt(linkIssueId.value);
  if (!targetId || targetId === currentIssue.id) return;

  try {
    const result = await invoke('link_issues', {
      issueA: currentIssue.id,
      issueB: targetId
    });
    if (result.ok) {
      linkIssueId.value = '';
      await loadIssueDetail(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function unlinkIssue(targetId) {
  if (!currentIssue) return;

  try {
    const result = await invoke('unlink_issues', {
      issueA: currentIssue.id,
      issueB: targetId
    });
    if (result.ok) {
      await loadIssueDetail(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function addComment() {
  if (!currentIssue || !commentInput.value.trim()) return;

  try {
    const result = await invoke('add_comment', {
      issueId: currentIssue.id,
      body: commentInput.value.trim()
    });
    if (result.ok) {
      commentInput.value = '';
      await loadComments(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

// ============ Labels ============

async function loadLabels() {
  try {
    const result = await invoke('list_labels');
    if (result.ok) {
      labels = result.data;
      renderLabelsBar();
      updateLabelFilter();
    }
  } catch (err) {
    console.error('Error loading labels:', err);
  }
}

function renderLabelsBar() {
  labelsList.innerHTML = labels.map(l => renderLabelPill(l)).join('');
}

function updateLabelFilter() {
  const currentValue = filterLabel.value;
  filterLabel.innerHTML = '<option value="">All labels</option>' +
    labels.map(l => `<option value="${escapeHtml(l.name)}" ${currentValue === l.name ? 'selected' : ''}>${escapeHtml(l.name)}</option>`).join('');
}

function openLabelsModal() {
  renderExistingLabels();
  newLabelName.value = '';
  newLabelColor.value = '';
  newLabelDesc.value = '';
  labelsModalOverlay.style.display = 'flex';
}

function closeLabelsModal() {
  labelsModalOverlay.style.display = 'none';
}

function renderExistingLabels() {
  existingLabels.innerHTML = labels.map(l => `
    <div class="label-row">
      ${renderLabelPill(l)}
      <button class="btn-icon delete-label" data-name="${escapeHtml(l.name)}" title="Delete label">🗑</button>
    </div>
  `).join('');

  existingLabels.querySelectorAll('.delete-label').forEach(btn => {
    btn.addEventListener('click', () => deleteLabel(btn.dataset.name));
  });
}

async function createLabel() {
  const name = newLabelName.value.trim();
  if (!name) return;

  const color = newLabelColor.value.trim() || null;
  const description = newLabelDesc.value.trim() || null;

  try {
    const result = await invoke('create_label', { name, description, color });
    if (result.ok) {
      newLabelName.value = '';
      newLabelColor.value = '';
      newLabelDesc.value = '';
      await loadLabels();
      renderExistingLabels();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function deleteLabel(name) {
  if (!confirm(`Delete label "${name}"?`)) return;

  try {
    const result = await invoke('delete_label', { name });
    if (result.ok) {
      await loadLabels();
      renderExistingLabels();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

// ============ Sort ============

function toggleSortOrder() {
  sortOrder = sortOrder === 'desc' ? 'asc' : 'desc';
  btnSortOrder.textContent = sortOrder === 'desc' ? '↓' : '↑';
  loadIssues();
}

// ============ Utilities ============

function renderLabelPill(label, small = false) {
  const color = label.color || '888888';
  const textColor = getContrastColor(color);
  return `<span class="label-pill ${small ? 'label-pill-small' : ''}" style="background-color: #${color}; color: ${textColor};">${escapeHtml(label.name)}</span>`;
}

function getContrastColor(hexColor) {
  const r = parseInt(hexColor.substr(0, 2), 16);
  const g = parseInt(hexColor.substr(2, 2), 16);
  const b = parseInt(hexColor.substr(4, 2), 16);
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.5 ? '#000' : '#fff';
}

function formatTimestamps(issue) {
  const created = formatRelativeTime(issue.created_at);
  const updated = formatRelativeTime(issue.updated_at);
  let text = `Created ${created}`;
  if (issue.updated_at !== issue.created_at) {
    text += ` · Updated ${updated}`;
  }
  if (issue.closed_at) {
    text += ` · Closed ${formatRelativeTime(issue.closed_at)}`;
  }
  return text;
}

function formatRelativeTime(isoString) {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now - date;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return 'just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;

  return date.toLocaleDateString();
}

function formatDateTime(isoString) {
  const date = new Date(isoString);
  return date.toLocaleString();
}

function shortenPath(path) {
  if (homeDir && path.startsWith(homeDir)) {
    return '~' + path.slice(homeDir.length);
  }
  return path;
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function showError(error) {
  alert(`Error: ${error}`);
}

function showEmptyState(message) {
  emptyState.style.display = 'flex';
  emptyState.innerHTML = `<p>${message}</p>`;
  issueList.innerHTML = '';
  issueCount.textContent = '0 issues';
}

function debounce(fn, delay) {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), delay);
  };
}

// ============ Start ============

init();
