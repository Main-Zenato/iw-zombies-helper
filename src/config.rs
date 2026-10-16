use crate::lang::{self, Lang};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "iw-helper.toml";
pub const STEAM_CLIENT: &str = "iw7_ship.exe";
/// The community client: no VAC, and it loads the scripts of iw7-mod/custom_scripts.
pub const MOD_CLIENT: &str = "iw7-mod.exe";

/// Actions that are not toggles but also have a shortcut.
pub const ACTION_ALL_OFF: &str = "all_off";
pub const ACTION_SAVE_POSITION: &str = "save_position";
pub const ACTION_TELEPORT: &str = "teleport";

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub process_names: Vec<String>,
    pub cpu: Cpu,
    pub memory: Memory,
    pub fps: Fps,
    pub trainer: Trainer,
    pub interface: Interface,
    /// Action name -> key name. An empty string removes the shortcut.
    pub hotkeys: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Cpu {
    pub enabled: bool,
    pub smt: bool,
    pub max_cores: usize,
    pub priority: Priority,
    pub reapply_minutes: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Normal,
    AboveNormal,
    High,
}

impl Priority {
    pub const ALL: [Priority; 3] = [Priority::Normal, Priority::AboveNormal, Priority::High];

    /// Value written in the configuration file.
    pub fn key(self) -> &'static str {
        match self {
            Priority::Normal => "normal",
            Priority::AboveNormal => "above_normal",
            Priority::High => "high",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Priority::Normal => lang::tr("Normal", "Normale"),
            Priority::AboveNormal => lang::tr("Above normal", "Supérieure à la normale"),
            Priority::High => lang::tr("High", "Haute"),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Memory {
    pub allow_steam_client: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fps {
    pub enabled: bool,
    pub limit: i32,
}

/// Interface settings: the language of the text, English by default.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Interface {
    pub language: Lang,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Trainer {
    pub enabled: bool,
    pub timescale: f32,
    pub money: i32,
    /// Confirmation beep on every action.
    pub sound: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            process_names: vec![STEAM_CLIENT.into(), MOD_CLIENT.into()],
            cpu: Cpu::default(),
            memory: Memory::default(),
            fps: Fps::default(),
            trainer: Trainer::default(),
            interface: Interface::default(),
            hotkeys: default_hotkeys(),
        }
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self { enabled: true, smt: false, max_cores: 0, priority: Priority::High, reapply_minutes: 0 }
    }
}

impl Default for Fps {
    fn default() -> Self {
        Self { enabled: false, limit: 1000 }
    }
}

impl Default for Trainer {
    fn default() -> Self {
        Self { enabled: false, timescale: 0.5, money: 100_000, sound: true }
    }
}

/// Default shortcuts: function keys, usable without a numeric keypad. F12 is avoided
/// (Steam screenshot).
fn default_hotkeys() -> BTreeMap<String, String> {
    [
        ("god_mode", "F1"),
        ("infinite_ammo", "F2"),
        ("one_hit_kill", "F3"),
        ("infinite_money", "F4"),
        ("noclip", "F5"),
        ("no_target", "F6"),
        ("slow_motion", "F7"),
        ("no_recoil", "F8"),
        ("rapid_fire", "F9"),
        ("low_gravity", "F10"),
        ("jump_pack", "F11"),
        ("pause_zombies", ""),
        // One-off actions: no key by default, to pick in the configuration file.
        ("refill_ammo", ""),
        ("add_money", ""),
        ("weaken_zombies", ""),
        ("end_round", ""),
        ("charge_cards", ""),
        ("add_tickets", ""),
        // Carried out by the iw7-mod script: iw7-mod only.
        ("upgrade_weapons", ""),
        ("kill_all", ""),
        ("open_doors", ""),
        ("all_perks", ""),
        ("box_weapon", ""),
        ("drop_max_ammo", ""),
        ("drop_nuke", ""),
        ("drop_instakill", ""),
        ("drop_double_money", ""),
        ("drop_fire_sale", ""),
        ("drop_infinite_ammo", ""),
        (ACTION_SAVE_POSITION, "PageUp"),
        (ACTION_TELEPORT, "PageDown"),
        (ACTION_ALL_OFF, "End"),
    ]
    .into_iter()
    .map(|(action, key)| (action.to_string(), key.to_string()))
    .collect()
}

/// Windows virtual-key code for a key name, case-insensitive.
pub fn key_code(name: &str) -> Option<u16> {
    let upper = name.trim().to_ascii_uppercase();
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()) {
        return (1..=24).contains(&n).then_some(0x70 + n - 1);
    }
    if let Some(n) = upper.strip_prefix("NUM").and_then(|n| n.parse::<u16>().ok()) {
        return (n <= 9).then_some(0x60 + n);
    }
    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        return (c.is_ascii_uppercase() || c.is_ascii_digit()).then_some(c as u16);
    }
    Some(match upper.as_str() {
        "INSERT" | "INSER" => 0x2D,
        "DELETE" | "SUPPR" => 0x2E,
        "HOME" | "DEBUT" => 0x24,
        "END" | "FIN" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "PAUSE" => 0x13,
        "SCROLLLOCK" => 0x91,
        "UP" => 0x26,
        "DOWN" => 0x28,
        "LEFT" => 0x25,
        "RIGHT" => 0x27,
        "NUMDECIMAL" => 0x6E,
        "NUMMULTIPLY" => 0x6A,
        "NUMDIVIDE" => 0x6F,
        "NUMADD" => 0x6B,
        "NUMSUBTRACT" => 0x6D,
        _ => return None,
    })
}

impl Config {
    pub fn wants_memory(&self) -> bool {
        self.fps.enabled || self.trainer.enabled
    }

    /// Memory features only touch the VAC-protected Steam client with explicit permission.
    pub fn memory_allowed_for(&self, process_name: &str) -> bool {
        self.memory.allow_steam_client || !process_name.eq_ignore_ascii_case(STEAM_CLIENT)
    }

    /// Key bound to an action, if it has one.
    pub fn hotkey(&self, action: &str) -> Option<&str> {
        self.hotkeys.get(action).map(String::as_str).filter(|key| !key.trim().is_empty())
    }

    fn validate(mut self) -> Result<Self, String> {
        if self.process_names.is_empty() {
            return Err(lang::tr("process_names cannot be empty", "process_names ne peut pas être vide").into());
        }
        if !(0.05..=10.0).contains(&self.trainer.timescale) {
            return Err(lang::tr("trainer.timescale must be between 0.05 and 10", "trainer.timescale doit être compris entre 0.05 et 10").into());
        }
        if !(30..=1000).contains(&self.fps.limit) {
            return Err(lang::tr("fps.limit must be between 30 and 1000", "fps.limit doit être compris entre 30 et 1000").into());
        }
        if !(0..=10_000_000).contains(&self.trainer.money) {
            return Err(lang::tr("trainer.money must be between 0 and 10000000", "trainer.money doit être compris entre 0 et 10000000").into());
        }
        let known = default_hotkeys();
        for (action, key) in &self.hotkeys {
            if !known.contains_key(action) {
                return Err(lang::trs(format!("hotkeys.{action}: unknown action"), format!("hotkeys.{action} : action inconnue")));
            }
            if !key.trim().is_empty() && key_code(key).is_none() {
                return Err(lang::trs(format!("hotkeys.{action}: unknown key \"{key}\""), format!("hotkeys.{action} : touche « {key} » inconnue")));
            }
        }
        // A file that only redefines a few shortcuts keeps the others at their defaults.
        for (action, key) in known {
            self.hotkeys.entry(action).or_insert(key);
        }
        Ok(self)
    }
}

/// Returns the configuration and the path it was read from (`None` for the default values).
pub fn load(explicit: Option<&Path>) -> Result<(Config, Option<PathBuf>), String> {
    let path = match explicit {
        Some(p) => Some(p.to_path_buf()),
        None => default_locations().into_iter().find(|p| p.is_file()),
    };
    let Some(path) = path else {
        return Ok((Config::default(), None));
    };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let config = parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((config, Some(path)))
}

fn parse(text: &str) -> Result<Config, String> {
    toml::from_str::<Config>(text).map_err(|e| e.to_string())?.validate()
}

fn default_locations() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // The executable's folder, then its two parents: from `target/release`, this reaches the
        // project root. Started by Steam, the current folder is the game's, not ours.
        out.extend(exe.ancestors().skip(1).take(3).map(|dir| dir.join(FILE_NAME)));
    }
    out.push(PathBuf::from(FILE_NAME));
    out
}

/// Where to create the configuration file when none exists: next to the executable.
pub fn default_path() -> PathBuf {
    std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(FILE_NAME))).unwrap_or_else(|| PathBuf::from(FILE_NAME))
}

/// Value to save into the configuration file.
pub enum Setting<'a> {
    Bool(bool),
    Int(i64),
    Text(&'a str),
}

/// Changes one value in the configuration file while keeping its comments.
pub fn save(path: &Path, table: &str, key: &str, value: Setting) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let updated = set_in_document(&text, table, key, value)?;
    // Read back before writing: a valid file is never replaced by an invalid one.
    parse(&updated)?;
    std::fs::write(path, updated).map_err(|e| format!("{}: {e}", path.display()))
}

fn set_in_document(text: &str, table: &str, key: &str, value: Setting) -> Result<String, String> {
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|e| e.to_string())?;
    let item = match value {
        Setting::Bool(v) => toml_edit::value(v),
        Setting::Int(v) => toml_edit::value(v),
        Setting::Text(v) => toml_edit::value(v),
    };
    let entry = doc.entry(table).or_insert(toml_edit::table());
    let section = entry.as_table_mut().ok_or_else(|| lang::trs(format!("[{table}] is not a section"), format!("[{table}] n'est pas une section")))?;
    // Replacing only the value keeps the comment above the key.
    match section.get_mut(key).and_then(|existing| existing.as_value_mut()) {
        Some(existing) => {
            let decor = existing.decor().clone();
            *existing = item.into_value().map_err(|_| lang::tr("invalid value", "valeur invalide").to_string())?;
            *existing.decor_mut() = decor;
        }
        None => {
            section.insert(key, item);
        }
    }
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_safe_defaults() {
        let c = parse("").unwrap();
        assert!(c.cpu.enabled && !c.cpu.smt);
        assert!(!c.wants_memory());
        assert!(!c.memory.allow_steam_client);
        assert_eq!(c.hotkey("god_mode"), Some("F1"));
    }

    #[test]
    fn shipped_config_parses() {
        let c = parse(include_str!("../iw-helper.toml")).unwrap();
        assert_eq!(c.cpu.priority, Priority::High);
        assert_eq!(c.trainer.money, 100_000);
        assert!(c.trainer.sound);
    }

    #[test]
    fn steam_client_needs_explicit_opt_in() {
        let c = parse("[trainer]\nenabled = true").unwrap();
        assert!(!c.memory_allowed_for("IW7_ship.exe"));
        assert!(c.memory_allowed_for("iw7-mod.exe"));
        let c = parse("[memory]\nallow_steam_client = true").unwrap();
        assert!(c.memory_allowed_for("iw7_ship.exe"));
    }

    #[test]
    fn typos_and_bad_values_are_rejected() {
        assert!(parse("[cpu]\nsmtt = true").is_err());
        assert!(parse("[trainer]\ntimescale = 0").is_err());
        assert!(parse("[trainer]\nmoney = -1").is_err());
        assert!(parse("[fps]\nlimit = 5").is_err());
        assert!(parse("[cpu]\npriority = \"realtime\"").is_err());
        assert!(parse("[hotkeys]\ngod_mode = \"F99\"").is_err());
        assert!(parse("[hotkeys]\nfly = \"F1\"").is_err());
    }

    #[test]
    fn hotkeys_can_be_changed_or_removed() {
        let c = parse("[hotkeys]\ngod_mode = \"Insert\"\nnoclip = \"\"").unwrap();
        assert_eq!(c.hotkey("god_mode"), Some("Insert"));
        assert_eq!(c.hotkey("noclip"), None);
        assert_eq!(c.hotkey("teleport"), Some("PageDown"));
    }

    #[test]
    fn key_names_map_to_virtual_keys() {
        assert_eq!(key_code("F1"), Some(0x70));
        assert_eq!(key_code("f10"), Some(0x79));
        assert_eq!(key_code("Num0"), Some(0x60));
        assert_eq!(key_code("PageUp"), Some(0x21));
        assert_eq!(key_code("Fin"), Some(0x23));
        assert_eq!(key_code("K"), Some(0x4B));
        assert_eq!(key_code("F25"), None);
        assert_eq!(key_code("Espace"), None);
    }

    #[test]
    fn saving_keeps_comments_and_other_values() {
        let text = "# header\n[trainer]\nenabled = true\n# beep\nsound = true # end of line\n\n[fps]\nlimit = 240\n";
        let updated = set_in_document(text, "trainer", "sound", Setting::Bool(false)).unwrap();
        assert!(updated.contains("# header") && updated.contains("# beep"));
        assert!(updated.contains("sound = false"));
        assert!(updated.contains("limit = 240"));
        let updated = set_in_document(&updated, "cpu", "max_cores", Setting::Int(6)).unwrap();
        let c = parse(&updated).unwrap();
        assert!(!c.trainer.sound && c.trainer.enabled);
        assert_eq!(c.cpu.max_cores, 6);
        let updated = set_in_document(&updated, "cpu", "priority", Setting::Text("normal")).unwrap();
        assert_eq!(parse(&updated).unwrap().cpu.priority, Priority::Normal);
    }
}
