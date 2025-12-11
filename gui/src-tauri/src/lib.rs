use serde::{Deserialize, Serialize};
use ski::{
    Comment, Issue, IssueCreate, IssueFilter, IssueState, IssueType, IssueUpdate, Label,
    LinkedIssueRef, SkisDb, SortField, SortOrder, StateReason,
};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Emitter, Manager, State};

// Application state holding the database connection
pub struct AppState {
    db: Mutex<Option<SkisDb>>,
    current_dir: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db: Mutex::new(None),
            current_dir: Mutex::new(None),
        }
    }
}

// Response wrapper for consistent API responses
#[derive(Debug, Serialize)]
pub struct Response<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> Response<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Response<T> {
        Response {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// Helper to get database connection or return error
macro_rules! with_db {
    ($state:expr, $body:expr) => {{
        let db_guard = $state.db.lock().unwrap();
        match db_guard.as_ref() {
            Some(db) => $body(db),
            None => Response::err("No SKIS repository open. Please select a directory."),
        }
    }};
}

// Extended issue view with labels and links
#[derive(Debug, Serialize)]
pub struct IssueView {
    #[serde(flatten)]
    pub issue: Issue,
    pub labels: Vec<Label>,
    pub linked_issues: Vec<LinkedIssueRef>,
}

// Filter parameters from frontend
#[derive(Debug, Deserialize)]
pub struct FilterParams {
    pub state: Option<String>,
    pub issue_type: Option<String>,
    pub labels: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_deleted: Option<bool>,
    pub search: Option<String>,
}

impl FilterParams {
    fn to_filter(&self) -> IssueFilter {
        let mut filter = IssueFilter::default();

        if let Some(state) = &self.state {
            filter.state = match state.to_lowercase().as_str() {
                "open" => Some(IssueState::Open),
                "closed" => Some(IssueState::Closed),
                _ => None,
            };
        }

        if let Some(issue_type) = &self.issue_type {
            filter.issue_type = issue_type.parse().ok();
        }

        if let Some(labels) = &self.labels {
            filter.labels = labels.clone();
        }

        if let Some(sort_by) = &self.sort_by {
            filter.sort_by = match sort_by.to_lowercase().as_str() {
                "created" => SortField::Created,
                "updated" => SortField::Updated,
                "id" => SortField::Id,
                _ => SortField::Updated,
            };
        }

        if let Some(sort_order) = &self.sort_order {
            filter.sort_order = match sort_order.to_lowercase().as_str() {
                "asc" => SortOrder::Asc,
                "desc" => SortOrder::Desc,
                _ => SortOrder::Desc,
            };
        }

        if let Some(limit) = self.limit {
            filter.limit = limit as usize;
        }

        if let Some(offset) = self.offset {
            filter.offset = offset as usize;
        }

        if let Some(include_deleted) = self.include_deleted {
            filter.include_deleted = include_deleted;
        }

        filter
    }
}

// Issue create parameters from frontend
#[derive(Debug, Deserialize)]
pub struct CreateIssueParams {
    pub title: String,
    pub body: Option<String>,
    pub issue_type: Option<String>,
    pub labels: Option<Vec<String>>,
}

// Issue update parameters from frontend
#[derive(Debug, Deserialize)]
pub struct UpdateIssueParams {
    pub title: Option<String>,
    pub body: Option<String>,
    pub issue_type: Option<String>,
}

// Directory state response
#[derive(Debug, Serialize)]
pub struct DirectoryState {
    pub path: Option<String>,
    pub initialized: bool,
}

// ============ Directory Commands ============

#[tauri::command]
fn get_current_dir(state: State<AppState>) -> Response<DirectoryState> {
    let dir_guard = state.current_dir.lock().unwrap();
    let db_guard = state.db.lock().unwrap();

    Response::ok(DirectoryState {
        path: dir_guard.as_ref().map(|p| p.display().to_string()),
        initialized: db_guard.is_some(),
    })
}

#[tauri::command]
fn select_directory(state: State<AppState>, path: String) -> Response<DirectoryState> {
    let dir_path = PathBuf::from(&path);
    let skis_dir = dir_path.join(".skis");

    // Try to open existing SKIS repository
    match SkisDb::open_at(&skis_dir) {
        Ok(db) => {
            let mut db_guard = state.db.lock().unwrap();
            let mut dir_guard = state.current_dir.lock().unwrap();
            *db_guard = Some(db);
            *dir_guard = Some(dir_path);
            Response::ok(DirectoryState {
                path: Some(path),
                initialized: true,
            })
        }
        Err(_) => {
            // Not initialized - store directory but no db
            let mut dir_guard = state.current_dir.lock().unwrap();
            let mut db_guard = state.db.lock().unwrap();
            *dir_guard = Some(dir_path);
            *db_guard = None;
            Response::ok(DirectoryState {
                path: Some(path),
                initialized: false,
            })
        }
    }
}

#[tauri::command]
fn init_repository(state: State<AppState>) -> Response<DirectoryState> {
    let dir_guard = state.current_dir.lock().unwrap();
    let dir_path = match dir_guard.as_ref() {
        Some(p) => p.clone(),
        None => return Response::err("No directory selected"),
    };
    drop(dir_guard);

    match SkisDb::init(&dir_path) {
        Ok(db) => {
            let mut db_guard = state.db.lock().unwrap();
            *db_guard = Some(db);
            Response::ok(DirectoryState {
                path: Some(dir_path.display().to_string()),
                initialized: true,
            })
        }
        Err(e) => Response::err(e.to_string()),
    }
}

#[tauri::command]
fn get_home_dir() -> Response<String> {
    match dirs::home_dir() {
        Some(p) => Response::ok(p.display().to_string()),
        None => Response::err("Could not determine home directory"),
    }
}

// ============ Issue Commands ============

#[tauri::command]
fn list_issues(state: State<AppState>, filter: FilterParams) -> Response<Vec<IssueView>> {
    with_db!(state, |db: &SkisDb| {
        let issue_filter = filter.to_filter();

        let issues = if let Some(search) = &filter.search {
            match ski::db::search_issues(db.conn(), search, &issue_filter) {
                Ok(i) => i,
                Err(e) => return Response::err(e.to_string()),
            }
        } else {
            match ski::db::list_issues(db.conn(), &issue_filter) {
                Ok(i) => i,
                Err(e) => return Response::err(e.to_string()),
            }
        };

        // Enrich each issue with labels and links
        let mut views = Vec::with_capacity(issues.len());
        for issue in issues {
            let labels = ski::db::get_issue_labels(db.conn(), issue.id).unwrap_or_default();
            let linked_issues =
                ski::db::get_linked_issues_with_titles(db.conn(), issue.id).unwrap_or_default();
            views.push(IssueView {
                issue,
                labels,
                linked_issues,
            });
        }

        Response::ok(views)
    })
}

#[tauri::command]
fn get_issue(state: State<AppState>, id: i64) -> Response<IssueView> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::get_issue(db.conn(), id) {
            Ok(Some(issue)) => {
                let labels = ski::db::get_issue_labels(db.conn(), id).unwrap_or_default();
                let linked_issues =
                    ski::db::get_linked_issues_with_titles(db.conn(), id).unwrap_or_default();
                Response::ok(IssueView {
                    issue,
                    labels,
                    linked_issues,
                })
            }
            Ok(None) => Response::err(format!("Issue #{} not found", id)),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn create_issue(state: State<AppState>, params: CreateIssueParams) -> Response<IssueView> {
    with_db!(state, |db: &SkisDb| {
        let issue_type = params
            .issue_type
            .as_ref()
            .and_then(|t| t.parse().ok())
            .unwrap_or(IssueType::Task);

        let create = IssueCreate {
            title: params.title,
            body: params.body,
            issue_type,
            labels: params.labels.unwrap_or_default(),
        };

        match ski::db::create_issue(db.conn(), &create) {
            Ok(issue) => {
                let labels = ski::db::get_issue_labels(db.conn(), issue.id).unwrap_or_default();
                Response::ok(IssueView {
                    issue,
                    labels,
                    linked_issues: vec![],
                })
            }
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn update_issue(state: State<AppState>, id: i64, params: UpdateIssueParams) -> Response<IssueView> {
    with_db!(state, |db: &SkisDb| {
        let update = IssueUpdate {
            title: params.title,
            body: params.body,
            issue_type: params.issue_type.as_ref().and_then(|t| t.parse().ok()),
        };

        match ski::db::update_issue(db.conn(), id, &update) {
            Ok(issue) => {
                let labels = ski::db::get_issue_labels(db.conn(), id).unwrap_or_default();
                let linked_issues =
                    ski::db::get_linked_issues_with_titles(db.conn(), id).unwrap_or_default();
                Response::ok(IssueView {
                    issue,
                    labels,
                    linked_issues,
                })
            }
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn close_issue(
    state: State<AppState>,
    id: i64,
    reason: Option<String>,
    comment: Option<String>,
) -> Response<IssueView> {
    with_db!(state, |db: &SkisDb| {
        let state_reason = reason
            .as_ref()
            .and_then(|r| match r.to_lowercase().as_str() {
                "completed" => Some(StateReason::Completed),
                "not_planned" => Some(StateReason::NotPlanned),
                _ => None,
            })
            .unwrap_or(StateReason::Completed);

        let result = match comment {
            Some(c) => ski::db::close_issue_with_comment(db.conn(), id, state_reason, Some(&c)),
            None => ski::db::close_issue(db.conn(), id, state_reason),
        };

        match result {
            Ok(issue) => {
                let labels = ski::db::get_issue_labels(db.conn(), id).unwrap_or_default();
                let linked_issues =
                    ski::db::get_linked_issues_with_titles(db.conn(), id).unwrap_or_default();
                Response::ok(IssueView {
                    issue,
                    labels,
                    linked_issues,
                })
            }
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn reopen_issue(state: State<AppState>, id: i64) -> Response<IssueView> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::reopen_issue(db.conn(), id) {
            Ok(issue) => {
                let labels = ski::db::get_issue_labels(db.conn(), id).unwrap_or_default();
                let linked_issues =
                    ski::db::get_linked_issues_with_titles(db.conn(), id).unwrap_or_default();
                Response::ok(IssueView {
                    issue,
                    labels,
                    linked_issues,
                })
            }
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn delete_issue(state: State<AppState>, id: i64) -> Response<()> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::delete_issue(db.conn(), id) {
            Ok(()) => Response::ok(()),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn restore_issue(state: State<AppState>, id: i64) -> Response<IssueView> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::restore_issue(db.conn(), id) {
            Ok(issue) => {
                let labels = ski::db::get_issue_labels(db.conn(), id).unwrap_or_default();
                let linked_issues =
                    ski::db::get_linked_issues_with_titles(db.conn(), id).unwrap_or_default();
                Response::ok(IssueView {
                    issue,
                    labels,
                    linked_issues,
                })
            }
            Err(e) => Response::err(e.to_string()),
        }
    })
}

// ============ Comment Commands ============

#[tauri::command]
fn get_comments(state: State<AppState>, issue_id: i64) -> Response<Vec<Comment>> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::get_comments(db.conn(), issue_id) {
            Ok(comments) => Response::ok(comments),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn add_comment(state: State<AppState>, issue_id: i64, body: String) -> Response<Comment> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::add_comment(db.conn(), issue_id, &body) {
            Ok(comment) => Response::ok(comment),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

// ============ Label Commands ============

#[tauri::command]
fn list_labels(state: State<AppState>) -> Response<Vec<Label>> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::list_labels(db.conn()) {
            Ok(labels) => Response::ok(labels),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn create_label(
    state: State<AppState>,
    name: String,
    description: Option<String>,
    color: Option<String>,
) -> Response<Label> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::create_label(
            db.conn(),
            &name,
            description.as_deref(),
            color.as_deref(),
        ) {
            Ok(label) => Response::ok(label),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn delete_label(state: State<AppState>, name: String) -> Response<()> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::delete_label(db.conn(), &name) {
            Ok(()) => Response::ok(()),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn add_label_to_issue(state: State<AppState>, issue_id: i64, label_name: String) -> Response<()> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::add_label_to_issue(db.conn(), issue_id, &label_name) {
            Ok(()) => Response::ok(()),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn remove_label_from_issue(
    state: State<AppState>,
    issue_id: i64,
    label_name: String,
) -> Response<()> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::remove_label_from_issue(db.conn(), issue_id, &label_name) {
            Ok(()) => Response::ok(()),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

// ============ Link Commands ============

#[tauri::command]
fn link_issues(state: State<AppState>, issue_a: i64, issue_b: i64) -> Response<()> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::add_link(db.conn(), issue_a, issue_b) {
            Ok(()) => Response::ok(()),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

#[tauri::command]
fn unlink_issues(state: State<AppState>, issue_a: i64, issue_b: i64) -> Response<()> {
    with_db!(state, |db: &SkisDb| {
        match ski::db::remove_link(db.conn(), issue_a, issue_b) {
            Ok(()) => Response::ok(()),
            Err(e) => Response::err(e.to_string()),
        }
    })
}

// ============ Window Commands ============

#[tauri::command]
fn open_edit_window(app: AppHandle, issue_id: Option<i64>) -> Response<()> {
    let label = match issue_id {
        Some(id) => format!("edit-{}", id),
        None => "new".to_string(),
    };

    let title = match issue_id {
        Some(id) => format!("Edit Issue #{}", id),
        None => "New Issue".to_string(),
    };

    // Check if window already exists
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_focus();
        return Response::ok(());
    }

    // Create new window
    match WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App("edit.html".into()))
        .title(&title)
        .inner_size(600.0, 580.0)
        .min_inner_size(500.0, 450.0)
        .resizable(true)
        .build()
    {
        Ok(_) => Response::ok(()),
        Err(e) => Response::err(e.to_string()),
    }
}

// ============ Menu Commands ============

#[tauri::command]
fn update_recent_menu(app: AppHandle, paths: Vec<String>) -> Response<()> {
    if let Err(e) = rebuild_menu(&app, &paths) {
        return Response::err(e.to_string());
    }
    Response::ok(())
}

fn rebuild_menu(app: &AppHandle, recent_paths: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Build "Open Recent" submenu
    let mut recent_submenu = SubmenuBuilder::new(app, "Open Recent");

    if recent_paths.is_empty() {
        recent_submenu = recent_submenu.item(
            &MenuItemBuilder::new("No Recent Items")
                .enabled(false)
                .build(app)?,
        );
    } else {
        for path in recent_paths {
            // Use last component of path as label, full path as id
            let label = PathBuf::from(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());

            let item = MenuItemBuilder::new(&label)
                .id(format!("recent:{}", path))
                .build(app)?;
            recent_submenu = recent_submenu.item(&item);
        }
    }

    // Build File menu
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(
            &MenuItemBuilder::new("Open...")
                .id("open")
                .accelerator("CmdOrCtrl+O")
                .build(app)?,
        )
        .item(&recent_submenu.build()?)
        .separator()
        .item(
            &MenuItemBuilder::new("New Issue")
                .id("new-issue")
                .accelerator("CmdOrCtrl+N")
                .build(app)?,
        )
        .separator()
        .quit()
        .build()?;

    // Build Edit menu
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // Build View menu
    let view_menu = SubmenuBuilder::new(app, "View")
        .item(
            &MenuItemBuilder::new("Toggle Sidebar")
                .id("toggle-sidebar")
                .accelerator("CmdOrCtrl+\\")
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::new("Reload")
                .id("reload")
                .accelerator("CmdOrCtrl+R")
                .build(app)?,
        )
        .build()?;

    // Build full menu
    let menu = MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()?;

    app.set_menu(menu)?;
    Ok(())
}

// ============ App Entry Point ============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .setup(|app| {
            // Build initial menu with empty recent list
            rebuild_menu(app.handle(), &[])?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "open" {
                let _ = app.emit("menu-open", ());
            } else if id == "reload" {
                let _ = app.emit("menu-reload", ());
            } else if id == "toggle-sidebar" {
                let _ = app.emit("menu-toggle-sidebar", ());
            } else if let Some(path) = id.strip_prefix("recent:") {
                let _ = app.emit("menu-open-recent", path);
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Directory
            get_current_dir,
            select_directory,
            init_repository,
            get_home_dir,
            // Issues
            list_issues,
            get_issue,
            create_issue,
            update_issue,
            close_issue,
            reopen_issue,
            delete_issue,
            restore_issue,
            // Comments
            get_comments,
            add_comment,
            // Labels
            list_labels,
            create_label,
            delete_label,
            add_label_to_issue,
            remove_label_from_issue,
            // Links
            link_issues,
            unlink_issues,
            // Windows
            open_edit_window,
            // Menu
            update_recent_menu,
        ])
        .run(tauri::generate_context!())
        .expect("error running SKIS GUI");
}
