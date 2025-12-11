// SKIS GUI - Main JavaScript
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

// ============ Constants ============

const MAX_RECENT_DIRS = 10;
const STORAGE_RECENT_DIRS = 'skis_recent_directories';
const STORAGE_WINDOW_STATE = 'skis_main_window_state';
const STORAGE_APP_STATE = 'skis_app_state';
const PAGE_SIZE = 50;

// ============ State ============

let currentIssue = null;
let issues = [];
let labels = [];
let sortOrder = 'desc';
let editingIssueId = null;
let homeDir = '';
let recentDirectories = [];
let isLoadingMore = false;
let hasMoreIssues = true;
let sidebarCollapsed = false;
let sidebarWidth = 320;

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
const issueListPanel = document.getElementById('issue-list-panel');
const issueList = document.getElementById('issue-list');
const issueCount = document.getElementById('issue-count');
const emptyState = document.getElementById('empty-state');
const sortBy = document.getElementById('sort-by');
const btnSortOrder = document.getElementById('btn-sort-order');
const paneDivider = document.getElementById('pane-divider');

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

// Close issue modal
const closeModalOverlay = document.getElementById('close-modal-overlay');
const closeReason = document.getElementById('close-reason');
const closeComment = document.getElementById('close-comment');
const btnConfirmClose = document.getElementById('btn-confirm-close');


// ============ Initialization ============

// Configure marked for GitHub-flavored markdown with syntax highlighting
marked.setOptions({
  gfm: true,
  breaks: true,
  highlight: function(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(code, { language: lang }).value;
      } catch (e) {}
    }
    return hljs.highlightAuto(code).value;
  }
});

async function init() {
  // Restore window size/position
  await restoreWindowState();

  // Restore sidebar state
  restoreSidebarState();

  // Save window state on resize/move
  const win = getCurrentWindow();
  win.onResized(debounce(saveWindowState, 500));
  win.onMoved(debounce(saveWindowState, 500));

  try {
    const result = await invoke('get_home_dir');
    if (result.ok) {
      homeDir = result.data;
    }
  } catch (e) {
    console.error('Could not get home dir:', e);
  }

  // Load recent directories
  loadRecentDirectories();

  // Update menu with recent directories
  await updateRecentMenu();

  // Listen for menu events
  await listen('menu-open-recent', async (event) => {
    const path = event.payload;
    if (path) {
      await selectDirectory(path);
    }
  });

  await listen('menu-open', async () => {
    await browseDirectory();
  });

  await listen('menu-reload', async () => {
    await reload();
  });

  await listen('menu-toggle-sidebar', () => {
    toggleSidebar();
  });

  // Listen for issue saved from edit window
  await listen('issue-saved', async (event) => {
    const { id } = event.payload;
    await loadIssues();
    await loadLabels();
    if (id) {
      await selectIssue(id);
    }
  });

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

  // Filters - save state after loading
  const loadAndSave = async () => {
    await loadIssues();
    saveAppState();
  };
  searchInput.addEventListener('input', debounce(loadAndSave, 300));
  filterState.addEventListener('change', loadAndSave);
  filterType.addEventListener('change', loadAndSave);
  filterLabel.addEventListener('change', loadAndSave);
  sortBy.addEventListener('change', loadAndSave);
  btnSortOrder.addEventListener('click', toggleSortOrder);

  // New issue
  btnNewIssue.addEventListener('click', () => openEditWindow(null));

  // Detail actions
  btnEdit.addEventListener('click', () => openEditWindow(currentIssue?.id));
  btnClose.addEventListener('click', showCloseModal);
  btnReopen.addEventListener('click', reopenIssue);
  btnDelete.addEventListener('click', deleteIssue);
  btnLink.addEventListener('click', linkIssue);
  btnAddComment.addEventListener('click', addComment);

  // Close modal
  closeModalOverlay.querySelectorAll('.btn-close-modal').forEach(btn => {
    btn.addEventListener('click', () => closeModalOverlay.style.display = 'none');
  });
  btnConfirmClose.addEventListener('click', closeIssue);
  closeModalOverlay.addEventListener('click', e => {
    if (e.target === closeModalOverlay) closeModalOverlay.style.display = 'none';
  });

  // Keyboard shortcuts
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape') {
      if (closeModalOverlay.style.display !== 'none') closeModalOverlay.style.display = 'none';
    }
    // Ctrl+N or Cmd+N for new issue
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      openEditWindow(null);
    }
  });

  // Infinite scroll for issue list
  issueList.addEventListener('scroll', () => {
    if (isLoadingMore || !hasMoreIssues) return;

    const { scrollTop, scrollHeight, clientHeight } = issueList;
    // Load more when within 100px of bottom
    if (scrollTop + clientHeight >= scrollHeight - 100) {
      loadIssues(true);
    }
  });

  // Pane divider drag
  setupPaneDivider();
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

      // Add to recent directories
      addRecentDirectory(path);

      if (result.data.initialized) {
        btnInit.style.display = 'none';

        // Restore app state before loading (filters, sort)
        const savedState = restoreAppState();

        await loadIssues();
        await loadLabels();

        // Restore label filter after labels are loaded
        if (savedState?.filterLabel) {
          filterLabel.value = savedState.filterLabel;
        }

        // Restore selected issue after issues are loaded
        if (savedState?.selectedIssueId) {
          await selectIssue(savedState.selectedIssueId);
        }
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

async function reload() {
  const currentId = currentIssue?.id;
  await loadIssues();
  await loadLabels();
  if (currentId) {
    await loadIssueDetail(currentId);
  }
}

// ============ Sidebar ============

function toggleSidebar() {
  sidebarCollapsed = !sidebarCollapsed;
  issueListPanel.classList.toggle('collapsed', sidebarCollapsed);
  saveSidebarState();
}

function setupPaneDivider() {
  let isDragging = false;
  let startX = 0;
  let startWidth = 0;

  paneDivider.addEventListener('mousedown', (e) => {
    if (sidebarCollapsed) return;
    isDragging = true;
    startX = e.clientX;
    startWidth = issueListPanel.offsetWidth;
    paneDivider.classList.add('dragging');
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    e.preventDefault();
  });

  document.addEventListener('mousemove', (e) => {
    if (!isDragging) return;
    const delta = e.clientX - startX;
    const newWidth = Math.max(200, Math.min(600, startWidth + delta));
    issueListPanel.style.width = newWidth + 'px';
    sidebarWidth = newWidth;
  });

  document.addEventListener('mouseup', () => {
    if (!isDragging) return;
    isDragging = false;
    paneDivider.classList.remove('dragging');
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    saveSidebarState();
  });
}

function saveSidebarState() {
  localStorage.setItem('skis_sidebar', JSON.stringify({
    collapsed: sidebarCollapsed,
    width: sidebarWidth
  }));
}

function restoreSidebarState() {
  try {
    const stored = localStorage.getItem('skis_sidebar');
    if (!stored) return;

    const state = JSON.parse(stored);
    sidebarCollapsed = state.collapsed || false;
    sidebarWidth = state.width || 320;

    issueListPanel.classList.toggle('collapsed', sidebarCollapsed);
    if (!sidebarCollapsed) {
      issueListPanel.style.width = sidebarWidth + 'px';
    }
  } catch (e) {
    console.error('Could not restore sidebar state:', e);
  }
}

// ============ App State ============

function saveAppState() {
  const state = {
    filterState: filterState.value,
    filterType: filterType.value,
    filterLabel: filterLabel.value,
    sortBy: sortBy.value,
    sortOrder: sortOrder,
    search: searchInput.value,
    selectedIssueId: currentIssue?.id || null
  };
  localStorage.setItem(STORAGE_APP_STATE, JSON.stringify(state));
}

function restoreAppState() {
  try {
    const stored = localStorage.getItem(STORAGE_APP_STATE);
    if (!stored) return null;

    const state = JSON.parse(stored);

    // Restore filter values
    if (state.filterState) filterState.value = state.filterState;
    if (state.filterType) filterType.value = state.filterType;
    // filterLabel will be restored after labels are loaded
    if (state.sortBy) sortBy.value = state.sortBy;
    if (state.sortOrder) {
      sortOrder = state.sortOrder;
      btnSortOrder.textContent = sortOrder === 'desc' ? '↓' : '↑';
    }
    if (state.search) searchInput.value = state.search;

    return state;
  } catch (e) {
    console.error('Could not restore app state:', e);
    return null;
  }
}

// ============ Issue List ============

async function loadIssues(append) {
  // Ensure append is strictly boolean (event objects passed from handlers should be ignored)
  append = append === true;

  if (isLoadingMore) return;

  const offset = append ? issues.length : 0;

  if (!append) {
    issues = [];
    hasMoreIssues = true;
  }

  if (!hasMoreIssues) return;

  isLoadingMore = true;

  const filter = {
    state: filterState.value || null,
    issue_type: filterType.value || null,
    labels: filterLabel.value ? [filterLabel.value] : null,
    sort_by: sortBy.value,
    sort_order: sortOrder,
    search: searchInput.value || null,
    limit: PAGE_SIZE,
    offset: offset
  };

  try {
    const result = await invoke('list_issues', { filter });
    if (result.ok) {
      const newIssues = result.data;

      if (append) {
        issues = [...issues, ...newIssues];
      } else {
        issues = newIssues;
      }

      hasMoreIssues = newIssues.length === PAGE_SIZE;
      renderIssueList();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  } finally {
    isLoadingMore = false;
  }
}

function renderIssueList() {
  if (issues.length === 0 && !isLoadingMore) {
    emptyState.style.display = 'flex';
    emptyState.innerHTML = '<p>No issues found</p>';
    issueList.innerHTML = '';
    issueCount.textContent = '0 issues';
    return;
  }

  emptyState.style.display = 'none';
  const countText = hasMoreIssues ? `${issues.length}+ issues` : `${issues.length} issue${issues.length !== 1 ? 's' : ''}`;
  issueCount.textContent = countText;

  issueList.innerHTML = issues.map(issue => `
    <div class="issue-item ${currentIssue && currentIssue.id === issue.id ? 'selected' : ''}"
         data-id="${issue.id}">
      <div class="issue-item-title"><span class="issue-item-id">#${issue.id}</span> ${escapeHtml(issue.title)}</div>
      <div class="issue-item-labels">
        <span class="label-pill type-${issue.type}">${issue.type}</span>
        ${issue.labels.map(l => renderLabelPill(l, true)).join('')}
      </div>
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

  // Save app state when issue selection changes
  saveAppState();
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
  detailType.textContent = currentIssue.type;
  detailType.className = `badge badge-type ${currentIssue.type}`;
  detailState.textContent = currentIssue.state;
  detailState.className = `badge badge-state ${currentIssue.state}`;
  detailTimestamps.textContent = formatTimestamps(currentIssue);

  // Render body as Markdown
  if (currentIssue.body) {
    detailBody.innerHTML = marked.parse(currentIssue.body);
  } else {
    detailBody.innerHTML = '';
  }

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
      <div class="comment-body markdown-body">${marked.parse(c.body)}</div>
    </div>
  `).join('');
}

// ============ Issue Actions ============

async function openEditWindow(issueId) {
  try {
    const result = await invoke('open_edit_window', { issueId });
    if (!result.ok) {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
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
      updateLabelFilter();
    }
  } catch (err) {
    console.error('Error loading labels:', err);
  }
}

function updateLabelFilter() {
  const currentValue = filterLabel.value;
  filterLabel.innerHTML = '<option value="">All labels</option>' +
    labels.map(l => `<option value="${escapeHtml(l.name)}" ${currentValue === l.name ? 'selected' : ''}>${escapeHtml(l.name)}</option>`).join('');
}

// ============ Sort ============

async function toggleSortOrder() {
  sortOrder = sortOrder === 'desc' ? 'asc' : 'desc';
  btnSortOrder.textContent = sortOrder === 'desc' ? '↓' : '↑';
  await loadIssues();
  saveAppState();
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

// ============ Recent Directories ============

function loadRecentDirectories() {
  try {
    const stored = localStorage.getItem(STORAGE_RECENT_DIRS);
    recentDirectories = stored ? JSON.parse(stored) : [];
  } catch (e) {
    recentDirectories = [];
  }
}

function saveRecentDirectories() {
  localStorage.setItem(STORAGE_RECENT_DIRS, JSON.stringify(recentDirectories));
}

function addRecentDirectory(path) {
  // Remove if already exists
  recentDirectories = recentDirectories.filter(p => p !== path);
  // Add to front
  recentDirectories.unshift(path);
  // Limit size
  recentDirectories = recentDirectories.slice(0, MAX_RECENT_DIRS);
  saveRecentDirectories();
  // Update menu
  updateRecentMenu();
}

async function updateRecentMenu() {
  try {
    await invoke('update_recent_menu', { paths: recentDirectories });
  } catch (e) {
    console.error('Could not update recent menu:', e);
  }
}

// ============ Window State ============

async function saveWindowState() {
  try {
    const win = getCurrentWindow();
    const size = await win.innerSize();
    const position = await win.outerPosition();
    const state = {
      width: size.width,
      height: size.height,
      x: position.x,
      y: position.y
    };
    localStorage.setItem(STORAGE_WINDOW_STATE, JSON.stringify(state));
  } catch (e) {
    console.error('Could not save window state:', e);
  }
}

async function restoreWindowState() {
  try {
    const stored = localStorage.getItem(STORAGE_WINDOW_STATE);
    if (!stored) return;

    const state = JSON.parse(stored);
    const win = getCurrentWindow();

    if (state.width && state.height) {
      await win.setSize({ type: 'Physical', width: state.width, height: state.height });
    }
    if (state.x !== undefined && state.y !== undefined) {
      await win.setPosition({ type: 'Physical', x: state.x, y: state.y });
    }
  } catch (e) {
    console.error('Could not restore window state:', e);
  }
}

// ============ Start ============

init();
