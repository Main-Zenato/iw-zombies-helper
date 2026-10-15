//! One-off changes to the persistent Zombies profile.
//!
//! Unlike trainer options, they are not undone when leaving the match: the game saves the profile
//! and sends it online. Each change checks all of its targets before writing a single one.

use crate::lang::{tr, trs};
use crate::process::Process;

const XP: usize = 0x1_45EC_A68C;
/// XP for level 999, the progression cap.
const MAX_XP: i32 = 95_310_685;
const SANE_XP: std::ops::RangeInclusive<i32> = 0..=1_000_000_000;

const DIRECTORS_CUT_STATE: usize = 0x1_45EC_B830;
const DIRECTORS_CUT_UNLOCKED: i32 = 0x7F_07C1;
/// One-byte flags to set around the state word, in this order.
const DIRECTORS_CUT_FLAGS: [(usize, u8); 8] = [
    (0x1_45EC_A686, 1),
    (0x1_45EC_A687, 1),
    (0x1_45EC_A688, 1),
    (0x1_45EC_A689, 1),
    (0x1_45EC_A68A, 1),
    (0x1_45EC_A68B, 0xFF),
    (0x1_45EC_B841, 0xFF),
    (0x1_45EC_A680, 1),
];

/// Setting this byte to 1 makes the game erase the profile.
const RESET_FLAG: usize = 0x1_45EC_A19D;

/// The two current contracts (bounties): 6 bytes each.
const CONTRACTS: usize = 0x1_45EC_B661;
/// Two contracts that the next in-match purchase completes.
const CONTRACTS_READY: [u8; 12] = [0x4F, 0xC3, 0x00, 0x00, 0x29, 0x02, 0x4F, 0xC3, 0x00, 0x00, 0x29, 0x02];
/// Byte to set to 0x67 to draw new contracts.
const CONTRACTS_REROLL: usize = 0x1_45EC_B66D;
const CONTRACTS_REROLL_VALUE: u8 = 0x67;

/// `zombiePlayerLoadout.characterSelect`: the special character requested for the next match.
/// The game script (`get_player_character_num`) only honours the
/// code on that character's map, then resets it to zero: it lasts for a single match.
const CHARACTER: usize = 0x1_45EC_B59D;
/// Code, character, map on which the game accepts it.
pub const CHARACTERS: [(u8, &str, &str); 6] = [
    (0, "Tirage normal", "Toutes les cartes"),
    (1, "David Hasselhoff", "Zombies in Spaceland"),
    (5, "Willard Wyler", "Zombies in Spaceland"),
    (2, "Kevin Smith", "Rave in the Redwoods"),
    (3, "Pam Grier", "Shaolin Shuffle"),
    (4, "Elvira", "Attack of the Radioactive Thing"),
];

const WEAPON_XP_VALUE: i32 = 999_999_999;
const WEAPON_XP: [usize; 47] = [
    0x1_45EC_1CD2, 0x1_45EC_1CDC, 0x1_45EC_1CE6, 0x1_45EC_1CF0, 0x1_45EC_1CFA, 0x1_45EC_1D04, 0x1_45EC_1D0A, 0x1_45EC_1D22,
    0x1_45EC_1D2C, 0x1_45EC_1D36, 0x1_45EC_1D40, 0x1_45EC_1D4A, 0x1_45EC_1D68, 0x1_45EC_1D72, 0x1_45EC_1D78, 0x1_45EC_1D90,
    0x1_45EC_1D9A, 0x1_45EC_1DA4, 0x1_45EC_1DAE, 0x1_45EC_1DB8, 0x1_45EC_1DC2, 0x1_45EC_1DC8, 0x1_45EC_1DD6, 0x1_45EC_1DDC,
    0x1_45EC_1DEA, 0x1_45EC_1DF4, 0x1_45EC_1DFE, 0x1_45EC_1E08, 0x1_45EC_1E12, 0x1_45EC_1E1C, 0x1_45EC_1E26, 0x1_45EC_1E30,
    0x1_45EC_1E54, 0x1_45EC_1E6C, 0x1_45EC_1E72, 0x1_45EC_1E80, 0x1_45EC_1E8A, 0x1_45EC_1E94, 0x1_45EC_1E9E, 0x1_45EC_1EA8,
    0x1_45EC_1EB2, 0x1_45EC_1EBC, 0x1_45EC_1EC6, 0x1_45EC_1ED0, 0x1_45EC_1EDA, 0x1_45EC_1EE4, 0x1_45EC_1EFE,
];

/// Start and end of the profile area holding every known cell.
pub(crate) const BLOCK: std::ops::Range<usize> = 0x1_45EC_1C00..0x1_45EC_B900;

/// Zombies statistics in the profile: single-source addresses, never checked in game nor written
/// by the tool. They only serve the full survey, read-only.
const STATISTICS: [(&str, usize); 10] = [
    ("Éliminations", 0x1_45EC_AEC2),
    ("Tirs à la tête", 0x1_45EC_AEDE),
    ("Mises à terre", 0x1_45EC_AEBE),
    ("Manche la plus haute", 0x1_45EC_AEDA),
    ("Manches survécues", 0x1_45EC_AED6),
    ("Atouts utilisés", 0x1_45EC_AF0A),
    ("Portes ouvertes", 0x1_45EC_AEEA),
    ("Parties jouées", 0x1_45EC_AF06),
    ("Temps de jeu", 0x1_45EC_AF02),
    ("Prestige multijoueur", 0x1_45EC_3ECA),
];

/// A known profile cell, for the full survey: name, address, size in bytes (4 = integer).
pub(crate) fn known_cells() -> Vec<(String, usize, usize)> {
    let mut cells = vec![
        ("XP".to_string(), XP, 4),
        ("Director's Cut, état".to_string(), DIRECTORS_CUT_STATE, 4),
        ("Indicateur de remise à zéro".to_string(), RESET_FLAG, 1),
        ("Contrats en cours".to_string(), CONTRACTS, CONTRACTS_READY.len()),
        ("Contrats, nouveau tirage".to_string(), CONTRACTS_REROLL, 1),
        ("Personnage de la prochaine partie".to_string(), CHARACTER, 1),
    ];
    cells.extend(DIRECTORS_CUT_FLAGS.iter().enumerate().map(|(index, &(addr, _))| (format!("Director's Cut, drapeau {}", index + 1), addr, 1)));
    cells.extend(STATISTICS.iter().map(|&(name, addr)| (format!("{name} (source unique, jamais vérifié)"), addr, 4)));
    cells.extend(WEAPON_XP.iter().enumerate().map(|(index, &addr)| (format!("XP d'arme, emplacement {}", index + 1), addr, 4)));
    cells
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Show,
    MaxLevel,
    WeaponXp,
    DirectorsCut,
    Contracts,
    RerollContracts,
    /// Special character for the next match, by its code (see CHARACTERS).
    Character(u8),
    Reset,
}

impl Action {
    pub const NAMES: &'static str = "show | max-level | weapon-xp | directors-cut | contracts | reroll-contracts | character-0 | character-1 | character-2 | character-3 | character-4 | character-5 | reset";

    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "show" => Self::Show,
            "max-level" => Self::MaxLevel,
            "weapon-xp" => Self::WeaponXp,
            "directors-cut" => Self::DirectorsCut,
            "contracts" => Self::Contracts,
            "reroll-contracts" => Self::RerollContracts,
            "reset" => Self::Reset,
            other => {
                let code = other.strip_prefix("character-")?.parse().ok()?;
                return CHARACTERS.iter().any(|(known, ..)| *known == code).then_some(Self::Character(code));
            }
        })
    }

    pub fn writes(self) -> bool {
        self != Self::Show
    }
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Show => tr("Read the profile", "Lire le profil"),
            Action::MaxLevel => tr("Level 999", "Niveau 999"),
            Action::WeaponXp => tr("Max weapon XP", "XP d'armes au maximum"),
            Action::DirectorsCut => tr("Unlock Director's Cut", "Débloquer le Director's Cut"),
            Action::Contracts => tr("Prepare contracts (bounties)", "Préparer les contrats (primes)"),
            Action::RerollContracts => tr("Draw new contracts", "Retirer de nouveaux contrats"),
            Action::Character(_) => tr("Character for the next match", "Personnage de la prochaine partie"),
            Action::Reset => tr("Reset the account", "Remettre le compte à zéro"),
        }
    }
}

/// Runs the action and returns the report, line by line.
pub fn run(p: &Process, action: Action) -> Result<Vec<String>, String> {
    match action {
        Action::Show => show(p),
        Action::MaxLevel => set_ints(p, "XP", &[XP], MAX_XP),
        Action::WeaponXp => set_ints(p, tr("Weapon XP", "XP d'armes"), &WEAPON_XP, WEAPON_XP_VALUE),
        Action::DirectorsCut => unlock_directors_cut(p),
        Action::Contracts => {
            let before = read_contracts(p)?;
            write(p, CONTRACTS, &CONTRACTS_READY)?;
            Ok(vec![
                trs(format!("Contracts: {} -> {}", hex(&before), hex(&CONTRACTS_READY)), format!("Contrats : {} -> {}", hex(&before), hex(&CONTRACTS_READY))),
                tr("Do this in the pre-game lobby, then start a match and buy something.", "À faire dans le salon d'avant-partie ; lancer ensuite une partie et acheter quelque chose.").to_string(),
            ])
        }
        Action::RerollContracts => {
            let before = read_u8(p, CONTRACTS_REROLL)?;
            write(p, CONTRACTS_REROLL, &[CONTRACTS_REROLL_VALUE])?;
            Ok(vec![trs(format!("New contract draw: byte {before:#04x} -> {CONTRACTS_REROLL_VALUE:#04x}"), format!("Nouveau tirage des contrats : octet {before:#04x} -> {CONTRACTS_REROLL_VALUE:#04x}"))])
        }
        Action::Character(code) => {
            let before = read_u8(p, CHARACTER)?;
            if character_name(before).is_none() {
                return Err(trs(format!("Character: unexpected value ({before}), the profile does not seem loaded. Nothing was written."), format!("Personnage : valeur inattendue ({before}), le profil ne semble pas chargé. Rien n'a été écrit.")));
            }
            write(p, CHARACTER, &[code])?;
            let name = |code| character_name(code).unwrap_or(tr("unknown", "inconnu"));
            Ok(vec![trs(format!("Character: {} -> {} (for the next match)", name(before), name(code)), format!("Personnage : {} -> {} (pour la prochaine partie)", name(before), name(code)))])
        }
        Action::Reset => {
            let before = read_u8(p, RESET_FLAG)?;
            write(p, RESET_FLAG, &[1])?;
            Ok(vec![trs(format!("Reset flag: {before} -> 1. The game resets the profile."), format!("Indicateur de remise à zéro : {before} -> 1. Le jeu réinitialise le profil."))])
        }
    }
}

fn show(p: &Process) -> Result<Vec<String>, String> {
    let xp = read_i32(p, XP)?;
    let state = read_i32(p, DIRECTORS_CUT_STATE)?;
    let unlocked = if state == DIRECTORS_CUT_UNLOCKED { tr("unlocked", "débloqué") } else { tr("locked", "verrouillé") };
    let weapons = WEAPON_XP.iter().map(|&addr| read_i32(p, addr)).collect::<Result<Vec<_>, _>>()?;
    Ok(vec![
        trs(format!("XP                  : {xp} (level 999 = {MAX_XP})"), format!("XP                  : {xp} (niveau 999 = {MAX_XP})")),
        trs(format!("Director's Cut      : {unlocked} (state {state:#x})"), format!("Director's Cut      : {unlocked} (état {state:#x})")),
        trs(
            format!("Weapon XP           : {} slots, from {} to {}", weapons.len(), weapons.iter().min().unwrap(), weapons.iter().max().unwrap()),
            format!("XP d'armes          : {} emplacements, de {} à {}", weapons.len(), weapons.iter().min().unwrap(), weapons.iter().max().unwrap()),
        ),
        trs(format!("Contracts           : {}", hex(&read_contracts(p)?)), format!("Contrats            : {}", hex(&read_contracts(p)?))),
        {
            let character = read_character(p).and_then(character_name).unwrap_or(tr("unexpected value", "valeur inattendue"));
            trs(format!("Next character      : {character}"), format!("Personnage demandé  : {character}"))
        },
        {
            let flag = read_u8(p, RESET_FLAG)?;
            trs(format!("Reset               : flag at {flag}"), format!("Remise à zéro       : indicateur à {flag}"))
        },
    ])
}

/// Character name for a `characterSelect` code, if known.
fn character_name(code: u8) -> Option<&'static str> {
    // "Normal draw" is the only name that gets translated; the others are proper names.
    CHARACTERS.iter().find(|(known, ..)| *known == code).map(|&(known, name, _)| if known == 0 { tr("Normal draw", name) } else { name })
}

/// Code of the character requested for the next match. `None` if the profile is not readable or if
/// the value is not a known code (profile not loaded yet).
pub fn read_character(p: &Process) -> Option<u8> {
    p.read_u8(CHARACTER).ok().filter(|code| character_name(*code).is_some())
}

/// Sets each address to `value`, after checking they all hold a plausible amount of XP.
fn set_ints(p: &Process, label: &str, addrs: &[usize], value: i32) -> Result<Vec<String>, String> {
    let before = addrs.iter().map(|&addr| read_i32(p, addr)).collect::<Result<Vec<_>, _>>()?;
    if let Some(odd) = before.iter().find(|v| !SANE_XP.contains(v)) {
        return Err(trs(format!("{label}: unexpected value ({odd}), the profile does not seem loaded. Nothing was written."), format!("{label} : valeur inattendue ({odd}), le profil ne semble pas chargé. Rien n'a été écrit.")));
    }
    for &addr in addrs {
        write(p, addr, &value.to_le_bytes())?;
    }
    let (min, max) = (before.iter().min().unwrap(), before.iter().max().unwrap());
    Ok(vec![trs(format!("{label}: {} slot(s), {min}..{max} -> {value}", addrs.len()), format!("{label} : {} emplacement(s), {min}..{max} -> {value}", addrs.len()))])
}

fn unlock_directors_cut(p: &Process) -> Result<Vec<String>, String> {
    let before = read_i32(p, DIRECTORS_CUT_STATE)?;
    for (addr, _) in DIRECTORS_CUT_FLAGS {
        read_u8(p, addr)?;
    }
    for (addr, flag) in &DIRECTORS_CUT_FLAGS[..6] {
        write(p, *addr, &[*flag])?;
    }
    write(p, DIRECTORS_CUT_STATE, &DIRECTORS_CUT_UNLOCKED.to_le_bytes())?;
    for (addr, flag) in &DIRECTORS_CUT_FLAGS[6..] {
        write(p, *addr, &[*flag])?;
    }
    Ok(vec![trs(format!("Director's Cut: state {before:#x} -> {DIRECTORS_CUT_UNLOCKED:#x}"), format!("Director's Cut : état {before:#x} -> {DIRECTORS_CUT_UNLOCKED:#x}"))])
}

fn read_contracts(p: &Process) -> Result<[u8; 12], String> {
    let mut contracts = [0u8; 12];
    p.read_bytes(CONTRACTS, &mut contracts).map_err(|_| trs(format!("cannot read {CONTRACTS:#x}: is the profile loaded?"), format!("lecture impossible en {CONTRACTS:#x} : le profil n'est pas chargé ?")))?;
    Ok(contracts)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

fn read_i32(p: &Process, addr: usize) -> Result<i32, String> {
    p.read_i32(addr).map_err(|_| trs(format!("cannot read {addr:#x}: is the profile loaded?"), format!("lecture impossible en {addr:#x} : le profil n'est pas chargé ?")))
}

fn read_u8(p: &Process, addr: usize) -> Result<u8, String> {
    p.read_u8(addr).map_err(|_| trs(format!("cannot read {addr:#x}: is the profile loaded?"), format!("lecture impossible en {addr:#x} : le profil n'est pas chargé ?")))
}

/// Writes then reads back, to report a write that was silently ignored.
fn write(p: &Process, addr: usize, data: &[u8]) -> Result<(), String> {
    p.write_bytes(addr, data).map_err(|e| trs(format!("write refused at {addr:#x}: {e}"), format!("écriture refusée en {addr:#x} : {e}")))?;
    let mut check = vec![0u8; data.len()];
    match p.read_bytes(addr, &mut check) {
        Ok(()) if check == data => Ok(()),
        _ => Err(trs(format!("write not confirmed at {addr:#x}"), format!("écriture non confirmée en {addr:#x}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Access;

    #[test]
    fn action_names_round_trip() {
        for name in Action::NAMES.split(" | ") {
            assert!(Action::parse(name).is_some(), "{name}");
        }
        assert_eq!(Action::parse("delete"), None);
        // A character code the game does not know is not an action.
        assert_eq!(Action::parse("character-5"), Some(Action::Character(5)));
        assert_eq!(Action::parse("character-6"), None);
        assert_eq!((character_name(3), character_name(9)), (Some("Pam Grier"), None));
        assert!(!Action::Show.writes() && Action::Reset.writes());
    }

    #[test]
    fn tables_are_consistent() {
        assert_eq!(MAX_XP, 0x5AE_535D);
        let mut sorted = WEAPON_XP.to_vec();
        sorted.dedup();
        assert_eq!(sorted.len(), 47);
        assert!(WEAPON_XP.windows(2).all(|w| w[0] < w[1]));
        // The contracts and their draw byte follow each other (base + 5ECB661 and 5ECB66D).
        assert_eq!(CONTRACTS + CONTRACTS_READY.len(), CONTRACTS_REROLL);
        assert_eq!(CONTRACTS_READY[..6], CONTRACTS_READY[6..]);
        // The Director's Cut flags sit just below the XP word and must not overlap it.
        assert!(DIRECTORS_CUT_FLAGS.iter().all(|(addr, _)| !(XP..XP + 4).contains(addr)));
    }

    #[test]
    fn set_ints_checks_everything_before_writing() {
        let p = Process::open(std::process::id(), "self".into(), Access::Memory).unwrap();
        let values = Box::new([10i32, 20, -5]);
        let addrs: Vec<usize> = values.iter().map(|v| v as *const i32 as usize).collect();
        assert!(set_ints(&p, "test", &addrs, 99).is_err());
        assert_eq!(p.read_i32(addrs[0]).unwrap(), 10);
        assert!(set_ints(&p, "test", &addrs[..2], 99).is_ok());
        assert_eq!((p.read_i32(addrs[0]).unwrap(), p.read_i32(addrs[1]).unwrap()), (99, 99));
        assert_eq!(p.read_i32(addrs[2]).unwrap(), -5);
    }

    #[test]
    fn unloaded_profile_is_reported() {
        let p = Process::open(std::process::id(), "self".into(), Access::Memory).unwrap();
        assert!(run(&p, Action::Show).is_err());
    }
}
