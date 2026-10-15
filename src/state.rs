//! What the tool changed in the game and has not restored yet, written down in a small file.
//!
//! If the tool is closed while options are still on, the game keeps them. When it reopens, the tool
//! must be able to restore them, but only those: a flag or value the game set by itself is not ours.
//! So the file names one precise process (pid and creation time) and the list of what the tool
//! turned on there. Any other case, a file from another process, a missing or unreadable file,
//! gives nothing to restore.
//!
//! The file lives in the log folder, next to the logs, never in the repository.

use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "etat-en-cours.txt";

/// What the tool left in the game. Options kept up by rewriting (ammo, money, one-hit kill) are not
/// in it: they stop with the tool. Nor are patched instructions and the old "no target" health: they
/// are recognised by their exact value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pending {
    pub god_mode: bool,
    pub noclip: bool,
    pub jump_pack: bool,
    /// The player's `ignoreme` field is set.
    pub no_target: bool,
    /// Game speed is no longer 1.
    pub timescale: bool,
    /// Movement speed is no longer 190.
    pub move_speed: bool,
    /// Original jump height, to put back into the Zombies suits.
    pub jump_height: Option<f32>,
    /// The `level` money multiplier is no longer 1.
    pub cash_scalar: bool,
    /// Zombies paused: number of zombies at a time to give back to the game.
    pub paused_zombies: Option<i32>,
}

impl Pending {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

fn path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// Writes down what is left to restore for this process. With nothing to restore, the file is deleted.
pub fn save(dir: &Path, pid: u32, created: u64, pending: &Pending) {
    if pending.is_empty() {
        let _ = std::fs::remove_file(path(dir));
        return;
    }
    let mut text = format!("pid={pid}\ncreated={created}\n");
    for (on, name) in [(pending.god_mode, "god_mode"), (pending.noclip, "noclip"), (pending.jump_pack, "jump_pack"), (pending.no_target, "no_target"), (pending.timescale, "timescale"), (pending.move_speed, "move_speed"), (pending.cash_scalar, "cash_scalar")] {
        if on {
            text.push_str(name);
            text.push('\n');
        }
    }
    if let Some(height) = pending.jump_height {
        text.push_str(&format!("jump_height={height}\n"));
    }
    if let Some(count) = pending.paused_zombies {
        text.push_str(&format!("paused_zombies={count}\n"));
    }
    let _ = std::fs::write(path(dir), text);
}

/// Reads what a previous session left in THIS process, then deletes the file: it is used only once.
/// Returns an empty list if the file is missing, unreadable, or about another process.
pub fn take(dir: &Path, pid: u32, created: u64) -> Pending {
    let text = std::fs::read_to_string(path(dir));
    let _ = std::fs::remove_file(path(dir));
    text.ok().and_then(|text| parse(&text, pid, created)).unwrap_or_default()
}

fn parse(text: &str, pid: u32, created: u64) -> Option<Pending> {
    let mut lines = text.lines();
    let same = lines.next()?.strip_prefix("pid=")?.parse::<u32>().ok()? == pid && lines.next()?.strip_prefix("created=")?.parse::<u64>().ok()? == created;
    if !same {
        return None;
    }
    let mut pending = Pending::default();
    for line in lines {
        match line {
            "god_mode" => pending.god_mode = true,
            "noclip" => pending.noclip = true,
            "jump_pack" => pending.jump_pack = true,
            "no_target" => pending.no_target = true,
            "timescale" => pending.timescale = true,
            "move_speed" => pending.move_speed = true,
            "cash_scalar" => pending.cash_scalar = true,
            other => match other.split_once('=') {
                Some(("jump_height", height)) => pending.jump_height = Some(height.parse::<f32>().ok().filter(|height| height.is_finite())?),
                Some(("paused_zombies", count)) => pending.paused_zombies = Some(count.parse::<i32>().ok()?),
                // An unknown line: the file is not the one we think, nothing is kept from it.
                _ => return None,
            },
        }
    }
    Some(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("iw-helper-etat-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_file_only_speaks_for_its_own_process_and_only_once() {
        let dir = scratch("processus");
        let pending = Pending { god_mode: true, timescale: true, jump_height: Some(39.0), cash_scalar: true, paused_zombies: Some(24), ..Pending::default() };
        save(&dir, 4242, 1_000, &pending);
        // Same pid but another creation time: it is another process. The file is deleted anyway.
        assert_eq!(take(&dir, 4242, 1_001), Pending::default());
        assert!(!path(&dir).exists());
        save(&dir, 4242, 1_000, &pending);
        assert_eq!(take(&dir, 4243, 1_000), Pending::default());
        // The right process: the list comes back as is, only once.
        save(&dir, 4242, 1_000, &pending);
        assert_eq!(take(&dir, 4242, 1_000), pending);
        assert_eq!(take(&dir, 4242, 1_000), Pending::default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn nothing_pending_means_no_file_and_garbage_means_nothing() {
        let dir = scratch("vide");
        save(&dir, 1, 2, &Pending { noclip: true, jump_pack: true, ..Pending::default() });
        assert!(path(&dir).exists());
        save(&dir, 1, 2, &Pending::default());
        assert!(!path(&dir).exists());
        for garbage in ["", "pid=1", "pid=1\ncreated=2\nformat_c", "pid=1\ncreated=2\njump_height=NaN", "pid=1\ncreated=2\npaused_zombies=x", "pid=x\ncreated=2\ngod_mode"] {
            std::fs::write(path(&dir), garbage).unwrap();
            assert_eq!(take(&dir, 1, 2), Pending::default(), "{garbage:?}");
            assert!(!path(&dir).exists());
        }
        // A folder that does not exist: nothing to read, nothing written, no panic.
        let missing = dir.join("absent");
        save(&missing, 1, 2, &Pending { god_mode: true, ..Pending::default() });
        assert_eq!(take(&missing, 1, 2), Pending::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
