//! Full survey: everything the tool can read from the game, written into a separate text file.
//!
//! It is a research tool. It shows at once what is readable (profile, engine settings, script
//! variables, suits, player state), and above all it compares: take a survey, change ONE thing in
//! the game, take another, and the second starts with what changed. Its text is written in French.
//!
//! Nothing is written into the game: the survey goes through a read-only handle to the process.
//! Each section stands alone: if it cannot be read, it says so and the survey goes on.
//! The file holds account data; it goes into the log folder and nowhere else.

use crate::process::Process;
use crate::trainer::{
    self, AGENT_SLOTS, ASSET_POOLS, ASSET_POOL_SIZES, ASSET_TYPE_SUIT, BASE, CHILD_PARTITION, CHILD_SIZE, CHILD_VARIABLES, CLIENT_FLAGS, ENTITY_CLIENT, ENTITY_FLAGS,
    DVAR_COUNT, DVAR_POOL, ENTITY_HEALTH, ENTITY_SIZE, FIRST_AGENT, FPS_DVAR_PTR, G_ENTITIES, HOST_CLIENT, IMAGE_END, OBJECT_COUNT, SCR_VAR_GLOB, SERVER_CLIENT_MONEY, SUIT_SIZE, SVS_CLIENTS, TIMESCALE,
};
use crate::{logbook, profile};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Limits: the file must stay readable in a text editor.
const MAX_CHANGES: usize = 500;
const MAX_DVARS: usize = 12_000;
const MAX_FIELDS: usize = 6_000;
const MAX_SUITS: usize = 64;
const MAX_OBJECTS: usize = 8;
const MAX_OBJECT_FIELDS: usize = 400;

// --- Engine settings (`dvar_t`, 96 bytes, from iw7-mod) -------------------------------------
const DVAR_SIZE: usize = 96;

// --- General engine state (iw7-mod addresses, read as they are) --------------------------------
const ENGINE_INTEGERS: [(&str, usize); 8] = [
    ("Scène du menu (s_frontEndScene_state)", BASE + 0x4BF_F608),
    ("Emplacements de joueurs du serveur (svs_numclients)", BASE + 0x6B2_29E0),
    ("Objet « level » des scripts (levelEntityId)", BASE + 0x665_A120),
    ("Objet « game » des scripts (gameEntityId)", BASE + 0x665_A124),
    ("Redémarrage de carte demandé (sv_map_restart)", BASE + 0x6B2_C9D4),
    ("Chargement des scripts demandé (sv_loadScripts)", BASE + 0x6B2_C9D8),
    ("Migration d'hôte (sv_migrate)", BASE + 0x6B2_C9DC),
    ("Saisie captée par un menu (keyCatchers)", BASE + 0x224_6C34),
];
/// Clocks: they move all the time, so they are not compared from one survey to the next.
const ENGINE_CLOCKS: [(&str, usize); 2] = [("Horloge de la partie (gameTime)", BASE + 0x3C9_86D8), ("Horloge du moteur (com_frameTime)", BASE + 0x600_53C0)];
/// Lobby party and server session: their layout is not known, they are dumped raw.
const PARTY_DATA: usize = BASE + 0x525_4DA0;
const SERVER_SESSION: usize = BASE + 0x6B4_E080;
/// Number of zombies in play: single-source address, never checked.
const ZOMBIE_COUNT: usize = BASE + 0x3BF_A168;

/// Type of a script value that points to an entity (`VAR_ENTITY` from iw7-mod).
const VAR_ENTITY: u8 = 0x15;

/// One part of the survey: notes, then pairs (what is read, its value).
struct Section {
    title: &'static str,
    notes: Vec<String>,
    entries: Vec<(String, String)>,
    /// False for what moves all the time (clocks) or never moves (code): nothing to compare.
    compared: bool,
}

impl Section {
    fn new(title: &'static str) -> Self {
        Self { title, notes: Vec::new(), entries: Vec::new(), compared: true }
    }

    fn note(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
    }

    fn entry(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.push((key.into(), value.into()));
    }

    /// 16-byte rows from `addr`. Rows that are all zero are left out.
    fn hex_rows(&mut self, p: &Process, what: &str, addr: usize, len: usize) {
        if p.read_u8(addr).is_err() {
            self.note(format!("{what} : illisible en {addr:#x}."));
            return;
        }
        let data = p.read_lossy(addr, len);
        let mut empty = 0;
        for (row, bytes) in data.chunks(16).enumerate() {
            if bytes.iter().all(|&byte| byte == 0) {
                empty += 1;
            } else {
                self.entry(format!("{:#011x}", addr + row * 16), trainer::hex(bytes));
            }
        }
        self.note(format!("{what} : {len:#x} octets à partir de {addr:#x}, {empty} lignes à zéro omises."));
    }
}

/// Everything read at a given moment.
pub struct Survey {
    pub in_match: bool,
    sections: Vec<Section>,
}

/// Reads everything readable. `p` only needs read access, and nothing is written.
pub fn take(p: &Process) -> Survey {
    let in_match = trainer::in_match(p);
    let mut sections = vec![profile_section(p), lobby_section(p), clocks_section(p), dvars_section(p), suits_section(p), asset_names_section(p)];
    if in_match {
        sections.extend([level_section(p), player_objects_section(p), player_section(p), zombies_section(p)]);
    }
    sections.push(control_section(p));
    sections.push(field_names_section(p));
    Survey { in_match, sections }
}

/// Writes the survey into `dir` and returns the file name. `previous` is the previous survey taken
/// in the same state (menus or match): the file then starts with what changed since.
pub fn write(dir: &Path, survey: &Survey, previous: Option<&Survey>, game: &str, pid: u32, recognised: bool) -> Result<String, String> {
    let (stamp, date) = logbook::timestamps();
    let header = [
        format!("Date : {date}"),
        format!("État : {}", survey.state()),
        format!("Version du jeu : {}", if recognised { "reconnue" } else { "NON reconnue (les adresses ne veulent alors rien dire)" }),
        format!("Processus : {game} (pid {pid})"),
    ];
    let text = survey.render(&header, previous);
    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    // Two surveys in the same second do not overwrite each other.
    let name = (1..100)
        .map(|attempt| if attempt == 1 { format!("releve-{stamp}.txt") } else { format!("releve-{stamp}-{attempt}.txt") })
        .find(|name| !dir.join(name).exists())
        .ok_or("trop de relevés dans la même seconde")?;
    std::fs::write(dir.join(&name), text).map_err(|e| format!("{name} : {e}"))?;
    Ok(name)
}

impl Survey {
    fn state(&self) -> &'static str {
        if self.in_match {
            "en partie"
        } else {
            "au menu (hors partie)"
        }
    }

    fn render(&self, header: &[String], previous: Option<&Survey>) -> String {
        let mut out = vec![
            "RELEVÉ COMPLET".to_string(),
            "Lecture seule : le jeu a été ouvert sans droit d'écriture, rien n'y a été modifié.".to_string(),
            "Ce fichier contient des données du compte : il reste dans le dossier des journaux.".to_string(),
        ];
        out.extend(header.iter().cloned());
        out.push(format!("Sections : {}", self.sections.iter().map(|section| section.title).collect::<Vec<_>>().join(" ; ")));
        out.push(String::new());
        out.push(format!("== Ce qui a changé depuis le relevé précédent ({}) ==", self.state()));
        match previous {
            None => out.push("  Premier relevé de cette session dans cet état : rien à comparer. Changez une chose dans le jeu, puis refaites un relevé.".to_string()),
            Some(previous) => {
                let changes = self.changes(previous);
                out.push(format!("  {} différence(s). Les horloges et le code ne sont pas comparés ; dans un vidage, « absent » veut dire ligne à zéro.", changes.len()));
                out.extend(changes.iter().take(MAX_CHANGES).map(|line| format!("  {line}")));
                if changes.len() > MAX_CHANGES {
                    out.push(format!("  … {} autres différences non listées.", changes.len() - MAX_CHANGES));
                }
            }
        }
        for section in &self.sections {
            out.push(String::new());
            out.push(format!("== {} ==", section.title));
            out.extend(section.notes.iter().map(|note| format!("  {note}")));
            out.extend(section.entries.iter().map(|(key, value)| format!("  {key:<58} {value}")));
        }
        out.push(String::new());
        out.join("\n")
    }

    /// What differs from `previous`: changed, new and missing values.
    fn changes(&self, previous: &Survey) -> Vec<String> {
        let before: HashMap<(&str, &str), &str> = previous.compared().into_iter().map(|(title, key, value)| ((title, key), value)).collect();
        let mut seen = HashSet::new();
        let mut lines = Vec::new();
        for (title, key, value) in self.compared() {
            seen.insert((title, key));
            match before.get(&(title, key)) {
                Some(&old) if old != value => lines.push(format!("[{title}] {key} : {old}  ->  {value}")),
                None => lines.push(format!("[{title}] {key} : (absent)  ->  {value}")),
                _ => {}
            }
        }
        for (title, key, old) in previous.compared() {
            if !seen.contains(&(title, key)) {
                lines.push(format!("[{title}] {key} : {old}  ->  (absent)"));
            }
        }
        lines
    }

    /// Everything compared from one survey to the next: (section, what is read, its value).
    fn compared(&self) -> Vec<(&'static str, &str, &str)> {
        self.sections.iter().filter(|section| section.compared).flat_map(|section| section.entries.iter().map(|(key, value)| (section.title, key.as_str(), value.as_str()))).collect()
    }
}

fn int(raw: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(raw[at..at + 4].try_into().unwrap())
}

fn float(raw: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(raw[at..at + 4].try_into().unwrap())
}

/// A readable float: tiny or huge values (often integers read as floats) in scientific notation.
fn show_float(value: f32) -> String {
    if value == 0.0 || (1.0e-4..1.0e9).contains(&value.abs()) {
        value.to_string()
    } else {
        format!("{value:e}")
    }
}

fn read_int(p: &Process, addr: usize) -> String {
    p.read_i32(addr).map_or_else(|_| "illisible".to_string(), |value| format!("{value} ({:#x})", value as u32))
}

fn in_image(addr: usize) -> bool {
    (BASE..IMAGE_END).contains(&addr)
}

// --- Profile ---------------------------------------------------------------------------------
fn profile_section(p: &Process) -> Section {
    let mut section = Section::new("Profil Zombies (progression du compte)");
    section.note("Cases connues d'abord, puis toute la zone en hexadécimal. Les cases marquées « source unique » n'ont jamais été vérifiées.");
    for (name, addr, size) in profile::known_cells() {
        let mut raw = vec![0u8; size];
        let value = match (p.read_bytes(addr, &mut raw), size) {
            (Err(_), _) => "illisible".to_string(),
            (Ok(()), 4) => format!("{} ({:#x})", int(&raw, 0), int(&raw, 0) as u32),
            (Ok(()), 1) => format!("{} ({:#04x})", raw[0], raw[0]),
            (Ok(()), _) => trainer::hex(&raw),
        };
        section.entry(format!("{name} [{addr:#011x}]"), value);
    }
    section.hex_rows(p, "Zone du profil", profile::BLOCK.start, profile::BLOCK.len());
    section
}

// --- Lobby and general state -----------------------------------------------------------------
fn lobby_section(p: &Process) -> Section {
    let mut section = Section::new("Salon et état général du moteur");
    section.note("Ce que l'outil sait lire hors partie. La disposition du groupe de joueurs (g_partyData) et de la session du serveur n'est pas connue : leur début est vidé brut.");
    for (name, addr) in ENGINE_INTEGERS {
        section.entry(format!("{name} [{addr:#011x}]"), read_int(p, addr));
    }
    section.entry(format!("Vitesse du jeu [{TIMESCALE:#011x}]"), p.read_f32(TIMESCALE).map_or_else(|_| "illisible".to_string(), show_float));
    section.entry(format!("Tableau des joueurs du serveur (svs_clients) [{SVS_CLIENTS:#011x}]"), p.read_usize(SVS_CLIENTS).map_or_else(|_| "illisible".to_string(), |ptr| format!("{ptr:#x}")));
    section.entry(format!("Nombre de zombies (source unique, jamais vérifié) [{ZOMBIE_COUNT:#011x}]"), read_int(p, ZOMBIE_COUNT));
    section.hex_rows(p, "Groupe de joueurs du salon (g_partyData)", PARTY_DATA, 0x400);
    section.hex_rows(p, "Session du serveur (g_serverSession)", SERVER_SESSION, 0x100);
    section
}

fn clocks_section(p: &Process) -> Section {
    let mut section = Section::new("Horloges (non comparées)");
    section.compared = false;
    for (name, addr) in ENGINE_CLOCKS {
        section.entry(format!("{name} [{addr:#011x}]"), read_int(p, addr));
    }
    section
}

// --- Engine settings ---------------------------------------------------------------------------
fn dvar_type(kind: u8) -> &'static str {
    match kind {
        0 => "booléen",
        1 => "flottant",
        2 => "2 flottants",
        3 => "3 flottants",
        4 => "4 flottants",
        5 => "entier",
        6 => "choix",
        7 => "chaîne",
        8 => "couleur",
        9 => "couleur (3 flottants)",
        _ => "type inconnu",
    }
}

/// Value of a setting, read from one of its 16-byte slots (current, latched, reset).
fn dvar_value(p: &Process, kind: u8, raw: &[u8]) -> String {
    let floats = |count: usize| (0..count).map(|index| show_float(float(raw, index * 4))).collect::<Vec<_>>().join(" ");
    match kind {
        0 => (if raw[0] != 0 { "vrai" } else { "faux" }).to_string(),
        1 => floats(1),
        2 => floats(2),
        3 | 9 => floats(3),
        4 => floats(4),
        // The engine stores a hash next to each integer; a wrong hash is reported.
        5 | 6 if int(raw, 4) == trainer::hash_integer(int(raw, 0)) => int(raw, 0).to_string(),
        5 | 6 => format!("{} (empreinte non conforme : {:#010x})", int(raw, 0), int(raw, 4) as u32),
        7 => {
            let text = usize::from_le_bytes(raw[..8].try_into().unwrap());
            p.read_c_string(text, 96).map_or_else(|| if text == 0 { "(vide)".to_string() } else { format!("(chaîne illisible en {text:#x})") }, |text| format!("« {text} »"))
        }
        8 => trainer::hex(&raw[..4]),
        _ => trainer::hex(raw),
    }
}

fn dvar_domain(kind: u8, raw: &[u8]) -> String {
    match kind {
        1..=4 | 9 => format!("{} à {}", show_float(float(raw, 0)), show_float(float(raw, 4))),
        5 => format!("{} à {}", int(raw, 0), int(raw, 4)),
        6 => format!("{} choix", int(raw, 0)),
        _ => "sans bornes".to_string(),
    }
}

fn dvars_section(p: &Process) -> Section {
    let mut section = Section::new("Réglages du moteur (dvars)");
    section.note("Un réglage n'a pas de nom en mémoire, seulement l'empreinte de son nom. Colonnes : type, valeur actuelle, valeur d'origine, bornes, drapeaux.");
    let count = match p.read_i32(DVAR_COUNT) {
        Ok(count) if (1..=100_000).contains(&count) => count as usize,
        Ok(count) => {
            section.note(format!("Table illisible : le nombre de réglages lu en {DVAR_COUNT:#x} ({count}) n'est pas plausible. Les réglages ne sont enregistrés qu'une fois un mode de jeu chargé."));
            return section;
        }
        Err(_) => {
            section.note(format!("Table illisible : nombre de réglages illisible en {DVAR_COUNT:#x}."));
            return section;
        }
    };
    let listed = count.min(MAX_DVARS);
    // Two possible layouts: settings stored back to back (what the surveys show: the FPS limit
    // falls on an exact multiple of 96 bytes), or an array of pointers (what iw7-mod assumes).
    // The table says which itself: sixteen pointers into the game image, or not.
    let mut head = [0u8; 16 * 8];
    let pointers = p.read_bytes(DVAR_POOL, &mut head).is_ok() && head.chunks(8).all(|raw| in_image(usize::from_le_bytes(raw.try_into().unwrap())));
    let dvars: Vec<(usize, Vec<u8>)> = if pointers {
        section.note(format!("{count} réglages ; la table en {DVAR_POOL:#x} est un tableau de pointeurs."));
        let table = p.read_lossy(DVAR_POOL, listed * 8);
        table.chunks(8).map(|raw| usize::from_le_bytes(raw.try_into().unwrap())).filter(|&addr| in_image(addr)).map(|addr| (addr, p.read_lossy(addr, DVAR_SIZE))).collect()
    } else {
        section.note(format!("{count} réglages, rangés bout à bout à partir de {DVAR_POOL:#x} (96 octets chacun)."));
        let pool = p.read_lossy(DVAR_POOL, listed * DVAR_SIZE);
        pool.chunks(DVAR_SIZE).enumerate().map(|(index, raw)| (DVAR_POOL + index * DVAR_SIZE, raw.to_vec())).collect()
    };
    if count > listed {
        section.note(format!("Seuls les {listed} premiers sont listés."));
    }
    let fps = p.read_usize(FPS_DVAR_PTR).unwrap_or(0);
    for (index, (addr, raw)) in dvars.iter().enumerate() {
        if raw.iter().all(|&byte| byte == 0) {
            continue;
        }
        let (checksum, flags, kind) = (int(raw, 0) as u32, int(raw, 4) as u32, raw[9]);
        let known = if *addr == fps { "  <- limite d'images" } else { "" };
        section.entry(
            format!("n° {index:<5} empreinte {checksum:#010x} [{addr:#011x}]"),
            format!(
                "{} ; actuelle {} ; origine {} ; bornes {} ; drapeaux {flags:#x}{known}",
                dvar_type(kind),
                dvar_value(p, kind, &raw[16..32]),
                dvar_value(p, kind, &raw[48..64]),
                dvar_domain(kind, &raw[64..80])
            ),
        );
    }
    section
}

// --- Suits -------------------------------------------------------------------------------------
fn suits_section(p: &Process) -> Section {
    let mut section = Section::new("Tenues chargées (toutes les valeurs)");
    let pool = p.read_usize(ASSET_POOLS + ASSET_TYPE_SUIT * 8).unwrap_or(0);
    let count = p.read_i32(ASSET_POOL_SIZES + ASSET_TYPE_SUIT * 4).unwrap_or(0);
    if !in_image(pool) || !(1..=256).contains(&count) {
        section.note(format!("Réserve des tenues illisible (adresse {pool:#x}, {count} emplacements)."));
        return section;
    }
    section.note(format!("Réserve en {pool:#x}, {count} emplacements de {SUIT_SIZE:#x} octets. Chaque mot de 4 octets non nul est donné en entier et en flottant ; le nom des champs est dans iw7-mod (SuitDef)."));
    let pool_end = pool + 8 + count as usize * SUIT_SIZE;
    let mut listed = 0;
    for index in 0..count as usize {
        let suit = pool + 8 + index * SUIT_SIZE;
        // A free slot starts with a pointer to the next free slot, inside the pool itself.
        let name = p.read_usize(suit).ok().filter(|name| !(pool..pool_end).contains(name)).and_then(|name| p.read_c_string(name, 64));
        let Some(name) = name else { continue };
        if listed == MAX_SUITS {
            section.note(format!("Seules les {MAX_SUITS} premières tenues sont listées."));
            break;
        }
        listed += 1;
        section.entry(name.clone(), format!("emplacement {index}, en {suit:#x}"));
        let raw = p.read_lossy(suit, SUIT_SIZE);
        for offset in (8..SUIT_SIZE).step_by(4).filter(|&offset| int(&raw, offset) != 0) {
            section.entry(format!("{name} +{offset:#05x}"), format!("entier {:<12} flottant {}", int(&raw, offset), show_float(float(&raw, offset))));
        }
    }
    if listed == 0 {
        section.note("Aucune tenue chargée.");
    }
    section
}

// --- The game's asset pools, by name ---------------------------------------------------------
/// Pool types walked, and the most entries listed: a bigger pool (images, sounds…) is only
/// summarised. Captured on 2026-10-14: type 38 = weapon attachments ("defaultattachment" first,
/// "g18cpap1"…), type 39 = weapons ("defaultweapon" first).
const ASSET_TYPES: usize = 100;
const MAX_POOL_NAMES: usize = 1_500;
const MAX_ASSET_NAMES: usize = 20_000;
const POOL_SAMPLE: usize = 0x8000;
const MAX_FREE_STEPS: usize = 4_096;
const WEAPON_POOL_FIRST: &str = "defaultweapon";
const MAX_ENTRY_BYTES: usize = 0x2000;
const POINTED_BYTES: usize = 0x400;
const MAX_POINTED: usize = 64;
/// Tables of pointers to weapons, searched in the game data: minimum length of a run, and number
/// of runs kept.
const MIN_TABLE_RUN: usize = 16;
const MAX_TABLE_RUNS: usize = 6;
/// Record of the held weapon, captured on 2026-10-14 (its number in the first two bytes).
const HELD_WEAPON: usize = HOST_CLIENT + 0x4AB0;

/// An address that can point to game memory: the names of assets loaded with a map are stored
/// outside the program image.
fn plausible_pointer(addr: usize) -> bool {
    (0x1_0000..0x7FFF_0000_0000).contains(&addr)
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Step between two entries of a pool, deduced from pointers to a name: the most frequent gap
/// between two consecutive name positions. Only used if the free slot list says nothing.
fn pool_stride(name_offsets: &[usize]) -> Option<(usize, usize)> {
    let mut gaps: HashMap<usize, usize> = HashMap::new();
    for pair in name_offsets.windows(2) {
        *gaps.entry(pair[1] - pair[0]).or_default() += 1;
    }
    let (&stride, &seen) = gaps.iter().max_by_key(|&(&gap, &seen)| (seen, std::cmp::Reverse(gap)))?;
    (seen >= 2).then(|| (stride, name_offsets[0] % stride))
}

/// Step of a pool from its free slots: the pool starts with a pointer to the first free slot, and
/// each free slot points to the next. Their offsets from the first entry are all multiples of the
/// step; their greatest common divisor gives it, if there are enough free slots.
fn free_list_stride(p: &Process, base: usize, count: usize) -> Option<usize> {
    let first = base + 8;
    let mut addr = p.read_usize(base).ok()?;
    let (mut stride, mut seen) = (0, 0);
    for _ in 0..MAX_FREE_STEPS.min(count) {
        let Some(offset) = addr.checked_sub(first).filter(|&offset| offset > 0 && offset % 8 == 0 && offset < count * 0x1_0000) else { break };
        stride = gcd(stride, offset);
        seen += 1;
        addr = p.read_usize(addr).unwrap_or(0);
    }
    (seen >= 3 && stride >= 8).then_some(stride)
}

/// A pool whose step was found.
struct Pool {
    kind: usize,
    base: usize,
    count: usize,
    stride: usize,
}

impl Pool {
    fn entry(&self, index: usize) -> usize {
        self.base + 8 + index * self.stride
    }

    fn end(&self) -> usize {
        self.entry(self.count)
    }

    /// String pointed to by the word at `addr`, if it does not point into the pool itself.
    fn text_at(&self, p: &Process, addr: usize) -> Option<String> {
        let target = p.read_usize(addr).ok()?;
        (plausible_pointer(target) && !(self.base..self.end()).contains(&target)).then(|| p.read_c_string(target, 96)).flatten().filter(|name| name.len() >= 2)
    }

    /// Name of entry `index` (its first word), or `None` for a free or nameless slot.
    fn name(&self, p: &Process, index: usize) -> Option<String> {
        self.text_at(p, self.entry(index))
    }

    /// Number of the entry that starts at `addr`, if it is in this pool.
    fn index_of(&self, addr: usize) -> Option<usize> {
        let offset = addr.checked_sub(self.base + 8)?;
        (offset % self.stride == 0 && offset / self.stride < self.count).then_some(offset / self.stride)
    }
}

/// The pool of type `kind`. Empty error if it does not exist, otherwise what is missing.
fn find_pool(p: &Process, kind: usize) -> Result<Pool, String> {
    let base = p.read_usize(ASSET_POOLS + kind * 8).unwrap_or(0);
    let count = p.read_i32(ASSET_POOL_SIZES + kind * 4).unwrap_or(0);
    if !in_image(base) || !(1..=1_000_000).contains(&count) {
        return Err(String::new());
    }
    let count = count as usize;
    if let Some(stride) = free_list_stride(p, base, count) {
        return Ok(Pool { kind, base, count, stride });
    }
    let sample = p.read_lossy(base + 8, POOL_SAMPLE);
    let name_at = |offset: usize| {
        let target = usize::from_le_bytes(sample[offset..offset + 8].try_into().unwrap());
        plausible_pointer(target) && !(base..base + 8 + POOL_SAMPLE).contains(&target) && p.read_c_string(target, 96).is_some_and(|name| name.len() >= 2)
    };
    let offsets: Vec<usize> = (0..POOL_SAMPLE).step_by(8).filter(|&offset| name_at(offset)).take(64).collect();
    let (stride, _) = pool_stride(&offsets).ok_or_else(|| format!("{base:#x}, {count} emplacements, pas introuvable"))?;
    Ok(Pool { kind, base, count, stride })
}

/// Name of the entry `addr` points to in one of the pools, with its type.
fn asset_name(p: &Process, pools: &[Pool], addr: usize) -> Option<String> {
    pools.iter().find_map(|pool| pool.index_of(addr).and_then(|index| pool.name(p, index)).map(|name| format!("type {} « {name} »", pool.kind)))
}

/// What a pointer points to: a named entry, a string, or a list of named entries.
fn describe_pointer(p: &Process, pools: &[Pool], target: usize) -> Option<String> {
    if let Some(name) = asset_name(p, pools, target) {
        return Some(name);
    }
    if let Some(text) = p.read_c_string(target, 96).filter(|text| text.len() >= 2) {
        return Some(format!("chaîne « {text} »"));
    }
    let list = p.read_lossy(target, MAX_POINTED * 8);
    let names: Vec<String> = list.chunks(8).map_while(|raw| asset_name(p, pools, usize::from_le_bytes(raw.try_into().unwrap()))).collect();
    (!names.is_empty()).then(|| format!("liste de {} : {}", names.len(), names.join(", ")))
}

/// Details an entry: its bytes, what its pointers point to, and one level further for pointers to
/// nameless blocks (attachment lists may be stored there).
fn describe_entry(p: &Process, section: &mut Section, pools: &[Pool], label: &str, entry: usize, len: usize) {
    let raw = p.read_lossy(entry, len);
    for (row, bytes) in raw.chunks(32).enumerate() {
        section.entry(format!("  {label} +{:#05x}", row * 32), trainer::hex(bytes));
    }
    for offset in (0..len.saturating_sub(7)).step_by(8) {
        let target = usize::from_le_bytes(raw[offset..offset + 8].try_into().unwrap());
        if !plausible_pointer(target) || (entry..entry + len).contains(&target) {
            continue;
        }
        if let Some(what) = describe_pointer(p, pools, target) {
            section.entry(format!("  {label} champ +{offset:#05x}"), format!("{target:#x} -> {what}"));
            continue;
        }
        let block = p.read_lossy(target, POINTED_BYTES);
        for inner in (0..POINTED_BYTES).step_by(8) {
            let pointed = usize::from_le_bytes(block[inner..inner + 8].try_into().unwrap());
            if plausible_pointer(pointed) {
                if let Some(what) = describe_pointer(p, pools, pointed).filter(|what| !what.starts_with("chaîne")) {
                    section.entry(format!("  {label} champ +{offset:#05x} -> +{inner:#05x}"), format!("{pointed:#x} -> {what}"));
                }
            }
        }
    }
}

/// Attachment lists of a weapon definition, captured on 2026-10-15: six 4-byte counts at +0x28,
/// then six pointers at +0x48 (an empty list keeps an unrelated pointer). The order of the lists
/// changes from one weapon to another: "g18pap1/2" in the 3rd for iw7_g18_zmr, "sonicpap1/2" in the 1st
/// for iw7_sonic_zmr.
const ATTACHMENT_LISTS: usize = 6;
const ATTACHMENT_COUNTS: usize = 0x28;
const ATTACHMENT_POINTERS: usize = 0x48;
const ATTACHMENT_BYTES: usize = 0x100;
const MAX_LISTED_ATTACHMENTS: usize = 16;

/// Each attachment list of the weapon, and the start of each attachment: enough to find the field
/// that decides its place in the held weapon's record.
fn describe_attachments(p: &Process, section: &mut Section, pools: &[Pool], label: &str, def: usize) {
    for list in 0..ATTACHMENT_LISTS {
        let count = p.read_i32(def + ATTACHMENT_COUNTS + list * 4).unwrap_or(0);
        let start = p.read_usize(def + ATTACHMENT_POINTERS + list * 8).unwrap_or(0);
        section.entry(format!("  {label} liste {list}"), format!("{count} accessoires en {start:#x}"));
        if !(1..=256).contains(&count) || !plausible_pointer(start) {
            continue;
        }
        for index in 0..(count as usize).min(MAX_LISTED_ATTACHMENTS) {
            let Ok(attachment) = p.read_usize(start + index * 8) else { continue };
            let name = asset_name(p, pools, attachment).unwrap_or_else(|| format!("{attachment:#x} sans nom"));
            section.entry(format!("  {label} liste {list} n° {index}"), name);
            for (row, bytes) in p.read_lossy(attachment, ATTACHMENT_BYTES).chunks(32).enumerate() {
                section.entry(format!("  {label} liste {list} n° {index} +{:#05x}", row * 32), trainer::hex(bytes));
            }
        }
    }
}

/// Runs of at least `MIN_TABLE_RUN` consecutive pointers to entries of `pool`, in the game data: maybe
/// a table ordered by weapon number. Returns the start and length of each.
fn pointer_tables(p: &Process, pool: &Pool) -> Vec<(usize, usize)> {
    let (start, end) = (trainer::CODE_RANGE.end, IMAGE_END);
    let mut runs = Vec::new();
    let mut run: Option<(usize, usize)> = None;
    for chunk_start in (start..end).step_by(0x40_0000) {
        let chunk = p.read_lossy(chunk_start, 0x40_0000.min(end - chunk_start));
        for (index, raw) in chunk.as_chunks::<8>().0.iter().enumerate() {
            let at = chunk_start + index * 8;
            if pool.index_of(usize::from_le_bytes(*raw)).is_some() {
                run = Some(run.map_or((at, 1), |(first, len)| (first, len + 1)));
                continue;
            }
            if let Some((first, len)) = run.take().filter(|&(_, len)| len >= MIN_TABLE_RUN) {
                runs.push((first, len));
                if runs.len() == MAX_TABLE_RUNS {
                    return runs;
                }
            }
        }
    }
    runs
}

/// Lists the loaded entries of each pool by name (weapons, attachments, suits…), then details the
/// held weapon. Not compared: it does not change during play.
fn asset_names_section(p: &Process) -> Section {
    let mut section = Section::new("Éléments chargés, par réserve (non comparée)");
    section.compared = false;
    section.note(format!("Pour chaque type : adresse, nombre d'emplacements, pas (déduit des emplacements libres, sinon des noms). Les emplacements occupés sont listés jusqu'à {MAX_POOL_NAMES} ; au-delà, seulement les premiers. Après le nom : les chaînes des deux mots suivants."));
    let mut pools = Vec::new();
    for kind in 0..ASSET_TYPES {
        match find_pool(p, kind) {
            Ok(pool) => pools.push(pool),
            Err(reason) if !reason.is_empty() => section.entry(format!("type {kind}"), reason),
            Err(_) => {}
        }
    }
    let mut listed = 0;
    for pool in &pools {
        section.entry(format!("type {}", pool.kind), format!("{:#x}, {} emplacements de {:#x} octets", pool.base, pool.count, pool.stride));
        let shown = if pool.count <= MAX_POOL_NAMES { pool.count } else { 8 };
        for index in 0..shown {
            if listed >= MAX_ASSET_NAMES {
                break;
            }
            let Some(name) = pool.name(p, index) else { continue };
            let extra: Vec<String> = [8, 16].iter().filter(|&&offset| offset < pool.stride).filter_map(|&offset| pool.text_at(p, pool.entry(index) + offset)).collect();
            section.entry(format!("  type {} n° {index}", pool.kind), format!("{name}{}   [{:#x}]", extra.iter().map(|text| format!(" / {text}")).collect::<String>(), pool.entry(index)));
            listed += 1;
        }
    }
    if listed >= MAX_ASSET_NAMES {
        section.note(format!("Arrêté à {MAX_ASSET_NAMES} noms."));
    }

    // The held weapon: its record, the entry with the same number in the weapon pool, and the tables
    // of pointers to weapons, in case the number goes through one of them.
    let Ok(held) = p.read_i32(HELD_WEAPON).map(|word| (word & 0xFFFF) as usize) else {
        section.note("Arme en main illisible.");
        return section;
    };
    section.entry("arme en main", format!("numéro {held} ({held:#x}), fiche {}", trainer::hex(&p.read_lossy(HELD_WEAPON, 20))));
    let Some(weapons) = pools.iter().find(|pool| pool.name(p, 0).as_deref() == Some(WEAPON_POOL_FIRST)) else {
        section.note("Réserve des armes introuvable (aucune ne commence par « defaultweapon »).");
        return section;
    };
    section.entry("réserve des armes", format!("type {}, pas {:#x}", weapons.kind, weapons.stride));
    let len = weapons.stride.min(MAX_ENTRY_BYTES);
    if held < weapons.count {
        let label = format!("arme n° {held} de la réserve ({})", weapons.name(p, held).unwrap_or_else(|| "sans nom".to_string()));
        section.entry(label.clone(), format!("{:#x}", weapons.entry(held)));
        describe_entry(p, &mut section, &pools, &label, weapons.entry(held), len);
    }
    for (table, run) in pointer_tables(p, weapons) {
        let pointed = if held < run { p.read_usize(table + held * 8).ok() } else { None };
        let name = pointed.and_then(|entry| weapons.index_of(entry)).and_then(|index| weapons.name(p, index));
        section.entry(format!("table de pointeurs vers les armes {table:#x}"), format!("{run} pointeurs ; case {held} : {}", name.clone().unwrap_or_else(|| "hors table ou sans nom".to_string())));
        if let (Some(entry), Some(name)) = (pointed, name) {
            let label = format!("arme « {name} » (table {table:#x})");
            describe_entry(p, &mut section, &pools, &label, entry, len);
            describe_attachments(p, &mut section, &pools, &label, entry);
        }
    }
    section
}

// --- Script variables --------------------------------------------------------------------------
fn script_type(kind: u8) -> &'static str {
    match kind {
        1 => "objet",
        2 => "chaîne",
        3 => "chaîne localisée",
        4 => "vecteur",
        5 => "flottant",
        6 => "entier",
        7 => "position de code",
        9 => "fonction",
        0xA => "fonction du moteur",
        0xB => "méthode du moteur",
        _ => "autre",
    }
}

/// Fields of a script object: (slot, name number, type, raw 8-byte value).
fn object_fields(p: &Process, object: u32) -> Vec<(usize, u32, u8, [u8; 8])> {
    let first = CHILD_PARTITION * (object as usize & 3);
    let raw = p.read_lossy(CHILD_VARIABLES + first * CHILD_SIZE, CHILD_PARTITION * CHILD_SIZE);
    raw.chunks(CHILD_SIZE)
        .enumerate()
        .filter(|(_, slot)| u16::from_le_bytes([slot[14], slot[15]]) == object as u16 && (1..0x18).contains(&slot[10]))
        .map(|(index, slot)| (first + index, u32::from(slot[11]) | (u32::from(u16::from_le_bytes([slot[12], slot[13]])) << 8), slot[10], slot[..8].try_into().unwrap()))
        .collect()
}

fn field_value(kind: u8, raw: &[u8; 8]) -> String {
    let shown = match kind {
        6 => int(raw, 0).to_string(),
        5 => show_float(float(raw, 0)),
        1 => format!("objet n° {}", int(raw, 0) as u32),
        2 | 3 => format!("chaîne n° {}", int(raw, 0) as u32),
        _ => format!("{:#x}", u64::from_le_bytes(*raw)),
    };
    format!("{} (type {kind}) = {shown}   [brut {}]", script_type(kind), trainer::hex(raw))
}

fn level_section(p: &Process) -> Section {
    let mut section = Section::new("Champs de l'objet « level » des scripts");
    let Some(level) = trainer::level_object(p) else {
        section.note("Objet « level » introuvable : aucun script ne tourne.");
        return section;
    };
    let fields = object_fields(p, level);
    section.note(format!("level = objet n° {level}, {} champs. Le nom d'un champ est un numéro, réattribué à chaque chargement des scripts ; la manche (level.wave_num) est l'un des entiers.", fields.len()));
    if fields.len() > MAX_FIELDS {
        section.note(format!("Seuls les {MAX_FIELDS} premiers sont listés."));
    }
    for (slot, name, kind, raw) in fields.iter().take(MAX_FIELDS) {
        section.entry(format!("nom {name:#08x} (case {slot})"), field_value(*kind, raw));
    }
    section
}

fn player_objects_section(p: &Process) -> Section {
    let mut section = Section::new("Champs de script du joueur (candidats)");
    section.note("Le moteur ne publie pas le lien entre une entité et son objet de script, donc l'objet du joueur ne peut pas être désigné avec certitude.");
    section.note("Sont listés les objets de type entité dont le numéro d'entité est 0 : le joueur hôte devrait en faire partie. Cela suppose le type dans l'octet bas du second mot de l'objet, ce qui n'a jamais été vérifié.");
    let table = p.read_lossy(SCR_VAR_GLOB, OBJECT_COUNT as usize * 8);
    let candidates: Vec<u32> = table.chunks(8).enumerate().filter(|(_, raw)| raw[4] == VAR_ENTITY && raw[2] == 0 && raw[3] == 0).map(|(id, _)| id as u32).collect();
    if candidates.is_empty() {
        section.note("Aucun objet ne correspond.");
    }
    if candidates.len() > MAX_OBJECTS {
        section.note(format!("{} objets correspondent ; seuls les {MAX_OBJECTS} premiers sont détaillés.", candidates.len()));
    }
    for &object in candidates.iter().take(MAX_OBJECTS) {
        let fields = object_fields(p, object);
        let raw = &table[object as usize * 8..object as usize * 8 + 8];
        section.entry(format!("objet {object}"), format!("{} champs   [brut {}]", fields.len(), trainer::hex(raw)));
        for (slot, name, kind, raw) in fields.iter().take(MAX_OBJECT_FIELDS) {
            section.entry(format!("objet {object} nom {name:#08x} (case {slot})"), field_value(*kind, raw));
        }
    }
    section
}

// --- Player and zombies ------------------------------------------------------------------------
fn player_section(p: &Process) -> Section {
    let mut section = Section::new("Joueur hôte (état du joueur, client, entité)");
    section.note("Champs connus d'abord (décalages d'iw7-mod), puis vidages. La position, les angles et les horloges bougent à chaque relevé.");
    let client = p.read_lossy(HOST_CLIENT, 0x200);
    let floats = |at: usize| format!("{} ; {} ; {}", show_float(float(&client, at)), show_float(float(&client, at + 4)), show_float(float(&client, at + 8)));
    let long = |at: usize| format!("{:#018x}", u64::from_le_bytes(client[at..at + 8].try_into().unwrap()));
    for (name, at) in [("commandTime", 0), ("pm_type", 4), ("pm_time", 8), ("linkFlags", 28), ("gravity", 120), ("speed", 124), ("groundEntityNum", 140), ("jumpTime", 156), ("eFlags", 344)] {
        section.entry(format!("état du joueur +{at:#05x} {name}"), format!("{} ({:#x})", int(&client, at), int(&client, at) as u32));
    }
    section.entry("état du joueur +0x00c pm_flags", long(12));
    section.entry("état du joueur +0x014 otherFlags", long(20));
    section.entry("état du joueur +0x028 origin", floats(40));
    section.entry("état du joueur +0x034 velocity", floats(52));
    section.entry("état du joueur +0x080 delta_angles", floats(128));
    section.entry("état du joueur +0x1b8 viewangles", floats(440));
    section.entry("client +0x4a90 connected", read_int(p, HOST_CLIENT + 19088));
    section.entry("client +0x4a94 sessionState", read_int(p, HOST_CLIENT + 19092));
    section.entry("client +0x4bb0 name", p.read_c_string(HOST_CLIENT + 19376, 32).unwrap_or_else(|| "illisible".to_string()));
    section.entry("client +0x4be0 team", read_int(p, HOST_CLIENT + 19424));
    section.entry(format!("client +{CLIENT_FLAGS:#06x} flags"), read_int(p, HOST_CLIENT + CLIENT_FLAGS));
    // Captured on 2026-10-14: the held weapon's 20-byte record (number, camo, attachments), the same
    // one that keys the ammo. Pack-a-Punch only changes bytes in it.
    section.entry("client +0x4ab0 arme en main", trainer::hex(&p.read_lossy(HELD_WEAPON, 20)));
    section.entry(format!("entité +{ENTITY_CLIENT:#05x} client"), p.read_usize(G_ENTITIES + ENTITY_CLIENT).map_or_else(|_| "illisible".to_string(), |ptr| format!("{ptr:#x}")));
    section.entry("entité +0x188 sentient", p.read_usize(G_ENTITIES + 0x188).map_or_else(|_| "illisible".to_string(), |ptr| format!("{ptr:#x}")));
    section.entry("entité +0x1ac classname (chaîne n°)", read_int(p, G_ENTITIES + 0x1AC));
    section.entry(format!("entité +{ENTITY_FLAGS:#05x} flags"), read_int(p, G_ENTITIES + ENTITY_FLAGS));
    section.entry(format!("entité +{ENTITY_HEALTH:#05x} health"), read_int(p, G_ENTITIES + ENTITY_HEALTH));
    let money = p.read_usize(SVS_CLIENTS).ok().filter(|&clients| clients > 0x1_0000 && clients < 0x7FFF_0000_0000).map(|clients| clients + SERVER_CLIENT_MONEY);
    section.entry("argent (joueur du serveur)", money.map_or_else(|| "aucun serveur en cours".to_string(), |addr| format!("{} en {addr:#x}", read_int(p, addr))));
    section.hex_rows(p, "Client, état du joueur et munitions", HOST_CLIENT, 0xC00);
    section.hex_rows(p, "Client, session", HOST_CLIENT + 0x4A80, 0x780);
    section.hex_rows(p, "Entité du joueur", G_ENTITIES, ENTITY_SIZE);
    section
}

fn zombies_section(p: &Process) -> Section {
    let mut section = Section::new("Emplacements de zombies");
    let mut alive = 0;
    for slot in 0..AGENT_SLOTS {
        let entity = G_ENTITIES + (FIRST_AGENT + slot) * ENTITY_SIZE;
        let raw = p.read_lossy(entity, ENTITY_SIZE);
        let (health, agent) = (int(&raw, ENTITY_HEALTH), usize::from_le_bytes(raw[0x180..0x188].try_into().unwrap()));
        if health == 0 && agent == 0 {
            continue;
        }
        alive += usize::from(health > 0);
        section.entry(
            format!("emplacement {slot:<2} [{entity:#011x}]"),
            format!("vie {health} ; drapeaux {:#x} ; agent {agent:#x} ; classname (chaîne n°) {}", int(&raw, ENTITY_FLAGS) as u32, int(&raw, 0x1AC) as u32),
        );
    }
    section.note(format!("{AGENT_SLOTS} emplacements (entités {FIRST_AGENT} à {}), {alive} avec une vie positive. Les emplacements vides sont omis.", FIRST_AGENT + AGENT_SLOTS - 1));
    section
}

// --- Check and code ----------------------------------------------------------------------------
fn control_section(p: &Process) -> Section {
    let mut section = Section::new("Contrôle des adresses et code relevé (non comparés)");
    section.compared = false;
    let (_, report) = trainer::verify(p);
    section.notes.extend(report);
    section.notes.push(String::new());
    section.notes.extend(trainer::describe_code(p));
    section
}

// --- Searching script fields by name -----------------------------------------------------------
/// Script fields whose storage place in the engine we look for. The zombie scripts skip any player
/// whose `ignoreme` is true: that is the clean "no target".
const FIELD_NAMES: [&str; 2] = ["ignoreme", "notarget"];
const SCAN_CHUNK: usize = 0x40_0000;
const MAX_NAME_HITS: usize = 8;
const MAX_ID_HITS: usize = 120;
const MAX_TREE_FUNCTIONS: usize = 40;
const TREE_BYTES: usize = 0x100;
const MAX_TABLES: usize = 40;
const TABLE_BYTES: usize = 0x180;
const MAX_REGISTER_SITES: usize = 48;
/// Name numbers of these fields in the public list of script names.
const FIELD_CANONICAL: [(&str, u32); 2] = [("ignoreme", 0x184), ("notarget", 0x204)];
/// Pointer to the script string table, read in the code of `SL_ConvertToString`.
const STRING_TABLE_PTR: usize = BASE + 0x60B_9588;
/// Write an object field, read it, and register a field (iw7-mod addresses).
const FIELD_FUNCTIONS: [(&str, usize); 4] = [("Scr_SetObjectField", BASE + 0x40_B6E0), ("GetEntityFieldValue", BASE + 0xC0_9CC0), ("Scr_AddClassField", BASE + 0xC0_61F0), ("Scr_SetEntityField", BASE + 0x40_B1C0)];
/// Pointer to the entity field table, read in the code that registers them (survey of 2026-10-14):
/// 0x30-byte rows, the pointer to the name first, then the name number.
const ENTITY_FIELDS_PTR: usize = BASE + 0x3F2_5A70;

/// Positions of `needle` in `haystack`.
fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }
    haystack.windows(needle.len()).enumerate().filter(|(_, window)| *window == needle).map(|(at, _)| at).collect()
}

/// A piece of the game image, or `None` if nothing in it is readable: there is then nothing to search.
fn read_chunk(p: &Process, addr: usize, len: usize) -> Option<Vec<u8>> {
    let mut data = vec![0u8; len];
    let mut readable = false;
    for (index, block) in data.chunks_mut(0x1_0000).enumerate() {
        readable |= p.read_bytes(addr + index * 0x1_0000, block).is_ok();
    }
    readable.then_some(data)
}

/// Addresses this code loads relative to the instruction (`lea register, [rip + offset]`) that fall
/// in the game data: they are the tables the function looks up.
fn data_targets(code: &[u8], addr: usize) -> Vec<usize> {
    let mut targets = Vec::new();
    for at in 0..code.len().saturating_sub(6) {
        let lea = matches!(code[at], 0x48 | 0x4C) && code[at + 1] == 0x8D && code[at + 2] & 0xC7 == 0x05;
        if !lea {
            continue;
        }
        let offset = i32::from_le_bytes(code[at + 3..at + 7].try_into().unwrap());
        let target = (addr + at + 7).wrapping_add_signed(offset as isize);
        if in_image(target) && !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets
}

/// Captures a function, what it calls (up to `depth` levels) and the tables it looks up.
fn code_tree(p: &Process, section: &mut Section, what: &str, addr: usize, depth: usize, seen: &mut Vec<usize>, tables: &mut Vec<usize>) {
    if seen.contains(&addr) || seen.len() >= MAX_TREE_FUNCTIONS {
        return;
    }
    seen.push(addr);
    let Some(code) = read_chunk(p, addr, TREE_BYTES) else {
        section.entry(format!("{what} {addr:#011x}"), "illisible");
        return;
    };
    section.entry(format!("{what} {addr:#011x}"), String::new());
    for (row, bytes) in code.chunks(32).enumerate() {
        section.entry(format!("  {addr:#011x} +{:#05x}", row * 32), trainer::hex(bytes));
    }
    for target in data_targets(&code, addr) {
        if !tables.contains(&target) {
            tables.push(target);
        }
    }
    if depth > 0 {
        for callee in trainer::call_targets(&code, addr) {
            code_tree(p, section, "appelée", callee, depth - 1, seen, tables);
        }
    }
}

/// Starts from a field's name to find where the engine stores it. Read-only: three passes over the
/// game image, and nothing else.
///
/// Fields the engine handles itself are registered by string number, not by address. So we
/// capture: the string and its number, every word of the image equal to that number (a field
/// table row), the code that reads and writes an object field with the tables it looks up, the
/// places where fields are registered, and the player structure where `ignoreme` should be.
fn field_names_section(p: &Process) -> Section {
    let mut section = Section::new("Recherche de champs par leur nom (non comparée)");
    section.compared = false;
    let image = IMAGE_END - BASE;
    let chunks = || (0..image).step_by(SCAN_CHUNK);

    // First pass: where the strings are, whole (a zero before, a zero after).
    let needles: Vec<Vec<u8>> = FIELD_NAMES.iter().map(|name| [&[0u8][..], name.as_bytes(), &[0u8][..]].concat()).collect();
    let mut names: Vec<(usize, &str)> = Vec::new();
    for start in chunks() {
        let Some(chunk) = read_chunk(p, BASE + start, (SCAN_CHUNK + 32).min(image - start)) else { continue };
        for (needle, name) in needles.iter().zip(FIELD_NAMES) {
            for at in find_all(&chunk, needle) {
                let addr = BASE + start + at + 1;
                if !names.contains(&(addr, name)) && names.iter().filter(|(_, known)| *known == name).count() < MAX_NAME_HITS {
                    names.push((addr, name));
                }
            }
        }
    }
    if names.is_empty() {
        section.note("Aucun de ces noms n'est écrit en clair dans l'image du jeu.");
    }
    // String number: the engine gives the text of string n at "base + 8 + 20 × n".
    let base = p.read_usize(STRING_TABLE_PTR).unwrap_or(0);
    section.entry("base de la table des chaînes", format!("{base:#x} (pointeur lu en {STRING_TABLE_PTR:#x})"));
    let mut ids: Vec<(u32, String)> = FIELD_CANONICAL.iter().map(|&(name, id)| (id, format!("numéro de nom de « {name} »"))).collect();
    for &(addr, name) in &names {
        let id = addr.checked_sub(base + 8).filter(|offset| base != 0 && offset % 20 == 0).map(|offset| offset / 20);
        section.entry(format!("chaîne « {name} »"), format!("{addr:#011x}, numéro de chaîne {}", id.map_or("incalculable".to_string(), |id| format!("{id} ({id:#x})"))));
        if let Some(id) = id.and_then(|id| u32::try_from(id).ok()).filter(|&id| id > 0xFF) {
            ids.push((id, format!("numéro de chaîne de « {name} »")));
        }
    }

    // Second pass: aligned 4-byte words equal to one of these numbers, outside the code.
    let mut hits = 0;
    for start in chunks() {
        let Some(chunk) = read_chunk(p, BASE + start, SCAN_CHUNK.min(image - start)) else { continue };
        for (index, raw) in chunk.as_chunks::<4>().0.iter().enumerate() {
            let (value, at) = (u32::from_le_bytes(*raw), BASE + start + index * 4);
            let Some((_, what)) = ids.iter().find(|(id, _)| *id == value) else { continue };
            // Small numbers show up everywhere: they only count when preceded by a pointer into the
            // game, as in a field table row (the name, then its number).
            let after_pointer = at >= 8 && index >= 2 && in_image(usize::from_le_bytes(chunk[(index - 2) * 4..index * 4].try_into().unwrap()));
            if value <= 0xFFFF && !what.contains("chaîne") && !after_pointer || trainer::CODE_RANGE.contains(&at) || hits >= MAX_ID_HITS {
                continue;
            }
            hits += 1;
            let from = (at & !0xF) - 0x20;
            section.entry(format!("{what} en {at:#011x}"), String::new());
            for (row, bytes) in p.read_lossy(from, 0x60).chunks(16).enumerate() {
                section.entry(format!("  {:#011x}", from + row * 16), trainer::hex(bytes));
            }
        }
    }
    section.entry("mots trouvés", format!("{hits} (au plus {MAX_ID_HITS})"));

    // The code that reads and writes an object field, what it calls, and the tables it looks up.
    let (mut seen, mut tables) = (Vec::new(), Vec::new());
    for (what, addr) in FIELD_FUNCTIONS {
        code_tree(p, &mut section, what, addr, 2, &mut seen, &mut tables);
    }
    for table in tables.into_iter().take(MAX_TABLES) {
        section.entry(format!("table consultée {table:#011x}"), String::new());
        for (row, bytes) in p.read_lossy(table, TABLE_BYTES).chunks(32).enumerate() {
            section.entry(format!("  {table:#011x} +{:#05x}", row * 32), trainer::hex(bytes));
        }
    }

    // The entity field table itself.
    match p.read_usize(ENTITY_FIELDS_PTR).ok().filter(|&addr| in_image(addr)) {
        Some(table) => {
            section.entry("table des champs d'entité", format!("{table:#011x} (pointeur lu en {ENTITY_FIELDS_PTR:#x})"));
            for (row, bytes) in p.read_lossy(table, 0x30 * 160).chunks(48).enumerate() {
                section.entry(format!("  ligne {row:3} {:#011x}", table + row * 48), trainer::hex(bytes));
            }
        }
        None => section.note("Table des champs d'entité : pointeur illisible."),
    }

    // Third pass, in the code only: the places where a field is registered.
    let register = FIELD_FUNCTIONS[2].1;
    let (code_start, code_len) = (trainer::CODE_RANGE.start, trainer::CODE_RANGE.end - trainer::CODE_RANGE.start);
    let mut sites = 0;
    for start in (0..code_len).step_by(SCAN_CHUNK) {
        let Some(chunk) = read_chunk(p, code_start + start, (SCAN_CHUNK + 8).min(code_len - start)) else { continue };
        for at in 0..chunk.len().saturating_sub(5).min(SCAN_CHUNK) {
            if chunk[at] != 0xE8 {
                continue;
            }
            let site = code_start + start + at;
            let target = (site + 5).wrapping_add_signed(i32::from_le_bytes(chunk[at + 1..at + 5].try_into().unwrap()) as isize);
            if target != register {
                continue;
            }
            sites += 1;
            if sites <= MAX_REGISTER_SITES {
                section.entry(format!("enregistrement de champ appelé en {site:#011x}"), String::new());
                for (row, bytes) in p.read_lossy(site - 0x60, 0x80).chunks(32).enumerate() {
                    section.entry(format!("  {:#011x}", site - 0x60 + row * 32), trainer::hex(bytes));
                }
            }
        }
    }
    section.entry("enregistrements de champ trouvés", format!("{sites} (au plus {MAX_REGISTER_SITES} relevés)"));

    // The structure the player entity points to at +0x180 (its "sentient": who sees it, who it ignores).
    match p.read_usize(G_ENTITIES + 0x180).ok().filter(|&addr| in_image(addr)) {
        Some(sentient) => {
            section.entry("structure désignée par l'entité du joueur (+0x180)", format!("{sentient:#011x}"));
            for (row, bytes) in p.read_lossy(sentient, 0x100).chunks(16).enumerate() {
                section.entry(format!("  {:#011x}", sentient + row * 16), trainer::hex(bytes));
            }
        }
        None => section.note("Entité du joueur : rien en +0x180 (hors partie, c'est attendu)."),
    }
    // The same structure for the first zombies present, to compare.
    for slot in 0..AGENT_SLOTS.min(3) {
        let entity = G_ENTITIES + (FIRST_AGENT + slot) * ENTITY_SIZE;
        for offset in [0x180, 0x190] {
            if let Some(pointed) = p.read_usize(entity + offset).ok().filter(|&addr| in_image(addr)) {
                section.entry(format!("zombie {slot} : structure désignée en +{offset:#x}"), format!("{pointed:#011x}"));
                for (row, bytes) in p.read_lossy(pointed, 0x80).chunks(16).enumerate() {
                    section.entry(format!("  {:#011x}", pointed + row * 16), trainer::hex(bytes));
                }
            }
        }
    }
    section
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Access;

    fn survey(in_match: bool, entries: &[(&str, &str)], clock: &str) -> Survey {
        let mut section = Section::new("Essai");
        entries.iter().for_each(|(key, value)| section.entry(*key, *value));
        let mut clocks = Section::new("Horloges (non comparées)");
        clocks.compared = false;
        clocks.entry("horloge", clock);
        Survey { in_match, sections: vec![section, clocks] }
    }

    #[test]
    fn the_second_survey_starts_with_what_changed() {
        let first = survey(true, &[("vie", "100"), ("argent", "500"), ("0x143ca2060", "01 02")], "10");
        let second = survey(true, &[("vie", "100"), ("argent", "750"), ("manche", "3")], "99");
        // One changed value, one new, one missing; the unchanged value and the clock are not listed.
        assert_eq!(second.changes(&first), ["[Essai] argent : 500  ->  750", "[Essai] manche : (absent)  ->  3", "[Essai] 0x143ca2060 : 01 02  ->  (absent)"]);
        assert!(second.changes(&second).is_empty());

        let header = ["Date : test".to_string()];
        let alone = first.render(&header, None);
        assert!(alone.starts_with("RELEVÉ COMPLET\nLecture seule"));
        assert!(alone.contains("== Ce qui a changé depuis le relevé précédent (en partie) ==\n  Premier relevé"));
        assert!(alone.contains("== Essai ==") && alone.contains("argent") && alone.contains("Date : test"));
        let compared = second.render(&header, Some(&first));
        assert!(compared.contains("  3 différence(s)."));
        // The differences come before the sections.
        assert!(compared.find("[Essai] argent : 500  ->  750").unwrap() < compared.find("== Essai ==").unwrap());
    }

    #[test]
    fn the_size_of_a_pool_entry_is_the_usual_gap_between_names() {
        // Names at +0x10 of 0x2C0-byte entries, a free slot in the middle, and a stray pointer.
        let offsets = [0x10, 0x2D0, 0x590, 0xB10, 0xB18, 0xDD0, 0x1090];
        assert_eq!(pool_stride(&offsets), Some((0x2C0, 0x10)));
        assert_eq!(pool_stride(&[0x10]), None);
        assert_eq!(pool_stride(&[0x10, 0x40, 0x90]), None, "deux écarts tous différents ne disent rien");
    }

    #[test]
    fn the_size_of_a_pool_entry_comes_from_its_free_slots() {
        // Pool of 8 entries of 0x138 bytes: the header points to entry 2, which points to 5, then 7.
        const STRIDE: usize = 0x138;
        let mut pool = vec![0u64; (8 + 8 * STRIDE) / 8];
        let base = pool.as_ptr() as usize;
        let entry = |index: usize| base + 8 + index * STRIDE;
        for (at, next) in [(base, entry(2)), (entry(2), entry(5)), (entry(5), entry(7)), (entry(7), 0)] {
            pool[(at - base) / 8] = next as u64;
        }
        let me = Process::open(std::process::id(), "self".into(), Access::Memory).unwrap();
        assert_eq!(free_list_stride(&me, base, 8), Some(STRIDE));
        // Only two free slots: too few to rely on.
        pool[(entry(5) - base) / 8] = 0;
        assert_eq!(free_list_stride(&me, base, 8), None);
    }

    #[test]
    fn whole_names_are_found_in_a_block_of_memory() {
        let block = b"\0xignoreme\0ignoreme\0ignoremegroup\0\0notarget\0";
        assert_eq!(find_all(block, b"\0ignoreme\0"), [10]);
        assert_eq!(find_all(block, b"\0notarget\0"), [34]);
        assert!(find_all(b"abc", b"abcd").is_empty());
        assert!(find_all(b"abc", b"").is_empty());
    }

    #[test]
    fn tables_read_by_a_function_are_found_in_its_code() {
        // `lea register, [rip + offset]` to the data, to another part of the image (kept too: tables
        // can sit next to the code), then a duplicate (ignored).
        let addr = BASE + 0x40_B6E0;
        let data = BASE + 0x300_0000;
        let lea = |at: usize, target: usize, rex: u8, modrm: u8| {
            let offset = (target as i64 - (addr + at + 7) as i64) as i32;
            [&[rex, 0x8D, modrm][..], &offset.to_le_bytes()].concat()
        };
        let mut code = vec![0x90u8; 0x40];
        code[0x00..0x07].copy_from_slice(&lea(0x00, data, 0x48, 0x0D));
        code[0x10..0x17].copy_from_slice(&lea(0x10, BASE + 0x50_0000, 0x48, 0x15));
        code[0x20..0x27].copy_from_slice(&lea(0x20, data, 0x4C, 0x05));
        code[0x30..0x37].copy_from_slice(&lea(0x30, data + 0x80, 0x4C, 0x3D));
        assert_eq!(data_targets(&code, addr), [data, BASE + 0x50_0000, data + 0x80]);
    }

    #[test]
    fn long_lists_of_changes_are_cut() {
        let many: Vec<(String, String)> = (0..MAX_CHANGES + 20).map(|index| (format!("case {index}"), "1".to_string())).collect();
        let borrowed: Vec<(&str, &str)> = many.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect();
        let text = survey(false, &borrowed, "0").render(&[], Some(&survey(false, &[], "0")));
        assert!(text.contains("au menu (hors partie)") && text.contains("… 20 autres différences non listées."));
    }

    #[test]
    fn floats_and_dvar_values_are_readable() {
        assert_eq!((show_float(39.0), show_float(0.0), show_float(0.5)), ("39".to_string(), "0".to_string(), "0.5".to_string()));
        assert_eq!(show_float(f32::from_bits(1)), "1e-45");
        let p = Process::open(std::process::id(), "self".into(), Access::MemoryRead).unwrap();
        let mut raw = [0u8; 16];
        raw[..4].copy_from_slice(&85i32.to_le_bytes());
        raw[4..8].copy_from_slice(&trainer::hash_integer(85).to_le_bytes());
        assert_eq!(dvar_value(&p, 5, &raw), "85");
        raw[4] ^= 1;
        assert!(dvar_value(&p, 5, &raw).starts_with("85 (empreinte non conforme"));
        raw[..4].copy_from_slice(&800.0f32.to_le_bytes());
        assert_eq!(dvar_value(&p, 1, &raw), "800");
        assert_eq!(dvar_value(&p, 0, &[0u8; 16]), "faux");
        assert_eq!(dvar_value(&p, 7, &[0u8; 16]), "(vide)");
        assert_eq!(dvar_domain(5, &[0, 0, 0, 0, 100, 0, 0, 0]), "0 à 100");
    }

    /// On a process that is not the game, each section says it could read nothing, and the survey completes.
    #[test]
    fn a_survey_of_something_else_than_the_game_still_completes() {
        let p = Process::open(std::process::id(), "self".into(), Access::MemoryRead).unwrap();
        let taken = take(&p);
        assert!(!taken.in_match);
        let text = taken.render(&[], None);
        for expected in ["== Profil Zombies", "Zone du profil : illisible", "== Réglages du moteur (dvars) ==", "Table illisible", "Réserve des tenues illisible", "== Contrôle des adresses"] {
            assert!(text.contains(expected), "{expected}");
        }
        // Outside a match, the match-only sections are not written.
        assert!(!text.contains("== Joueur hôte"));
    }
}
