use std::path::PathBuf;

use ski::Result;

/// Get the GUI log directory path
fn get_log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("skis")
        .join("logs")
}

pub fn run() -> Result<()> {
    let log_dir = get_log_dir();

    // Find the most recent log file
    if log_dir.exists() {
        let mut log_files: Vec<_> = std::fs::read_dir(&log_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("skis.log")
            })
            .collect();

        // Sort by modification time (most recent first)
        log_files.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        if let Some(latest) = log_files.first() {
            println!("{}", latest.path().display());
            return Ok(());
        }
    }

    // No log file found, print the directory where logs would be
    println!("{}", log_dir.join("skis.log").display());
    Ok(())
}
