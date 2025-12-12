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
const detailFooter = document.getElementById('detail-footer');
const detailId = document.getElementById('detail-id');
const detailTitle = document.getElementById('detail-title');
const detailTitleEdit = document.getElementById('detail-title-edit');
const titleEditInput = document.getElementById('title-edit-input');
const btnEditTitle = document.getElementById('btn-edit-title');
const btnTitleCancel = document.getElementById('btn-title-cancel');
const btnTitleSave = document.getElementById('btn-title-save');
const detailType = document.getElementById('detail-type');
const detailState = document.getElementById('detail-state');
const detailTimestamps = document.getElementById('detail-timestamps');
const detailLabels = document.getElementById('detail-labels');
const labelsEditForm = document.getElementById('labels-edit-form');
const labelsMenuInline = document.getElementById('labels-menu-inline');
const btnEditLabels = document.getElementById('btn-edit-labels');
const btnLabelsDone = document.getElementById('btn-labels-done');
const detailBody = document.getElementById('detail-body');
const detailBodyEdit = document.getElementById('detail-body-edit');
const bodyEditInput = document.getElementById('body-edit-input');
const bodyEditPreview = document.getElementById('body-edit-preview');
const btnEditBody = document.getElementById('btn-edit-body');
const btnBodyCancel = document.getElementById('btn-body-cancel');
const btnBodySave = document.getElementById('btn-body-save');
const bodyEditTabEdit = document.getElementById('body-edit-tab-edit');
const bodyEditTabPreview = document.getElementById('body-edit-tab-preview');
const linkedIssues = document.getElementById('linked-issues');
const linkForm = document.getElementById('link-form');
const linkIssueId = document.getElementById('link-issue-id');
const btnNewLink = document.getElementById('btn-new-link');
const btnLink = document.getElementById('btn-link');
const btnCancelLink = document.getElementById('btn-cancel-link');
const commentsList = document.getElementById('comments-list');
const commentForm = document.getElementById('comment-form');
const commentInput = document.getElementById('comment-input');
const btnNewComment = document.getElementById('btn-new-comment');
const btnCancelComment = document.getElementById('btn-cancel-comment');
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
  // Get window reference once
  const win = getCurrentWindow();
  const isMainWindow = win.label === 'main';

  // Restore window size/position
  await restoreWindowState();

  // Restore sidebar state
  restoreSidebarState();

  // Save window state on resize/move
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

  await listen('menu-new-database', async () => {
    await createNewDatabase();
  });

  await listen('menu-export-json', async () => {
    await exportToJson();
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

  // Only restore directory for main window - new windows start blank
  if (isMainWindow) {
    const savedDir = localStorage.getItem('skis_directory');
    if (savedDir) {
      await selectDirectory(savedDir, true);
    }
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
  btnNewLink.addEventListener('click', toggleLinkForm);
  btnLink.addEventListener('click', linkIssue);
  btnCancelLink.addEventListener('click', hideLinkForm);
  btnNewComment.addEventListener('click', toggleCommentForm);
  btnCancelComment.addEventListener('click', hideCommentForm);
  btnAddComment.addEventListener('click', addComment);

  // Labels inline editing
  btnEditLabels.addEventListener('click', toggleEditLabels);
  btnLabelsDone.addEventListener('click', finishEditLabels);

  // Title inline editing
  btnEditTitle.addEventListener('click', toggleEditTitle);
  btnTitleCancel.addEventListener('click', cancelEditTitle);
  btnTitleSave.addEventListener('click', saveEditTitle);
  titleEditInput.addEventListener('keydown', e => {
    if (e.key === 'Enter') {
      e.preventDefault();
      saveEditTitle();
    } else if (e.key === 'Escape') {
      cancelEditTitle();
    }
  });

  // Body inline editing
  btnEditBody.addEventListener('click', toggleEditBody);
  btnBodyCancel.addEventListener('click', cancelEditBody);
  btnBodySave.addEventListener('click', saveEditBody);
  bodyEditTabEdit.addEventListener('click', () => {
    bodyEditTabEdit.classList.add('active');
    bodyEditTabPreview.classList.remove('active');
    bodyEditInput.style.display = 'block';
    bodyEditPreview.classList.remove('active');
  });
  bodyEditTabPreview.addEventListener('click', () => {
    bodyEditTabPreview.classList.add('active');
    bodyEditTabEdit.classList.remove('active');
    bodyEditInput.style.display = 'none';
    bodyEditPreview.classList.add('active');
    const content = bodyEditInput.value.trim();
    bodyEditPreview.innerHTML = content ? marked.parse(content) : '';
  });

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
    // Don't handle shortcuts when in input fields
    const inInput = e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT';

    if (e.key === 'Escape') {
      if (closeModalOverlay.style.display !== 'none') closeModalOverlay.style.display = 'none';
    }
    // Ctrl+N or Cmd+N for new issue
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      openEditWindow(null);
    }

    // Issue list navigation (only when not in input)
    if (!inInput && issues.length > 0) {
      if (e.key === 'ArrowDown' || e.key === 'j') {
        e.preventDefault();
        navigateIssueList(1);
      } else if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault();
        navigateIssueList(-1);
      } else if (e.key === 'Enter' && currentIssue) {
        e.preventDefault();
        openEditWindow(currentIssue.id);
      } else if (e.key === 'Home') {
        e.preventDefault();
        navigateIssueList(-Infinity);
      } else if (e.key === 'End') {
        e.preventDefault();
        navigateIssueList(Infinity);
      }
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

async function selectDirectory(path, restoreState = false) {
  try {
    const result = await invoke('select_directory', { path });
    if (result.ok) {
      directoryPath.value = shortenPath(path);
      localStorage.setItem('skis_directory', path);

      // Add to recent directories
      addRecentDirectory(path);

      if (result.data.initialized) {
        btnInit.style.display = 'none';

        // Restore app state before loading (filters, sort) - only for main window on startup
        const savedState = restoreState ? restoreAppState() : null;

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

async function createNewDatabase() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Choose location for new SKIS database'
    });
    if (selected) {
      // Select the directory first
      const selectResult = await invoke('select_directory', { path: selected });
      if (selectResult.ok) {
        if (selectResult.data.initialized) {
          // Already has a database
          if (!confirm('This directory already contains a SKIS database. Open it instead?')) {
            return;
          }
          await selectDirectory(selected);
        } else {
          // Initialize new database
          const initResult = await invoke('init_repository');
          if (initResult.ok) {
            directoryPath.value = shortenPath(selected);
            localStorage.setItem('skis_directory', selected);
            addRecentDirectory(selected);
            btnInit.style.display = 'none';
            await loadIssues();
            await loadLabels();
          } else {
            showError(initResult.error);
          }
        }
      } else {
        showError(selectResult.error);
      }
    }
  } catch (err) {
    console.error('Error creating database:', err);
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

async function exportToJson() {
  try {
    const result = await invoke('export_json');
    if (result.ok) {
      const json = JSON.stringify(result.data, null, 2);

      // Use save dialog to get file path
      const { save } = window.__TAURI__.dialog;
      const filePath = await save({
        defaultPath: 'skis-export.json',
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });

      if (filePath) {
        // Write the file
        const { writeTextFile } = window.__TAURI__.fs;
        await writeTextFile(filePath, json);
        alert(`Exported ${result.data.issues.length} issues to ${filePath}`);
      }
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
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

function navigateIssueList(direction) {
  if (issues.length === 0) return;

  let currentIndex = currentIssue ? issues.findIndex(i => i.id === currentIssue.id) : -1;

  let newIndex;
  if (direction === -Infinity) {
    // Home - go to first
    newIndex = 0;
  } else if (direction === Infinity) {
    // End - go to last
    newIndex = issues.length - 1;
  } else {
    // Relative navigation
    if (currentIndex === -1) {
      newIndex = direction > 0 ? 0 : issues.length - 1;
    } else {
      newIndex = currentIndex + direction;
    }
  }

  // Clamp to valid range
  newIndex = Math.max(0, Math.min(issues.length - 1, newIndex));

  if (newIndex !== currentIndex && issues[newIndex]) {
    selectIssue(issues[newIndex].id);

    // Scroll the selected item into view
    const selectedEl = issueList.querySelector(`[data-id="${issues[newIndex].id}"]`);
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }

    // Load more issues if near the end
    if (newIndex >= issues.length - 5 && hasMoreIssues && !isLoadingMore) {
      loadIssues(true);
    }
  }
}

async function selectIssue(id) {
  const issue = issues.find(i => i.id === id);

  if (issue) {
    currentIssue = issue;
    renderIssueList(); // Update selection
  }

  // Always load the detail - even if not in current filtered list
  await loadIssueDetail(id);

  // Update window title to show current issue
  await updateWindowTitle();

  // Save app state when issue selection changes
  saveAppState();
}

async function updateWindowTitle() {
  const win = getCurrentWindow();
  let title = 'SKIS';
  if (currentIssue) {
    // Show first ~30 chars of issue title
    const issueTitle = currentIssue.title.length > 30
      ? currentIssue.title.substring(0, 30) + '...'
      : currentIssue.title;
    title = `#${currentIssue.id} ${issueTitle} - SKIS`;
  }
  await win.setTitle(title);
  // Refresh the menu to update window list
  await invoke('refresh_window_menu');
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
    detailFooter.style.display = 'none';
    return;
  }

  detailEmpty.style.display = 'none';
  detailContent.style.display = 'block';
  detailFooter.style.display = 'flex';

  detailId.textContent = `#${currentIssue.id}`;
  detailTitle.textContent = currentIssue.title;
  detailType.textContent = currentIssue.type;
  detailType.className = `label-pill type-${currentIssue.type}`;
  detailState.textContent = currentIssue.state;
  detailState.className = `label-pill label-pill-state ${currentIssue.state}`;
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

  // Reset labels edit state (in case it was open for a different issue)
  labelsEditForm.style.display = 'none';
  detailLabels.style.display = 'flex';
  btnEditLabels.textContent = '✎';
  btnEditLabels.title = 'Edit labels';

  // Reset title edit state (in case it was open for a different issue)
  detailTitleEdit.style.display = 'none';
  detailTitle.style.display = 'block';
  btnEditTitle.textContent = '✎';
  btnEditTitle.title = 'Edit title';

  // Reset body edit state (in case it was open for a different issue)
  detailBodyEdit.style.display = 'none';
  detailBody.style.display = 'block';
  btnEditBody.textContent = '✎';
  btnEditBody.title = 'Edit';

  // Reset link form state
  linkForm.style.display = 'none';
  linkIssueId.value = '';
  btnNewLink.textContent = '+';
  btnNewLink.title = 'Link issue';

  // Reset comment form state
  commentForm.style.display = 'none';
  commentInput.value = '';
  btnNewComment.textContent = '+';
  btnNewComment.title = 'Add comment';
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
    <div class="comment-item" data-comment-id="${c.id}">
      <div class="comment-header">
        <div class="comment-meta">
          ${formatDateTime(c.created_at)}
          ${c.updated_at && c.updated_at !== c.created_at ? '<span class="comment-edited">(edited)</span>' : ''}
        </div>
        <div class="comment-actions">
          <button class="btn-icon btn-edit-comment" data-id="${c.id}" title="Edit">✎</button>
          <button class="btn-icon btn-delete-comment" data-id="${c.id}" title="Delete">×</button>
        </div>
      </div>
      <div class="comment-body markdown-body">${marked.parse(c.body)}</div>
      <div class="comment-edit-form" style="display: none;">
        <textarea class="comment-edit-input">${escapeHtml(c.body)}</textarea>
        <div class="comment-edit-actions">
          <button class="btn-secondary btn-cancel-edit">Cancel</button>
          <button class="btn-primary btn-save-edit">Save</button>
        </div>
      </div>
    </div>
  `).join('');

  // Add event handlers
  commentsList.querySelectorAll('.btn-edit-comment').forEach(btn => {
    btn.addEventListener('click', () => startEditComment(parseInt(btn.dataset.id)));
  });

  commentsList.querySelectorAll('.btn-delete-comment').forEach(btn => {
    btn.addEventListener('click', () => deleteComment(parseInt(btn.dataset.id)));
  });

  commentsList.querySelectorAll('.btn-cancel-edit').forEach(btn => {
    btn.addEventListener('click', () => {
      const item = btn.closest('.comment-item');
      cancelEditComment(item);
    });
  });

  commentsList.querySelectorAll('.btn-save-edit').forEach(btn => {
    btn.addEventListener('click', () => {
      const item = btn.closest('.comment-item');
      saveEditComment(parseInt(item.dataset.commentId), item);
    });
  });
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

function showLinkForm() {
  linkForm.style.display = 'flex';
  btnNewLink.textContent = '−';
  btnNewLink.title = 'Cancel';
  linkIssueId.focus();
}

function hideLinkForm() {
  linkForm.style.display = 'none';
  linkIssueId.value = '';
  btnNewLink.textContent = '+';
  btnNewLink.title = 'Link issue';
}

function toggleLinkForm() {
  if (linkForm.style.display === 'none' || linkForm.style.display === '') {
    showLinkForm();
  } else {
    hideLinkForm();
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
      linkForm.style.display = 'none';
      btnNewLink.textContent = '+';
      btnNewLink.title = 'Link issue';
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

// ============ Labels Inline Editing ============

function startEditLabels() {
  if (!currentIssue) return;

  // Build the labels menu with current selection
  const currentLabelNames = new Set(currentIssue.labels.map(l => l.name));

  if (labels.length === 0) {
    labelsMenuInline.innerHTML = '<span style="color: var(--color-text-muted); font-size: 0.8rem;">No labels defined</span>';
  } else {
    labelsMenuInline.innerHTML = labels.map(l => {
      const isSelected = currentLabelNames.has(l.name);
      return `
        <label class="label-toggle ${isSelected ? 'selected' : ''}" data-label="${escapeHtml(l.name)}">
          <input type="checkbox" value="${escapeHtml(l.name)}" ${isSelected ? 'checked' : ''}>
          ${renderLabelPill(l)}
        </label>
      `;
    }).join('');

    // Add change handlers
    labelsMenuInline.querySelectorAll('input[type="checkbox"]').forEach(cb => {
      cb.addEventListener('change', () => toggleLabel(cb.value, cb.checked));
    });
  }

  // Hide labels display, show edit form
  detailLabels.style.display = 'none';
  labelsEditForm.style.display = 'block';

  // Change pencil to X
  btnEditLabels.textContent = '×';
  btnEditLabels.title = 'Cancel';
}

async function toggleLabel(labelName, add) {
  if (!currentIssue) return;

  try {
    let result;
    if (add) {
      result = await invoke('add_label_to_issue', { issueId: currentIssue.id, labelName });
    } else {
      result = await invoke('remove_label_from_issue', { issueId: currentIssue.id, labelName });
    }

    if (result.ok) {
      // Update local state
      if (add) {
        const label = labels.find(l => l.name === labelName);
        if (label) {
          currentIssue.labels.push(label);
        }
      } else {
        currentIssue.labels = currentIssue.labels.filter(l => l.name !== labelName);
      }

      // Update the checkbox label styling
      const labelEl = labelsMenuInline.querySelector(`[data-label="${labelName}"]`);
      if (labelEl) {
        labelEl.classList.toggle('selected', add);
      }

      // Update the issue in the list
      const idx = issues.findIndex(i => i.id === currentIssue.id);
      if (idx !== -1) {
        issues[idx] = currentIssue;
        renderIssueList();
      }
    } else {
      showError(result.error);
      // Revert checkbox state
      const cb = labelsMenuInline.querySelector(`input[value="${labelName}"]`);
      if (cb) cb.checked = !add;
    }
  } catch (err) {
    showError(err);
  }
}

function finishEditLabels() {
  // Update the labels display
  detailLabels.innerHTML = currentIssue.labels.map(l => renderLabelPill(l)).join('');

  // Hide edit form, show labels display
  labelsEditForm.style.display = 'none';
  detailLabels.style.display = 'flex';

  // Restore pencil icon
  btnEditLabels.textContent = '✎';
  btnEditLabels.title = 'Edit labels';
}

function toggleEditLabels() {
  if (labelsEditForm.style.display === 'none' || labelsEditForm.style.display === '') {
    startEditLabels();
  } else {
    finishEditLabels();
  }
}

// ============ Title Inline Editing ============

function startEditTitle() {
  if (!currentIssue) return;

  // Hide the title, show edit form
  detailTitle.style.display = 'none';
  detailTitleEdit.style.display = 'flex';

  // Change pencil to X
  btnEditTitle.textContent = '×';
  btnEditTitle.title = 'Cancel';

  // Populate input with current title
  titleEditInput.value = currentIssue.title;

  // Focus and select all
  titleEditInput.focus();
  titleEditInput.select();
}

function cancelEditTitle() {
  // Hide edit form, show title
  detailTitleEdit.style.display = 'none';
  detailTitle.style.display = 'block';

  // Restore pencil icon
  btnEditTitle.textContent = '✎';
  btnEditTitle.title = 'Edit title';
}

function toggleEditTitle() {
  if (detailTitleEdit.style.display === 'none' || detailTitleEdit.style.display === '') {
    startEditTitle();
  } else {
    cancelEditTitle();
  }
}

async function saveEditTitle() {
  if (!currentIssue) return;

  const title = titleEditInput.value.trim();
  if (!title) {
    showError('Title cannot be empty');
    return;
  }

  try {
    const result = await invoke('update_issue', {
      id: currentIssue.id,
      params: { title }
    });
    if (result.ok) {
      currentIssue = result.data;
      detailTitle.textContent = currentIssue.title;

      // Hide edit form, show title
      detailTitleEdit.style.display = 'none';
      detailTitle.style.display = 'block';

      // Restore pencil icon
      btnEditTitle.textContent = '✎';
      btnEditTitle.title = 'Edit title';

      // Update the issue in the list
      const idx = issues.findIndex(i => i.id === currentIssue.id);
      if (idx !== -1) {
        issues[idx] = currentIssue;
        renderIssueList();
      }

      // Update window title
      await updateWindowTitle();
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

// ============ Body Inline Editing ============

function startEditBody() {
  if (!currentIssue) return;

  // Hide the rendered body, show edit form
  detailBody.style.display = 'none';
  detailBodyEdit.style.display = 'block';

  // Change pencil to X
  btnEditBody.textContent = '×';
  btnEditBody.title = 'Cancel';

  // Populate textarea with current body
  bodyEditInput.value = currentIssue.body || '';

  // Reset to Edit tab
  bodyEditTabEdit.classList.add('active');
  bodyEditTabPreview.classList.remove('active');
  bodyEditInput.style.display = 'block';
  bodyEditPreview.classList.remove('active');

  // Focus the textarea
  bodyEditInput.focus();
}

function cancelEditBody() {
  // Hide edit form, show rendered body
  detailBodyEdit.style.display = 'none';
  detailBody.style.display = 'block';

  // Restore pencil icon
  btnEditBody.textContent = '✎';
  btnEditBody.title = 'Edit';
}

function toggleEditBody() {
  if (detailBodyEdit.style.display === 'none' || detailBodyEdit.style.display === '') {
    startEditBody();
  } else {
    cancelEditBody();
  }
}

async function saveEditBody() {
  if (!currentIssue) return;

  const body = bodyEditInput.value.trim() || null;

  try {
    const result = await invoke('update_issue', {
      id: currentIssue.id,
      params: {
        body: body
      }
    });
    if (result.ok) {
      // Update the current issue and re-render
      currentIssue = result.data;

      // Re-render the body
      if (currentIssue.body) {
        detailBody.innerHTML = marked.parse(currentIssue.body);
      } else {
        detailBody.innerHTML = '';
      }

      // Hide edit form, show rendered body
      detailBodyEdit.style.display = 'none';
      detailBody.style.display = 'block';

      // Restore pencil icon
      btnEditBody.textContent = '✎';
      btnEditBody.title = 'Edit';

      // Update the issue in the list if needed
      const idx = issues.findIndex(i => i.id === currentIssue.id);
      if (idx !== -1) {
        issues[idx] = currentIssue;
      }
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

function showCommentForm() {
  commentForm.style.display = 'flex';
  btnNewComment.textContent = '−';
  btnNewComment.title = 'Cancel';
  commentInput.focus();
}

function hideCommentForm() {
  commentForm.style.display = 'none';
  commentInput.value = '';
  btnNewComment.textContent = '+';
  btnNewComment.title = 'Add comment';
}

function toggleCommentForm() {
  if (commentForm.style.display === 'none' || commentForm.style.display === '') {
    showCommentForm();
  } else {
    hideCommentForm();
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
      commentForm.style.display = 'none';
      btnNewComment.textContent = '+';
      btnNewComment.title = 'Add comment';
      await loadComments(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

function startEditComment(commentId) {
  const item = commentsList.querySelector(`[data-comment-id="${commentId}"]`);
  if (!item) return;

  // Hide body, show edit form
  item.querySelector('.comment-body').style.display = 'none';
  item.querySelector('.comment-actions').style.display = 'none';
  item.querySelector('.comment-edit-form').style.display = 'block';

  // Focus the textarea
  const textarea = item.querySelector('.comment-edit-input');
  textarea.focus();
  textarea.setSelectionRange(textarea.value.length, textarea.value.length);
}

function cancelEditComment(item) {
  item.querySelector('.comment-body').style.display = 'block';
  item.querySelector('.comment-actions').style.display = 'flex';
  item.querySelector('.comment-edit-form').style.display = 'none';
}

async function saveEditComment(commentId, item) {
  const textarea = item.querySelector('.comment-edit-input');
  const body = textarea.value.trim();

  if (!body) {
    showError('Comment cannot be empty');
    return;
  }

  try {
    const result = await invoke('update_comment', { commentId, body });
    if (result.ok) {
      await loadComments(currentIssue.id);
    } else {
      showError(result.error);
    }
  } catch (err) {
    showError(err);
  }
}

async function deleteComment(commentId) {
  if (!confirm('Delete this comment?')) return;

  try {
    const result = await invoke('delete_comment', { commentId });
    if (result.ok) {
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
