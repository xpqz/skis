// SKIS GUI - Edit Window
const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { emit } = window.__TAURI__.event;

const STORAGE_EDIT_WINDOW_STATE = 'skis_edit_window_state';

let issueId = null;
let labels = [];
let selectedLabels = new Set();
let originalLabels = new Set(); // Track original labels for diffing on save

const inputTitle = document.getElementById('input-title');
const inputType = document.getElementById('input-type');
const inputBody = document.getElementById('input-body');
const btnCancel = document.getElementById('btn-cancel');
const btnSave = document.getElementById('btn-save');
const issueForm = document.getElementById('issue-form');

// Labels dropdown elements
const labelsDropdown = document.getElementById('labels-dropdown');
const labelsTrigger = document.getElementById('labels-trigger');
const labelsMenu = document.getElementById('labels-menu');
const selectedLabelsEl = document.getElementById('selected-labels');

async function init() {
  // Restore window size/position
  await restoreWindowState();

  // Get issue ID from window label (format: "edit-{id}" or "new")
  const win = getCurrentWindow();
  const label = win.label;

  // Save window state on resize/move
  win.onResized(debounce(saveWindowState, 500));
  win.onMoved(debounce(saveWindowState, 500));

  if (label.startsWith('edit-')) {
    issueId = parseInt(label.substring(5));
    document.title = `Edit Issue #${issueId}`;
    btnSave.textContent = 'Save Changes';
  } else {
    issueId = null;
    document.title = 'New Issue';
    btnSave.textContent = 'Create Issue';
  }

  // Load labels
  try {
    const result = await invoke('list_labels');
    if (result.ok) {
      labels = result.data;
    }
  } catch (e) {
    console.error('Could not load labels:', e);
  }

  // Load issue data if editing
  if (issueId) {
    try {
      const result = await invoke('get_issue', { id: issueId });
      if (result.ok) {
        const issue = result.data;
        inputTitle.value = issue.title;
        inputType.value = issue.type;
        inputBody.value = issue.body || '';

        // Select issue's labels
        issue.labels.forEach(l => {
          selectedLabels.add(l.name);
          originalLabels.add(l.name);
        });
      }
    } catch (e) {
      console.error('Could not load issue:', e);
    }
  }

  renderLabelsDropdown();
  setupLabelsDropdown();
  inputTitle.focus();
}

function renderLabelsDropdown() {
  // Render menu items
  if (labels.length === 0) {
    labelsMenu.innerHTML = '<div class="labels-menu-empty">No labels defined</div>';
  } else {
    labelsMenu.innerHTML = labels.map(l => {
      const color = l.color || '888888';
      const textColor = getContrastColor(color);
      return `
        <label class="labels-menu-item">
          <input type="checkbox" value="${escapeHtml(l.name)}" ${selectedLabels.has(l.name) ? 'checked' : ''}>
          <span class="label-tag" style="background-color: #${color}; color: ${textColor};">${escapeHtml(l.name)}</span>
        </label>
      `;
    }).join('');
  }

  // Render selected labels in trigger
  renderSelectedLabels();
}

function renderSelectedLabels() {
  if (selectedLabels.size === 0) {
    selectedLabelsEl.innerHTML = '';
  } else {
    selectedLabelsEl.innerHTML = Array.from(selectedLabels).map(name => {
      const label = labels.find(l => l.name === name);
      const color = label?.color || '888888';
      const textColor = getContrastColor(color);
      return `<span class="label-tag" style="background-color: #${color}; color: ${textColor};">${escapeHtml(name)}</span>`;
    }).join('');
  }
}

function setupLabelsDropdown() {
  // Toggle dropdown
  labelsTrigger.addEventListener('click', () => {
    const isOpen = labelsMenu.classList.contains('open');
    labelsMenu.classList.toggle('open');
    labelsTrigger.classList.toggle('open');
  });

  // Handle checkbox changes
  labelsMenu.addEventListener('change', e => {
    if (e.target.type === 'checkbox') {
      if (e.target.checked) {
        selectedLabels.add(e.target.value);
      } else {
        selectedLabels.delete(e.target.value);
      }
      renderSelectedLabels();
    }
  });

  // Close on click outside
  document.addEventListener('click', e => {
    if (!labelsDropdown.contains(e.target)) {
      labelsMenu.classList.remove('open');
      labelsTrigger.classList.remove('open');
    }
  });
}

function getContrastColor(hexColor) {
  const r = parseInt(hexColor.substr(0, 2), 16);
  const g = parseInt(hexColor.substr(2, 2), 16);
  const b = parseInt(hexColor.substr(4, 2), 16);
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.5 ? '#000' : '#fff';
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

async function save() {
  try {
    let result;
    if (issueId) {
      // Update existing issue
      result = await invoke('update_issue', {
        id: issueId,
        params: {
          title: inputTitle.value,
          body: inputBody.value || null,
          issue_type: inputType.value
        }
      });

      if (result.ok) {
        // Update labels: find added and removed
        const labelsToAdd = Array.from(selectedLabels).filter(l => !originalLabels.has(l));
        const labelsToRemove = Array.from(originalLabels).filter(l => !selectedLabels.has(l));

        // Add new labels
        for (const labelName of labelsToAdd) {
          await invoke('add_label_to_issue', { issueId, labelName });
        }

        // Remove old labels
        for (const labelName of labelsToRemove) {
          await invoke('remove_label_from_issue', { issueId, labelName });
        }
      }
    } else {
      // Create new issue (labels included in create)
      result = await invoke('create_issue', {
        params: {
          title: inputTitle.value,
          body: inputBody.value || null,
          issue_type: inputType.value,
          labels: Array.from(selectedLabels)
        }
      });
    }

    if (result.ok) {
      // Emit event to main window to refresh
      await emit('issue-saved', { id: result.data.id });
      const win = getCurrentWindow();
      await win.close();
    } else {
      alert(`Error: ${result.error}`);
    }
  } catch (e) {
    alert(`Error: ${e}`);
  }
}

// Event listeners
issueForm.addEventListener('submit', e => {
  e.preventDefault();
  save();
});

btnCancel.addEventListener('click', async () => {
  const win = getCurrentWindow();
  await win.close();
});

// Keyboard shortcuts
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') {
    // Close dropdown first if open, then close window
    if (labelsMenu.classList.contains('open')) {
      labelsMenu.classList.remove('open');
      labelsTrigger.classList.remove('open');
    } else {
      getCurrentWindow().close();
    }
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault();
    save();
  }
});

// ============ Window State ============

function debounce(fn, delay) {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), delay);
  };
}

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
    localStorage.setItem(STORAGE_EDIT_WINDOW_STATE, JSON.stringify(state));
  } catch (e) {
    console.error('Could not save window state:', e);
  }
}

async function restoreWindowState() {
  try {
    const stored = localStorage.getItem(STORAGE_EDIT_WINDOW_STATE);
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

init();
