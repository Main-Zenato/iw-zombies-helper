//! Session log: each message is timestamped, written to a file and kept in memory for the
//! interface. This file is what you read after a match to understand what happened.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

const KEPT_IN_MEMORY: usize = 400;

pub struct Logbook {
    inner: Mutex<Inner>,
}

struct Inner {
    file: Option<File>,
    path: Option<PathBuf>,
    recent: VecDeque<String>,
    /// Also copies each line to standard output (command line).
    echo: bool,
}

fn now() -> SYSTEMTIME {
    let mut time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut time) };
    time
}

impl Logbook {
    /// Opens `dir/session-YYYYMMDD-HHMMSS.log`. Without a folder, or if it cannot be opened, the log
    /// stays in memory only.
    pub fn open(dir: Option<&Path>, echo: bool) -> Arc<Self> {
        let t = now();
        let mut path = None;
        let mut file = None;
        if let Some(dir) = dir {
            if std::fs::create_dir_all(dir).is_ok() {
                let candidate = dir.join(format!(
                    "session-{:04}{:02}{:02}-{:02}{:02}{:02}.log",
                    t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
                ));
                if let Ok(created) = File::create(&candidate) {
                    file = Some(created);
                    path = Some(candidate);
                }
            }
        }
        Arc::new(Self { inner: Mutex::new(Inner { file, path, recent: VecDeque::new(), echo }) })
    }

    /// Adds a message; a multi-line text gives as many timestamped lines.
    pub fn line(&self, text: impl AsRef<str>) {
        let t = now();
        let mut inner = self.inner.lock().unwrap();
        for part in text.as_ref().lines() {
            let entry = format!("{:02}:{:02}:{:02}  {part}", t.wHour, t.wMinute, t.wSecond);
            if inner.echo {
                println!("{entry}");
            }
            if let Some(file) = inner.file.as_mut() {
                let _ = writeln!(file, "{entry}");
            }
            if inner.recent.len() == KEPT_IN_MEMORY {
                inner.recent.pop_front();
            }
            inner.recent.push_back(entry);
        }
    }

    pub fn recent(&self) -> Vec<String> {
        self.inner.lock().unwrap().recent.iter().cloned().collect()
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.inner.lock().unwrap().path.clone()
    }

    /// Log folder, when the log is written to disk. What goes with the logs (full surveys, current
    /// state) is stored there too; an in-memory log writes nothing.
    pub fn dir(&self) -> Option<PathBuf> {
        self.path().and_then(|path| path.parent().map(Path::to_path_buf))
    }
}

/// Local date and time for a file name (`YYYYMMDD-HHMMSS`) and for text (`DD/MM/YYYY HH:MM:SS`).
pub fn timestamps() -> (String, String) {
    let t = now();
    (
        format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond),
        format!("{:02}/{:02}/{:04} {:02}:{:02}:{:02}", t.wDay, t.wMonth, t.wYear, t.wHour, t.wMinute, t.wSecond),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_timestamped_lines_to_file_and_memory() {
        let dir = std::env::temp_dir().join(format!("iw-helper-logtest-{}", std::process::id()));
        let log = Logbook::open(Some(&dir), false);
        log.line("first line\nsecond line");
        let recent = log.recent();
        assert_eq!(recent.len(), 2);
        assert!(recent[1].ends_with("  second line"));
        let written = std::fs::read_to_string(log.path().unwrap()).unwrap();
        assert_eq!(written.lines().count(), 2);
        assert!(written.contains("first line"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_only_the_most_recent_lines_in_memory() {
        let log = Logbook::open(None, false);
        for i in 0..KEPT_IN_MEMORY + 10 {
            log.line(format!("ligne {i}"));
        }
        let recent = log.recent();
        assert_eq!(recent.len(), KEPT_IN_MEMORY);
        assert!(recent[0].ends_with("ligne 10"));
        assert!(log.path().is_none());
    }
}
