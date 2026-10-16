//! Memory features for IW7's latest Steam build (buildid 2715288, executable of 2018-04-18).
//!
//! Sources, by order of confidence:
//! - the iw7-mod code (auroramod), which gives the real structures and addresses of this build:
//!   `g_entities`, `gentity_s`, `gclient_s`, `client_t`, `dvar_t`, and how the engine itself
//!   implements `god` and `noclip`;
//! - addresses captured on the running game and cross-checked against each other;
//! - IW FPS Unlocker (TacoCake) for the location of the FPS limit.
//!
//! The game image has no ASLR and iw7-mod loads it at the same base: addresses are absolute.
//! The game code is encrypted on disk, so nothing here can be checked offline. Instead, the
//! trainer refuses to write until the running game's code matches `CODE_SITES`.

use crate::config;
use crate::lang::{self, tr, trs};
use crate::process::Process;
use crate::state::Pending;
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

pub(crate) const BASE: usize = 0x1_4000_0000;
pub(crate) const IMAGE_END: usize = BASE + 0xC30_1400;

// --- Entities (`gentity_s`, 0x3F8 bytes) ------------------------------------------------------
pub(crate) const G_ENTITIES: usize = BASE + 0x3D2_2610;
pub(crate) const ENTITY_SIZE: usize = 0x3F8;
/// `gentity_s::client`: pointer to the player's `gclient_s`, null for other entities.
pub(crate) const ENTITY_CLIENT: usize = 0x170;
/// `gentity_s::flags`: the flag the engine's `god` command toggles.
pub(crate) const ENTITY_FLAGS: usize = 0x1C8;
pub(crate) const ENTITY_HEALTH: usize = 0x1E0;
const FL_GODMODE: i32 = 1;
/// Entities 0 to 17 are players; agents (zombies) take the following slots.
pub(crate) const FIRST_AGENT: usize = 18;
pub(crate) const AGENT_SLOTS: usize = 24;
const PLAYER_HEALTH: usize = G_ENTITIES + ENTITY_HEALTH;
const DEFAULT_HEALTH: i32 = 100;
/// Negative health that the first version of "no target" wrote: the zombie scripts no longer aim at
/// a player whose health is not positive. It worked in game on 2026-10-13, but the screen stayed red
/// as if the player were dying. Replaced by `ignoreme` below; kept only to recognise and restore what
/// that first version may have left in a game (its value is exact). The engine's `notarget` flag has
/// no effect on zombies.
const NO_TARGET_HEALTH: i32 = i32::MIN + 1;
/// Structure that says who sees the entity and who it ignores (`sentient`), pointed to by the entity at
/// +0x180; it starts with a pointer back to that entity. Captured in a match on 2026-10-14: the
/// player's is at 0x145C6F5A0, the zombies' follow every 0xA0 bytes.
const ENTITY_SENTIENT: usize = 0x180;
/// `ignoreme`: the zombie scripts skip any player for whom this field is true. Stored at +0x1C,
/// before `ignoreall` and the "origin changed" flag, from this structure's usual layout in this
/// engine and the survey (team at +0x10, view distance at +0x2C: 20000 for the player, 8192 for a
/// zombie). Seen working in game on 2026-10-15.
const SENTIENT_IGNORE_ME: usize = 0x1C;

// --- Host player (`gclient_s`, which starts with its `playerState_s`) ------------------------
pub(crate) const HOST_CLIENT: usize = BASE + 0x3CA_2060;
/// `playerState_s::origin`: X, Y, Z as three consecutive floats.
pub(crate) const CLIENT_ORIGIN: usize = 0x28;
/// `gclient_s::flags`: the flag the engine's `noclip` command toggles.
pub(crate) const CLIENT_FLAGS: usize = 0x51BC;
const CLIENT_NOCLIP: i32 = 1;
/// Moves the Zombies mode scripts forbid to the player and that the "Jump pack" option gives back:
/// offset of the word in the client, "forbidden" bit, and highest plausible value of the word.
/// Read in the game code (surveys of 2026-10-14): the functions the scripts call to allow a move
/// clear this bit, and set it to forbid it. In a Zombies match the two words are 2 and 0x40:
/// double jump and wall running forbidden, even though the player's suit holds all their settings.
const DOUBLE_JUMP_FORBIDDEN: (usize, i32, i32) = (0x3FC, 0x2, 0xFF);
const WALL_RUN_FORBIDDEN: (usize, i32, i32) = (0x420, 0x40, 0xFFFF);
/// Ledge climbing, also forbidden by the mode's scripts. Read in the game code (survey of
/// 2026-10-14): `allowmantle` sets bit 28 of `pm_flags` (`client + 0xC`) to forbid it and clears
/// it to allow it; in a Zombies match `pm_flags` is 0x10000000, in multiplayer this bit is 0.
/// `pm_flags` changes all the time with the player's moves: only the byte holding this bit is read
/// and rewritten, so nothing else gets overwritten.
const MANTLE_FORBIDDEN_BYTE: usize = 0x0C + 3;
const MANTLE_FORBIDDEN_BIT: u8 = 0x10;
const FORBIDDEN_MOVES: [(usize, i32, i32); 2] = [DOUBLE_JUMP_FORBIDDEN, WALL_RUN_FORBIDDEN];
/// Rounds loaded per weapon slot (right and left hand), and reserve ammo.
const CLIP_SLOTS: [usize; 9] = [0x3CA_2AC0, 0x3CA_2AC4, 0x3CA_2AE8, 0x3CA_2B08, 0x3CA_2B0C, 0x3CA_2B2C, 0x3CA_2B30, 0x3CA_2B50, 0x3CA_2B54];
const RESERVE_SLOTS: [usize; 4] = [0x3CA_294C, 0x3CA_2994, 0x3CA_29AC, 0x3CA_29C4];
const CLIP_VALUE: i32 = 511;
const RESERVE_VALUE: i32 = 9999;

pub(crate) const TIMESCALE: usize = BASE + 0x600_5D84;
/// Number of the current round, at the fixed address where it sits in the first match.
/// It is a script variable, so a slot of `scr_VarGlob`: see the next section.
const ROUND: usize = BASE + 0x685_4DC0;

// --- Script variables (`scrVarGlob_t`, from iw7-mod) ------------------------------------------
// The game scripts keep their variables in one big table. Each object (including `level`) has a
// linked list of its fields; a field is a 24-byte slot: value, type, name (24-bit number) and
// link to the next field. A slot is assigned when the variable is created, so its address changes
// from one match to the next. The name number may change too, the game reassigning it every time
// the scripts are loaded (see `RoundVariable`).
pub(crate) const SCR_VAR_GLOB: usize = BASE + 0x669_1180;
/// `childVariableValue`: after 40960 objects of 8 bytes, 40960 lists of 4 bytes and 65536 16-bit words.
pub(crate) const CHILD_VARIABLES: usize = SCR_VAR_GLOB + 40_960 * 8 + 40_960 * 4 + 65_536 * 2;
pub(crate) const CHILD_SIZE: usize = 24;
/// Slots are split into four partitions: an object keeps its fields in partition `id & 3`
/// (iw7-mod). The table cannot hold the 384000 slots iw7-mod declares: `scr_VmPub` (0x146B183B0)
/// would then sit in the middle of it.
pub(crate) const CHILD_PARTITION: usize = 0xA000;
const CHILD_COUNT: usize = 4 * CHILD_PARTITION;
pub(crate) const OBJECT_COUNT: u32 = 40_960;
/// Id of the scripts' `level` object.
const LEVEL_ENTITY_ID: usize = BASE + 0x665_A120;
/// Value types (`VAR_*` from iw7-mod): empty slot, integer, and the first value that is no longer a field type.
const VAR_UNDEFINED: u8 = 0;
const VAR_INTEGER: u8 = 6;
const VAR_FREE: u8 = 0x18;
/// Number of neighbouring fields kept around the round variable, and the minimum to rely on them.
/// Name number of the round variable among the `level` fields. Captured on the game: the same over
/// three matches of two different launches (2026-10-13 and 14), while the `level` object and the
/// field address change. Seen on several maps since.
const ROUND_NAME: u32 = 0x1_3BD3;
const ROUND_NEIGHBOURS: usize = 48;
const ROUND_MIN_NEIGHBOURS: usize = 12;
/// Type of a float value, and type of a script object that points to an entity.
const VAR_FLOAT: u8 = 5;
const VAR_ENTITY: u8 = 0x15;

// Other script fields named by their name. The numbers are those of IW7's public list of names,
// like the round's: all found under these numbers in the survey of 2026-10-14, with the value
// the mode's scripts give them.
/// `level.cash_scalar`: multiplies the money of each zombie killed (2 during Double Points).
const CASH_SCALAR_NAME: u32 = 0x3B0F;
/// `level.max_static_spawned_enemies`: zombies present at a time. If the game counts more, its spawn
/// loop kills the extra zombies (and brings them back later): at 0, the map empties.
const MAX_ZOMBIES_NAME: u32 = 0xB467;
/// `level.current_enemy_deaths` and `level.desired_enemy_deaths_this_wave`: the round ends when the
/// first reaches the second.
const ENEMY_DEATHS_NAME: u32 = 0x4B6E;
const WAVE_DEATHS_NAME: u32 = 0x5274;
/// Player fields: `maxcurrency` (used to recognise their object), the Fate & Fortune card meter and
/// its maximum, and arcade tickets.
const MAX_CURRENCY_NAME: u32 = 0xB48A;
const CARD_METER_NAME: u32 = 0x4560;
const CARD_METER_MAX_NAME: u32 = 0x4563;
const TICKETS_NAME: u32 = 0xC206;
/// `level.command_given`: the mailbox of the iw7-mod script (iw7-mod/custom_scripts/cp/iw_helper.gsc).
/// A name from the public list that no Zombies script gives `level` (only zombie agents use it). The
/// script keeps it at `SCRIPT_READY`, runs the command written as `SCRIPT_READY + code`, then puts
/// it back.
const SCRIPT_MAILBOX_NAME: u32 = 0x441A;
const SCRIPT_READY: i32 = 7_700;
const SANE_MAILBOX: std::ops::RangeInclusive<i32> = SCRIPT_READY..=SCRIPT_READY + 99;

// --- Server-side players (`client_t`, 0xAEA98 bytes per player) ------------------------------
/// `svs_clients`: pointer to the server's player array, allocated when the match starts.
pub(crate) const SVS_CLIENTS: usize = BASE + 0x6B2_2950;
/// Size of a `client_t`: only used to cross-check the sources (see the tests).
#[cfg(test)]
const SERVER_CLIENT_SIZE: usize = 0xA_EA98;
/// Address of the money when the player array is allocated at its most common place. From one
/// player to the next it moves by 0xAEA98, the size of a `client_t`: the money is therefore a field
/// of that structure, and the fixed address is only right if the array is allocated there.
const LEGACY_MONEY: usize = 0x2_5E4B_EE4E;
/// Offset of the money in `client_t`. Captured in a match on 2026-10-13: the array was at
/// 0x25E415080 and infinite money worked at the fixed address.
pub(crate) const SERVER_CLIENT_MONEY: usize = LEGACY_MONEY - 0x2_5E41_5080;

const SANE_HEALTH: std::ops::RangeInclusive<i32> = 1..=1_000_000;
const SANE_AMMO: std::ops::RangeInclusive<i32> = 0..=100_000;
const SANE_MONEY: std::ops::RangeInclusive<i32> = 0..=10_000_000;
const SANE_ROUND: std::ops::RangeInclusive<i32> = 0..=9_999;
const SANE_TIMESCALE: std::ops::RangeInclusive<f32> = 0.01..=100.0;
const SANE_JUMP_HEIGHT: std::ops::RangeInclusive<f32> = 1.0..=2_000.0;
const SANE_CASH_SCALAR: std::ops::RangeInclusive<i32> = 0..=100;
const SANE_MAX_ZOMBIES: std::ops::RangeInclusive<i32> = 0..=100;
const SANE_DEATHS: std::ops::RangeInclusive<i32> = 0..=100_000;
const SANE_CARD_METER: std::ops::RangeInclusive<f32> = 0.0..=1_000_000.0;
const SANE_TICKETS: std::ops::RangeInclusive<i32> = 0..=10_000_000;
/// Amount added by the "add money" action.
const MONEY_GIFT: i32 = 10_000;
/// Tickets added by the "add tickets" action.
const TICKET_GIFT: i32 = 1_000;

// --- Code -------------------------------------------------------------------------------------
struct CodeSite {
    /// Name in English then in French.
    label: [&'static str; 2],
    addr: usize,
    original: &'static [u8],
    patched: &'static [u8],
}

const NO_RECOIL: CodeSite = CodeSite { label: ["No recoil", "Sans recul"], addr: BASE + 0x8E_188C, original: &[0x0F, 0x84, 0xAE, 0x00], patched: &[0x0F, 0x85, 0xAE, 0x00] };
const RAPID_FIRE: CodeSite = CodeSite { label: ["Rapid fire", "Tir rapide"], addr: BASE + 0x72_EB61, original: &[0x84], patched: &[0x94] };
const LOW_GRAVITY: CodeSite = CodeSite { label: ["Low gravity", "Faible gravité"], addr: BASE + 0x70_F47C, original: &[0x74], patched: &[0x75] };

// `add [rax+rdx*4+14h], edi` followed by `mov rcx, [rsp+60h]`: the ammo counter update.
const AMMO_PATTERN: [u8; 9] = [0x01, 0x7C, 0x90, 0x14, 0x48, 0x8B, 0x4C, 0x24, 0x60];

// The ammo decrement and damage instructions. `iw-helper` does not change them (ammo and one-hit
// kill freeze values): they only serve to recognise the game version and, in "turn everything
// off", to restore what another program may have left.
// `sub [rdi+1E0h], ebx` and `sub [rbx+1E0h], edi` are where damage is taken from an entity's health.
const AMMO_DECREMENT: CodeSite = CodeSite { label: ["Ammo decrement", "Décompte des munitions"], addr: BASE + 0x73_323D, original: &[0x01, 0x7C, 0x90, 0x14], patched: &[0x90; 4] };
const PLAYER_DAMAGE: CodeSite = CodeSite { label: ["Damage to the player", "Dégâts au joueur"], addr: BASE + 0xB0_F8BE, original: &[0x29, 0x9F, 0xE0, 0x01, 0x00, 0x00], patched: &[0x90; 6] };
const ZOMBIE_DAMAGE: CodeSite = CodeSite {
    label: ["Damage to zombies", "Dégâts aux zombies"],
    addr: BASE + 0xC1_B223,
    original: &[0x85, 0xFF, 0x74, 0x2F, 0x29, 0xBB, 0xE0, 0x01, 0x00, 0x00],
    // xor edi, edi / mov [rbx+1E0h], edi: the slightest hit sets the zombie's health to zero.
    patched: &[0x31, 0xFF, 0x90, 0x90, 0x89, 0xBB, 0xE0, 0x01, 0x00, 0x00],
};

/// Every known instruction. Finding them all at these addresses, original or patched, identifies
/// the game version. The last three only serve that purpose.
const CODE_SITES: [CodeSite; 6] = [NO_RECOIL, RAPID_FIRE, LOW_GRAVITY, AMMO_DECREMENT, PLAYER_DAMAGE, ZOMBIE_DAMAGE];
/// The only instructions the tool changes itself.
const OWN_CODE_SITES: [CodeSite; 3] = [NO_RECOIL, RAPID_FIRE, LOW_GRAVITY];

#[derive(Debug, PartialEq)]
enum SiteState {
    Original,
    Patched,
    Unknown(Vec<u8>),
    Unreadable,
}

impl CodeSite {
    fn name(&self) -> &'static str {
        lang::current().pick(self.label[0], self.label[1])
    }

    fn state(&self, p: &Process) -> SiteState {
        let mut current = vec![0u8; self.original.len()];
        if p.read_bytes(self.addr, &mut current).is_err() {
            SiteState::Unreadable
        } else if current == self.original {
            SiteState::Original
        } else if current == self.patched {
            SiteState::Patched
        } else {
            SiteState::Unknown(current)
        }
    }

    /// Toggles the patch; returns whether it is now active.
    fn toggle(&self, p: &Process) -> Result<bool, &'static str> {
        let (bytes, active) = match self.state(p) {
            SiteState::Original => (self.patched, true),
            SiteState::Patched => (self.original, false),
            _ => return Err(tr("unexpected code at this address", "code inattendu à cette adresse")),
        };
        p.write_bytes(self.addr, bytes).map_err(|_| tr("write refused", "écriture refusée"))?;
        Ok(active)
    }

    fn restore(&self, p: &Process) {
        if self.state(p) == SiteState::Patched {
            let _ = p.write_bytes(self.addr, self.original);
        }
    }
}

/// True when every known instruction sits where this game version puts it.
pub fn build_is_recognised(p: &Process) -> bool {
    CODE_SITES.iter().all(|site| matches!(site.state(p), SiteState::Original | SiteState::Patched))
}

// --- FPS limit (`dvar_t`, 96 bytes) -----------------------------------------------------------
/// Engine global that points to the FPS limit dvar; null before it is registered.
pub(crate) const FPS_DVAR_PTR: usize = BASE + 0x600_5758;
/// Location of this dvar in `dvarPool` (index 2506), used when the pointer is null.
const FPS_DVAR_DIRECT: usize = BASE + 0x75D_0A20;
const DVAR_TYPE: usize = 9;
const DVAR_TYPE_INTEGER: u8 = 5;
const DVAR_CURRENT: usize = 0x10;
const DVAR_LATCHED: usize = 0x20;
const DVAR_RESET: usize = 0x30;
const DVAR_DOMAIN: usize = 0x40;

// --- Engine settings found by the hash of their name -------------------------------------------
/// Number of registered settings, then a table of pointers to them, sorted by increasing hash.
/// Captured on the game on 2026-10-14: 3464 settings, each starting with the hash of its name.
pub(crate) const DVAR_COUNT: usize = BASE + 0x759_5E54;
pub(crate) const DVAR_POOL: usize = BASE + 0x759_5E60;
const MAX_DVARS: usize = 12_000;
/// Hash of the movement speed setting. The names are not in memory: this setting was recognised in
/// the survey as the only integer whose original value is 190, the engine's run speed, then seen
/// acting in game.
const MOVE_SPEED_DVAR: u32 = 0xA5D5_41B8;
const MOVE_SPEED_DEFAULT: i32 = 190;
const SANE_MOVE_SPEED: std::ops::RangeInclusive<i32> = 1..=2_000;

/// Address, in a table sorted by hash, of the setting that carries `hash`. `name_of` reads the hash
/// stored at the start of a setting; a failed read stops the search.
fn search_dvar(pointers: &[usize], hash: u32, name_of: impl Fn(usize) -> Option<u32>) -> Option<usize> {
    let (mut low, mut high) = (0, pointers.len());
    while low < high {
        let middle = low + (high - low) / 2;
        match name_of(pointers[middle])?.cmp(&hash) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Some(pointers[middle]),
        }
    }
    None
}

/// Address of the engine setting that carries this hash, or `None` if it is not registered (yet).
fn find_dvar(p: &Process, hash: u32) -> Option<usize> {
    let count = p.read_i32(DVAR_COUNT).ok().and_then(|count| usize::try_from(count).ok()).filter(|count| (1..=MAX_DVARS).contains(count))?;
    let table = p.read_lossy(DVAR_POOL, count * 8);
    let pointers: Vec<usize> = table.as_chunks::<8>().0.iter().map(|raw| usize::from_le_bytes(*raw)).collect();
    search_dvar(&pointers, hash, |addr| if (BASE..IMAGE_END).contains(&addr) { p.read_i32(addr).ok().map(|name| name as u32) } else { None })
}

/// Current and original value of an integer setting, if these bytes really are one: the type is
/// "integer" and the hash stored next to each value matches it.
fn parse_int_dvar(raw: &[u8; 0x48]) -> Option<(i32, i32)> {
    let int = |offset: usize| i32::from_le_bytes(raw[offset..offset + 4].try_into().unwrap());
    let (value, reset) = (int(DVAR_CURRENT), int(DVAR_RESET));
    (raw[DVAR_TYPE] == DVAR_TYPE_INTEGER && int(DVAR_CURRENT + 4) == hash_integer(value) && int(DVAR_RESET + 4) == hash_integer(reset)).then_some((value, reset))
}

/// Address and value of the movement speed setting. It is only returned if it looks like what was
/// captured: an integer whose original value is 190 and whose current value is plausible.
fn move_speed_dvar(p: &Process) -> Option<(usize, i32)> {
    let addr = find_dvar(p, MOVE_SPEED_DVAR)?;
    let mut raw = [0u8; 0x48];
    p.read_bytes(addr, &mut raw).ok()?;
    let (value, reset) = parse_int_dvar(&raw)?;
    (reset == MOVE_SPEED_DEFAULT && SANE_MOVE_SPEED.contains(&value)).then_some((addr, value))
}

fn write_move_speed(p: &Process, addr: usize, speed: i32) -> bool {
    let value = hashed_value(speed);
    p.write_bytes(addr + DVAR_CURRENT, &value).is_ok() && p.write_bytes(addr + DVAR_LATCHED, &value).is_ok()
}

/// Hash the engine stores next to each integer dvar value, and checks.
///
/// Port of iw7-mod's state machine (`hash_integer`), itself captured from the game.
pub(crate) fn hash_integer(value: i32) -> i32 {
    let mut hashed = value as u32;
    let mut temp = 0u32;
    let mut xor_key = 0x87C6_B47Fu32;
    let mut key = 0x89E1_A665u32;
    while key != 0xE9CC_2CC0 {
        match key {
            0x89E1_A665 => {
                temp = hashed & 0xFFFF;
                xor_key = 0x008B_237C;
            }
            0x1C9C_36AA => {
                temp = (hashed & 0xFFFF) ^ 0x7434;
                xor_key = 0xAB74_F2A4;
                hashed = hashed.rotate_right(16);
            }
            0x6006_EB8D => {
                temp = (hashed & 0xFFFF) ^ 0x9133;
                xor_key = 0xA6F5_E979;
                hashed = temp ^ hashed.rotate_left(16);
            }
            0x896A_8519 => {
                temp ^= 0x52D0;
                xor_key = 0x6B8E_2B15;
                hashed = hashed.rotate_left(16);
            }
            0xC6F3_02F4 => {
                xor_key = 0x2F3F_2E34;
                hashed = hashed.rotate_right(16);
            }
            0xE2E4_AE0C => {
                hashed ^= temp;
                xor_key = 0x82E2_4581;
            }
            0xF6B9_AD6A => {
                temp ^= 0xCA71;
                xor_key = 0x52DB_7D55;
            }
            _ => {}
        }
        key ^= xor_key;
    }
    hashed as i32
}

fn hashed_value(value: i32) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    bytes[..4].copy_from_slice(&value.to_le_bytes());
    bytes[4..].copy_from_slice(&hash_integer(value).to_le_bytes());
    bytes
}

#[derive(Debug, PartialEq)]
struct FpsDvar {
    addr: usize,
    limit: i32,
    min: i32,
    max: i32,
}

#[derive(Debug, PartialEq)]
enum FpsRead {
    /// An integer dvar whose hash matches its value: it really is the right structure.
    Valid(FpsDvar),
    /// Readable memory, but not (yet) the expected dvar.
    NotADvar { addr: usize },
    Unreadable,
}

fn read_fps_dvar(p: &Process) -> FpsRead {
    let addr = match p.read_usize(FPS_DVAR_PTR) {
        Ok(dvar) if (BASE..IMAGE_END).contains(&dvar) => dvar,
        _ => FPS_DVAR_DIRECT,
    };
    let mut raw = [0u8; 0x48];
    if p.read_bytes(addr, &mut raw).is_err() {
        return FpsRead::Unreadable;
    }
    let int = |offset: usize| i32::from_le_bytes(raw[offset..offset + 4].try_into().unwrap());
    let limit = int(DVAR_CURRENT);
    if raw[DVAR_TYPE] != DVAR_TYPE_INTEGER || limit <= 0 || int(DVAR_CURRENT + 4) != hash_integer(limit) {
        return FpsRead::NotADvar { addr };
    }
    FpsRead::Valid(FpsDvar { addr, limit, min: int(DVAR_DOMAIN), max: int(DVAR_DOMAIN + 4) })
}

#[derive(Debug, PartialEq)]
pub enum FpsOutcome {
    Set { previous: i32, limit: i32 },
    AlreadySet,
    /// The dvar is not registered yet, or the memory does not look like it.
    NotReady,
}

/// Sets the engine's FPS limit. The engine restores its limit when the game mode changes: this
/// function must therefore be called again regularly.
///
/// The dvar's bounds (0 to 100 in Zombies) are not applied: the engine only checks them when its
/// own commands write it, and IW FPS Unlocker writes 1000 there.
pub fn set_fps_limit(p: &Process, limit: i32) -> FpsOutcome {
    let FpsRead::Valid(dvar) = read_fps_dvar(p) else { return FpsOutcome::NotReady };
    if dvar.limit == limit {
        return FpsOutcome::AlreadySet;
    }
    let value = hashed_value(limit);
    let written = p.write_bytes(dvar.addr + DVAR_CURRENT, &value).is_ok() && p.write_bytes(dvar.addr + DVAR_LATCHED, &value).is_ok();
    if written {
        FpsOutcome::Set { previous: dvar.limit, limit }
    } else {
        FpsOutcome::NotReady
    }
}

// --- Trainer ----------------------------------------------------------------------------------
/// An option that is turned on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    GodMode,
    NoTarget,
    Noclip,
    InfiniteMoney,
    InfiniteAmmo,
    OneHitKill,
    NoRecoil,
    RapidFire,
    SlowMotion,
    LowGravity,
    JumpPack,
    PauseZombies,
}

impl Feature {
    pub const ALL: [Feature; 12] = [
        Feature::GodMode,
        Feature::NoTarget,
        Feature::Noclip,
        Feature::InfiniteMoney,
        Feature::InfiniteAmmo,
        Feature::OneHitKill,
        Feature::NoRecoil,
        Feature::RapidFire,
        Feature::SlowMotion,
        Feature::LowGravity,
        Feature::JumpPack,
        Feature::PauseZombies,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Feature::GodMode => tr("God mode", "Invincibilité"),
            Feature::NoTarget => tr("No target", "Sans cible"),
            Feature::Noclip => "Noclip",
            Feature::InfiniteMoney => tr("Infinite money", "Argent infini"),
            Feature::InfiniteAmmo => tr("Infinite ammo", "Munitions infinies"),
            Feature::OneHitKill => tr("One-hit kill", "Un coup = un mort"),
            Feature::NoRecoil => tr("No recoil", "Sans recul"),
            Feature::RapidFire => tr("Rapid fire", "Tir rapide"),
            Feature::SlowMotion => tr("Game speed", "Vitesse du jeu"),
            Feature::LowGravity => tr("Low gravity", "Faible gravité"),
            Feature::JumpPack => "Jump pack",
            Feature::PauseZombies => tr("Pause zombies", "Pause des zombies"),
        }
    }

    /// Name used in the configuration, notably for keyboard shortcuts.
    pub fn key(self) -> &'static str {
        match self {
            Feature::GodMode => "god_mode",
            Feature::NoTarget => "no_target",
            Feature::Noclip => "noclip",
            Feature::InfiniteMoney => "infinite_money",
            Feature::InfiniteAmmo => "infinite_ammo",
            Feature::OneHitKill => "one_hit_kill",
            Feature::NoRecoil => "no_recoil",
            Feature::RapidFire => "rapid_fire",
            Feature::SlowMotion => "slow_motion",
            Feature::LowGravity => "low_gravity",
            Feature::JumpPack => "jump_pack",
            Feature::PauseZombies => "pause_zombies",
        }
    }

    /// True if the option changes the game's code: that is what its protection watches, so what must
    /// be turned off before leaving a match.
    pub fn patches_code(self) -> bool {
        matches!(self, Feature::NoRecoil | Feature::RapidFire | Feature::LowGravity)
    }

    /// True if the option was never seen working in game: all of them have been (no target and pause
    /// zombies on 2026-10-15).
    pub fn trial(self) -> bool {
        false
    }
}

/// A numeric setting, valid for one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tweak {
    Timescale,
    MoveSpeed,
    JumpHeight,
    CashScalar,
}

impl Tweak {
    pub const ALL: [Tweak; 4] = [Tweak::Timescale, Tweak::MoveSpeed, Tweak::JumpHeight, Tweak::CashScalar];

    /// Name used by the interface.
    pub fn key(self) -> &'static str {
        match self {
            Tweak::Timescale => "timescale",
            Tweak::MoveSpeed => "move_speed",
            Tweak::JumpHeight => "jump_height",
            Tweak::CashScalar => "cash_scalar",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tweak::Timescale => tr("Game speed", "Vitesse du jeu"),
            Tweak::MoveSpeed => tr("Movement speed", "Vitesse de déplacement"),
            Tweak::JumpHeight => tr("Jump height", "Hauteur de saut"),
            Tweak::CashScalar => tr("Money multiplier", "Multiplicateur d'argent"),
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Tweak::Timescale => tr("Slows down or speeds up the whole match. Back to 1 when the match ends.", "Ralentit ou accélère toute la partie. Revient à 1 en fin de partie."),
            Tweak::MoveSpeed => tr("How fast the player runs. Back to 190 when the match ends.", "Vitesse de course du joueur. Revient à 190 en fin de partie."),
            Tweak::JumpHeight => tr("How high a jump goes, set in the Zombies suit. Back to 39 when the match ends.", "Hauteur d'un saut, réglée dans la tenue Zombies. Revient à 39 en fin de partie."),
            Tweak::CashScalar => tr("Money earned per zombie killed, like Double Points but at the rate you pick. Lasts for the current match.", "Argent gagné par zombie tué, comme les points doubles mais au choix. Vaut pour la partie en cours."),
        }
    }

    /// Accepted bounds: smallest value, largest, and the step the interface offers.
    pub fn range(self) -> (f32, f32, f32) {
        match self {
            Tweak::Timescale => (0.1, 3.0, 0.05),
            Tweak::MoveSpeed => (50.0, 600.0, 10.0),
            Tweak::JumpHeight => (10.0, 400.0, 1.0),
            Tweak::CashScalar => (1.0, 10.0, 1.0),
        }
    }

    /// The game's original value.
    pub fn default(self) -> f32 {
        match self {
            Tweak::Timescale => 1.0,
            Tweak::MoveSpeed => MOVE_SPEED_DEFAULT as f32,
            Tweak::JumpHeight => 39.0,
            Tweak::CashScalar => 1.0,
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Tweak::Timescale | Tweak::CashScalar => "×",
            Tweak::MoveSpeed | Tweak::JumpHeight => "",
        }
    }

    /// True as long as the setting has not been seen working on the real game.
    pub fn trial(self) -> bool {
        matches!(self, Tweak::CashScalar)
    }
}

/// A one-off action: a single write, nothing is kept up afterwards.
///
/// The actions with a `script_code` are carried out by the iw7-mod script
/// (`iw7-mod/custom_scripts/cp/iw_helper.gsc`): the tool only writes the command into its mailbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OneShot {
    RefillAmmo,
    AddMoney,
    WeakenZombies,
    EndRound,
    ChargeCards,
    AddTickets,
    UpgradeWeapons,
    KillAll,
    OpenDoors,
    AllPerks,
    BoxWeapon,
    DropMaxAmmo,
    DropNuke,
    DropInstakill,
    DropDoubleMoney,
    DropFireSale,
    DropInfiniteAmmo,
    NextCharacter,
    Revive,
    ThirdPerson,
    ExplosiveBullets,
}

impl OneShot {
    pub const ALL: [OneShot; 21] = [
        OneShot::RefillAmmo,
        OneShot::AddMoney,
        OneShot::WeakenZombies,
        OneShot::EndRound,
        OneShot::ChargeCards,
        OneShot::AddTickets,
        OneShot::UpgradeWeapons,
        OneShot::KillAll,
        OneShot::OpenDoors,
        OneShot::AllPerks,
        OneShot::BoxWeapon,
        OneShot::DropMaxAmmo,
        OneShot::DropNuke,
        OneShot::DropInstakill,
        OneShot::DropDoubleMoney,
        OneShot::DropFireSale,
        OneShot::DropInfiniteAmmo,
        OneShot::NextCharacter,
        OneShot::Revive,
        OneShot::ThirdPerson,
        OneShot::ExplosiveBullets,
    ];

    /// Name used by the interface and in the configuration (keyboard shortcuts).
    pub fn key(self) -> &'static str {
        match self {
            OneShot::RefillAmmo => "refill_ammo",
            OneShot::AddMoney => "add_money",
            OneShot::WeakenZombies => "weaken_zombies",
            OneShot::EndRound => "end_round",
            OneShot::ChargeCards => "charge_cards",
            OneShot::AddTickets => "add_tickets",
            OneShot::UpgradeWeapons => "upgrade_weapons",
            OneShot::KillAll => "kill_all",
            OneShot::OpenDoors => "open_doors",
            OneShot::AllPerks => "all_perks",
            OneShot::BoxWeapon => "box_weapon",
            OneShot::DropMaxAmmo => "drop_max_ammo",
            OneShot::DropNuke => "drop_nuke",
            OneShot::DropInstakill => "drop_instakill",
            OneShot::DropDoubleMoney => "drop_double_money",
            OneShot::DropFireSale => "drop_fire_sale",
            OneShot::DropInfiniteAmmo => "drop_infinite_ammo",
            OneShot::NextCharacter => "next_character",
            OneShot::Revive => "revive",
            OneShot::ThirdPerson => "third_person",
            OneShot::ExplosiveBullets => "explosive_bullets",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OneShot::RefillAmmo => tr("Refill ammo", "Remplir les munitions"),
            OneShot::AddMoney => tr("Add $10,000", "Ajouter 10\u{a0}000\u{a0}$"),
            OneShot::WeakenZombies => tr("Weaken current zombies", "Affaiblir les zombies présents"),
            OneShot::EndRound => tr("End the round", "Finir la manche"),
            OneShot::ChargeCards => tr("Recharge Fate & Fortune cards", "Recharger les cartes Fate & Fortune"),
            OneShot::AddTickets => tr("Add 1,000 tickets", "Ajouter 1\u{a0}000 tickets"),
            OneShot::UpgradeWeapons => "Pack-a-Punch",
            OneShot::KillAll => tr("Kill every zombie", "Tuer tous les zombies"),
            OneShot::OpenDoors => tr("Open doors and turn on power", "Ouvrir les portes et allumer le courant"),
            OneShot::AllPerks => tr("All perks", "Tous les atouts"),
            OneShot::BoxWeapon => tr("Mystery weapon", "Arme mystère"),
            OneShot::DropMaxAmmo => tr("Max ammo bonus", "Bonus munitions max"),
            OneShot::DropNuke => tr("Nuke bonus", "Bonus nuke"),
            OneShot::DropInstakill => tr("Instakill bonus", "Bonus instakill"),
            OneShot::DropDoubleMoney => tr("Double money bonus", "Bonus argent double"),
            OneShot::DropFireSale => tr("Fire sale bonus", "Bonus soldes"),
            OneShot::DropInfiniteAmmo => tr("Infinite ammo bonus", "Bonus munitions infinies"),
            OneShot::NextCharacter => tr("Change character", "Changer de personnage"),
            OneShot::Revive => tr("Revive", "Se relever"),
            OneShot::ThirdPerson => tr("Third-person view", "Vue à la troisième personne"),
            OneShot::ExplosiveBullets => tr("Explosive bullets", "Balles explosives"),
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            OneShot::RefillAmmo => tr("Magazines to 511 and reserves to 9999, once.", "Chargeurs à 511 et réserves à 9999, une seule fois."),
            OneShot::AddMoney => tr("Once. The in-game counter catches up on your next gain or purchase.", "Une seule fois. L'affichage du jeu suit au prochain gain ou achat."),
            OneShot::WeakenZombies => tr("Health set to 1 for zombies already spawned. For later ones too, use One-hit kill.", "Vie à 1 pour ceux déjà apparus. Pour les suivants aussi : « Un coup = un mort »."),
            OneShot::EndRound => tr("The round counts as won and the next one starts. Zombies still alive stay.", "La manche compte comme gagnée et la suivante commence. Les zombies encore là restent."),
            OneShot::ChargeCards => tr("The card meter fills up on your next money gain.", "La jauge des cartes se remplit au prochain gain d'argent."),
            OneShot::AddTickets => tr("Zombies in Spaceland arcade tickets. The counter catches up on your next ticket won.", "Tickets d'arcade de Zombies in Spaceland. Le compteur suit au prochain ticket gagné."),
            OneShot::UpgradeWeapons => tr("Every player's weapons go up one level. Again for the next level.", "Les armes de tous les joueurs montent d'un niveau. Encore une fois pour le niveau suivant."),
            OneShot::KillAll => tr("Every zombie alive, at once. Bosses may resist.", "Tous les zombies en vie, d'un coup. Les boss peuvent résister."),
            OneShot::OpenDoors => tr("What the game's direct boss fight mode does: power, doors and paid passages.", "Ce que fait le mode combat de boss direct du jeu : courant, portes et passages payants."),
            OneShot::AllPerks => tr("Every perk of this map, for you.", "Tous les atouts de cette carte, pour vous."),
            OneShot::BoxWeapon => tr("A random weapon from this map's list, in place of the one in hand if you are full.", "Une arme au hasard dans la liste de la carte, à la place de celle en main si vous êtes plein."),
            OneShot::DropMaxAmmo | OneShot::DropNuke | OneShot::DropInstakill | OneShot::DropDoubleMoney | OneShot::DropFireSale | OneShot::DropInfiniteAmmo => {
                tr("Appears in front of you: walk into it. Not every map has every bonus.", "Apparaît devant vous : passez dessus. Toutes les cartes n'ont pas tous les bonus.")
            }
            OneShot::NextCharacter => tr(
                "The next of this map's characters, hidden ones included (Hasselhoff, Wyler, Kevin Smith, Pam Grier). Other maps' characters are not loaded.",
                "Le personnage suivant de cette carte, cachés compris (Hasselhoff, Wyler, Kevin Smith, Pam Grier). Ceux des autres cartes ne sont pas chargés.",
            ),
            OneShot::Revive => tr("Every player down gets back up, from last stand or the afterlife arcade.", "Tout joueur à terre se relève, en dernier recours ou depuis l'arcade de l'au-delà."),
            OneShot::ThirdPerson => tr("Switches between first and third-person view.", "Passe de la vue à la première personne à la troisième, et inversement."),
            OneShot::ExplosiveBullets => tr("Switches on or off: each shot explodes where it lands, away from you.", "S'active ou se coupe : chaque tir explose là où il touche, loin de vous."),
        }
    }

    /// Button text.
    pub fn button(self) -> &'static str {
        match self {
            OneShot::RefillAmmo => tr("Refill", "Remplir"),
            OneShot::AddMoney | OneShot::AddTickets => tr("Add", "Ajouter"),
            OneShot::WeakenZombies => tr("Weaken", "Affaiblir"),
            OneShot::EndRound => tr("End", "Finir"),
            OneShot::ChargeCards => tr("Recharge", "Recharger"),
            OneShot::UpgradeWeapons => tr("Upgrade", "Améliorer"),
            OneShot::KillAll => tr("Kill", "Tuer"),
            OneShot::OpenDoors => tr("Open", "Ouvrir"),
            OneShot::NextCharacter => tr("Change", "Changer"),
            OneShot::Revive => tr("Revive", "Relever"),
            OneShot::ThirdPerson | OneShot::ExplosiveBullets => tr("Switch", "Basculer"),
            OneShot::AllPerks | OneShot::BoxWeapon => tr("Get", "Obtenir"),
            OneShot::DropMaxAmmo | OneShot::DropNuke | OneShot::DropInstakill | OneShot::DropDoubleMoney | OneShot::DropFireSale | OneShot::DropInfiniteAmmo => {
                tr("Drop", "Faire apparaître")
            }
        }
    }

    /// True as long as the action has not been seen working on the real game. The other iw7-mod
    /// extras were seen working, from the console and from the window, on 2026-10-16.
    pub fn trial(self) -> bool {
        matches!(self, OneShot::Revive | OneShot::ThirdPerson | OneShot::ExplosiveBullets)
    }

    /// Number of the command for the iw7-mod script, for the actions it carries out. Keep in step
    /// with `run_command` in iw_helper.gsc.
    pub fn script_code(self) -> Option<i32> {
        Some(match self {
            OneShot::UpgradeWeapons => 1,
            OneShot::KillAll => 2,
            OneShot::OpenDoors => 3,
            OneShot::AllPerks => 4,
            OneShot::BoxWeapon => 5,
            OneShot::DropMaxAmmo => 10,
            OneShot::DropNuke => 11,
            OneShot::DropInstakill => 12,
            OneShot::DropDoubleMoney => 13,
            OneShot::DropFireSale => 14,
            OneShot::DropInfiniteAmmo => 15,
            OneShot::NextCharacter => 20,
            OneShot::Revive => 21,
            OneShot::ThirdPerson => 22,
            OneShot::ExplosiveBullets => 23,
            _ => return None,
        })
    }
}

fn not_in_match() -> &'static str {
    tr("not in a match", "pas en partie")
}

/// What was captured about the round variable when the fixed address pointed to it.
#[derive(Debug, Clone, PartialEq)]
struct RoundVariable {
    /// Name number of the field. The game reassigns it every time the scripts are loaded.
    name: u32,
    kind: u8,
    /// Gaps between this number and those of the nearest `level` fields. Numbers are given in the
    /// order the scripts are loaded: from one match to the next they may shift, but names introduced
    /// by the same script keep their gaps.
    neighbours: Vec<i32>,
}

/// How the round variable was found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoundSource {
    /// Among the `level` fields, under its known name number.
    Known,
    /// At the fixed address: first match after the game launched.
    Fixed,
    /// Elsewhere, under the same name number.
    Name,
    /// Under another name number, recognised by its neighbouring fields.
    Neighbours { shift: i32, matched: usize, of: usize },
}

pub struct Trainer {
    timescale: f32,
    money: i32,
    infinite_ammo: bool,
    infinite_money: bool,
    no_target: bool,
    one_hit_kill: bool,
    slow_motion: bool,
    jump_pack: bool,
    saved_position: Option<[f32; 3]>,
    /// Script variable of the round, captured when the fixed address is the right one.
    round_variable: Option<RoundVariable>,
    /// Original jump height and the one we wrote, while it is to be restored.
    jump_height: Option<(f32, f32)>,
    /// Movement speed was changed by this session and is not back to 190 yet.
    move_speed: bool,
    /// Native flags this session set itself and has not cleared yet.
    god_mode: bool,
    noclip: bool,
    /// The game is followed from its launch: no match can have been played without the tool seeing it.
    from_launch: bool,
    /// Number of matches seen by this session.
    matches: u32,
    /// Money multiplier kept in `level`, as long as it is not 1.
    cash_scalar: Option<i32>,
    /// "Pause zombies" on: number of zombies at a time the game had set, given back when turned off.
    paused_zombies: Option<i32>,
    /// Slot of each `level` field already found, so its whole partition is not read again on every pass.
    level_slots: std::collections::HashMap<u32, usize>,
}

impl Trainer {
    pub fn new(timescale: f32, money: i32) -> Self {
        Self {
            move_speed: false,
            god_mode: false,
            noclip: false,
            from_launch: false,
            matches: 0,
            timescale,
            money,
            infinite_ammo: false,
            infinite_money: false,
            no_target: false,
            one_hit_kill: false,
            slow_motion: false,
            jump_pack: false,
            saved_position: None,
            round_variable: None,
            jump_height: None,
            cash_scalar: None,
            paused_zombies: None,
            level_slots: std::collections::HashMap::new(),
        }
    }

    /// `level` integer named by its name, if it is within bounds: its slot and value.
    fn level_int(&mut self, p: &Process, name: u32, sane: &std::ops::RangeInclusive<i32>) -> Option<(usize, i32)> {
        let found = level_int(p, name, sane, self.level_slots.get(&name).copied());
        if let Some((index, _)) = found {
            self.level_slots.insert(name, index);
        }
        found
    }

    /// Gives the number of zombies at a time back to the game, if the value in place is still ours.
    fn resume_zombies(&mut self, p: &Process) {
        let Some(saved) = self.paused_zombies.take() else { return };
        if let Some((index, 0)) = self.level_int(p, MAX_ZOMBIES_NAME, &SANE_MAX_ZOMBIES) {
            write_child_int(p, index, saved);
        }
    }

    /// Puts the money multiplier back to 1, if the value in place is still ours.
    fn restore_cash_scalar(&mut self, p: &Process) {
        let Some(scalar) = self.cash_scalar.take() else { return };
        if let Some((index, _)) = self.level_int(p, CASH_SCALAR_NAME, &SANE_CASH_SCALAR).filter(|&(_, current)| current == scalar) {
            write_child_int(p, index, 1);
        }
    }

    /// Changes the current round number; returns the old one, and how the variable was found. The
    /// game takes it into account at the end of the round.
    pub fn set_round(&mut self, p: &Process, round: i32) -> Result<(i32, RoundSource), String> {
        if !in_match(p) {
            return Err(not_in_match().into());
        }
        if !SANE_ROUND.contains(&round) {
            return Err(tr("requested round out of range", "manche demandée hors bornes").into());
        }
        let (addr, source) = self.round_address(p)?;
        let previous = p.read_i32(addr).map_err(|_| tr("read refused", "lecture refusée"))?;
        p.write_bytes(addr, &round.to_le_bytes()).map_err(|_| tr("write refused", "écriture refusée"))?;
        Ok((previous, source))
    }

    /// Tells the trainer whether the game is followed from its launch. The caller has to know: it
    /// started the game itself, or found it right after its creation. Without that, the round's
    /// fixed slot is never taken for the round.
    pub fn followed_from_launch(mut self, yes: bool) -> Self {
        self.from_launch = yes;
        self
    }

    /// To call at the start of every match.
    pub fn match_started(&mut self) {
        self.matches += 1;
    }

    /// True as long as the round's fixed address can be trusted: it is the first match since the
    /// game launched, and a Zombies match (its suits are loaded). Elsewhere, the same slot holds
    /// another field, sometimes another `level` integer: captured on the game, in the third match of
    /// a session, the fixed slot was another `level` field.
    fn fixed_round_is_trustworthy(&self, p: &Process) -> bool {
        self.from_launch && self.matches <= 1 && !zombie_suits(p).is_empty()
    }

    /// Remembers the round variable while the fixed address points to it; to call during a match.
    /// Returns its name number and the number of neighbouring fields kept, the first time.
    pub fn learn_round_variable(&mut self, p: &Process) -> Option<(u32, usize)> {
        if self.round_variable.is_some() || !self.fixed_round_is_trustworthy(p) {
            return None;
        }
        let (level, var) = (level_object(p)?, read_child(p, round_fixed_index())?);
        // A round at zero teaches nothing: the slot could just as well be empty.
        if !(is_round_field(&var, level) && var.value > 0) {
            return None;
        }
        let neighbours = neighbour_deltas(object_fields(p, level).iter().map(|(_, field)| field.name), var.name);
        let count = neighbours.len();
        self.round_variable = Some(RoundVariable { name: var.name, kind: var.kind, neighbours });
        Some((var.name, count))
    }

    /// Address of the round variable for the current match.
    ///
    /// The fixed address is only right in the first match after the game launched: it is only
    /// trusted there (`fixed_round_is_trustworthy`), and the field it points to is then remembered
    /// with its neighbours. Everywhere else, the field is searched among the `level` fields: it is
    /// only accepted if its neighbours are found around it, and if it is the only one in that case.
    /// If the variable could never be captured, nothing is written.
    fn round_address(&mut self, p: &Process) -> Result<(usize, RoundSource), String> {
        let level = level_object(p).ok_or(tr("script level object not found", "objet « level » des scripts introuvable"))?;
        let fields = object_fields(p, level);
        // The known name is enough when it points to a plausible `level` integer: no condition on
        // when the tool was opened. The rest only serves if that name is missing (another map).
        if let Some(index) = known_round_field(&fields, level) {
            return Ok((CHILD_VARIABLES + index * CHILD_SIZE, RoundSource::Known));
        }
        if self.fixed_round_is_trustworthy(p) {
            self.learn_round_variable(p);
            let fixed = read_child(p, round_fixed_index()).filter(|var| is_round_field(var, level));
            // Before the first wave the round is 0 and not captured yet: the fixed slot is used anyway.
            if fixed.is_some_and(|var| self.round_variable.as_ref().is_none_or(|variable| variable.name == var.name)) {
                return Ok((ROUND, RoundSource::Fixed));
            }
        }
        let Some(variable) = &self.round_variable else {
            return Err(tr("round variable never captured: restart the game with the tool open, then play a first Zombies match", "variable de la manche jamais relevée : relancer le jeu avec l'outil ouvert, puis faire une première partie Zombies").into());
        };
        let of = variable.neighbours.len();
        let found = |index: usize, source: RoundSource| Ok((CHILD_VARIABLES + index * CHILD_SIZE, source));
        if of < ROUND_MIN_NEIGHBOURS {
            // Too few neighbours to recognise the field. Its number alone is not enough: from one match
            // to the next, it may point to another `level` integer. Nothing is written.
            return Err(trs(format!("round variable captured with too few neighbouring fields ({of}) to be recognised in this match"), format!("variable de la manche relevée avec trop peu de champs voisins ({of}) pour être reconnue dans cette partie")));
        }
        match align_round(variable, &fields) {
            Alignment::Found { index, name, .. } if name == variable.name => found(index, RoundSource::Name),
            Alignment::Found { index, name, matched } => found(index, RoundSource::Neighbours { shift: name as i32 - variable.name as i32, matched, of }),
            Alignment::Ambiguous => Err(tr("several fields look like the round variable: nothing was written", "plusieurs champs ressemblent à la variable de la manche : rien n'est écrit").into()),
            Alignment::Missing { best } => Err(trs(format!("round variable not found in this match (at best {best} of {of} neighbouring fields)"), format!("variable de la manche introuvable dans cette partie (au mieux {best} champs voisins sur {of})"))),
        }
    }

    /// Sets a numeric value; returns the old value and the one written (brought within bounds).
    pub fn set_tweak(&mut self, p: &Process, tweak: Tweak, value: f32) -> Result<(f32, f32), &'static str> {
        if !value.is_finite() {
            return Err(tr("invalid value", "valeur invalide"));
        }
        if !in_match(p) {
            return Err(not_in_match());
        }
        let (min, max, _) = tweak.range();
        let value = value.clamp(min, max);
        match tweak {
            Tweak::Timescale => {
                let previous = read_timescale(p).ok_or(tr("unexpected value at the game speed address", "valeur inattendue à l'adresse de la vitesse du jeu"))?;
                p.write_bytes(TIMESCALE, &value.to_le_bytes()).map_err(|_| tr("write refused", "écriture refusée"))?;
                // The "game speed" option shows the same setting: it turns on as soon as the speed is no longer normal.
                self.slow_motion = value != 1.0;
                Ok((previous, value))
            }
            Tweak::MoveSpeed => {
                let (addr, previous) = move_speed_dvar(p).ok_or(tr("movement speed setting not found, or unexpected value", "réglage de la vitesse de déplacement introuvable, ou valeur inattendue"))?;
                let speed = value.round() as i32;
                if !write_move_speed(p, addr, speed) {
                    return Err(tr("write refused", "écriture refusée"));
                }
                self.move_speed = speed != MOVE_SPEED_DEFAULT;
                Ok((previous as f32, speed as f32))
            }
            Tweak::JumpHeight => {
                // Only Zombies suits whose current jump height is plausible are returned.
                let suits = zombie_suits(p);
                let previous = suits.first().map(|&(_, height)| height).ok_or(tr("Zombies suit not found, or unexpected jump height", "tenue Zombies introuvable, ou hauteur de saut inattendue"))?;
                for &(suit, _) in &suits {
                    p.write_bytes(suit + SUIT_JUMP_HEIGHT, &value.to_le_bytes()).map_err(|_| tr("write refused", "écriture refusée"))?;
                }
                let original = self.jump_height.map_or(previous, |(original, _)| original);
                self.jump_height = (value != original).then_some((original, value));
                Ok((previous, value))
            }
            Tweak::CashScalar => {
                let (index, previous) = self.level_int(p, CASH_SCALAR_NAME, &SANE_CASH_SCALAR).ok_or(tr("money multiplier not found, or unexpected value", "multiplicateur d'argent introuvable, ou valeur inattendue"))?;
                let scalar = value.round() as i32;
                if !write_child_int(p, index, scalar) {
                    return Err(tr("write refused", "écriture refusée"));
                }
                // Double Points puts it back to 1 when it ends: it is kept up as long as it is not 1.
                self.cash_scalar = (scalar != 1).then_some(scalar);
                Ok((previous as f32, scalar as f32))
            }
        }
    }

    /// Current value of a setting, read from the game; `None` outside a match or if it is unreadable.
    pub fn tweak_value(&self, p: &Process, tweak: Tweak) -> Option<f32> {
        if !in_match(p) {
            return None;
        }
        match tweak {
            Tweak::Timescale => read_timescale(p),
            Tweak::MoveSpeed => move_speed_dvar(p).map(|(_, speed)| speed as f32),
            Tweak::JumpHeight => zombie_suits(p).first().map(|&(_, height)| height),
            Tweak::CashScalar => level_int(p, CASH_SCALAR_NAME, &SANE_CASH_SCALAR, self.level_slots.get(&CASH_SCALAR_NAME).copied()).map(|(_, scalar)| scalar as f32),
        }
    }

    /// Runs a one-off action; returns what was done, in a few words.
    pub fn run_action(&mut self, p: &Process, action: OneShot) -> Result<String, &'static str> {
        if !in_match(p) {
            return Err(not_in_match());
        }
        match action {
            OneShot::RefillAmmo => {
                let clips = CLIP_SLOTS.iter().filter(|&&slot| write_if_sane(p, BASE + slot, CLIP_VALUE, &SANE_AMMO)).count();
                let reserves = RESERVE_SLOTS.iter().filter(|&&slot| write_if_sane(p, BASE + slot, RESERVE_VALUE, &SANE_AMMO)).count();
                if clips + reserves == 0 {
                    return Err(tr("unexpected values at the ammo addresses", "valeurs inattendues aux adresses des munitions"));
                }
                Ok(trs(format!("{clips} magazines, {reserves} reserves"), format!("{clips} chargeurs, {reserves} réserves")))
            }
            OneShot::AddMoney => {
                let addr = money_address(p).ok_or(not_in_match())?;
                let current = p.read_i32(addr).ok().filter(|money| SANE_MONEY.contains(money)).ok_or(tr("unexpected value at the money address", "valeur inattendue à l'adresse de l'argent"))?;
                let total = (current + MONEY_GIFT).min(*SANE_MONEY.end());
                p.write_bytes(addr, &total.to_le_bytes()).map_err(|_| tr("write refused", "écriture refusée"))?;
                Ok(format!("{current} -> {total}"))
            }
            OneShot::WeakenZombies => match weaken_agents(p) {
                0 => Err(tr("no zombies around", "aucun zombie présent")),
                count => Ok(format!("{count} zombies")),
            },
            OneShot::EndRound => {
                // The game's spawn loop stops as soon as the deaths reach the round's target.
                let deaths = self.level_int(p, ENEMY_DEATHS_NAME, &SANE_DEATHS);
                let wanted = self.level_int(p, WAVE_DEATHS_NAME, &SANE_DEATHS);
                let (Some((index, deaths)), Some((_, wanted))) = (deaths, wanted) else {
                    return Err(tr("round counters not found, or unexpected values", "compteurs de la manche introuvables, ou valeurs inattendues"));
                };
                if deaths >= wanted {
                    return Err(tr("round already over, the next one is about to start", "manche déjà finie, la suivante va commencer"));
                }
                if !write_child_int(p, index, wanted) {
                    return Err(tr("write refused", "écriture refusée"));
                }
                Ok(trs(format!("{deaths} -> {wanted} zombies killed"), format!("{deaths} -> {wanted} zombies tués")))
            }
            OneShot::ChargeCards => {
                let fields = object_fields(p, player_object(p).ok_or(no_player_object())?);
                let field = |name: u32| fields.iter().find(|(_, var)| var.name == name).copied();
                let (Some((index, meter)), Some((_, full))) = (field(CARD_METER_NAME), field(CARD_METER_MAX_NAME)) else {
                    return Err(tr("card meter not found", "jauge des cartes introuvable"));
                };
                let sane = |var: &ChildVariable| number(var).filter(|value| SANE_CARD_METER.contains(value));
                let (Some(current), Some(full)) = (sane(&meter), sane(&full).filter(|&full| full > 0.0)) else {
                    return Err(tr("unexpected value in the card meter", "valeur inattendue dans la jauge des cartes"));
                };
                if current >= full {
                    return Err(tr("meter already full", "jauge déjà pleine"));
                }
                // The meter keeps its type: integer as long as it only received integers, float otherwise.
                let raw = if meter.kind == VAR_FLOAT { full.to_le_bytes() } else { (full.round() as i32).to_le_bytes() };
                p.write_bytes(CHILD_VARIABLES + index * CHILD_SIZE, &raw).map_err(|_| tr("write refused", "écriture refusée"))?;
                Ok(format!("{current} -> {full}"))
            }
            OneShot::AddTickets => {
                let player = player_object(p).ok_or(no_player_object())?;
                let (index, var) = object_fields(p, player).into_iter().find(|(_, var)| var.name == TICKETS_NAME).ok_or(tr("no tickets on this map", "pas de tickets sur cette carte"))?;
                if var.kind != VAR_INTEGER || !SANE_TICKETS.contains(&var.value) {
                    return Err(tr("unexpected ticket count", "valeur inattendue dans les tickets"));
                }
                let total = (var.value + TICKET_GIFT).min(*SANE_TICKETS.end());
                if !write_child_int(p, index, total) {
                    return Err(tr("write refused", "écriture refusée"));
                }
                Ok(format!("{} -> {total}", var.value))
            }
            OneShot::UpgradeWeapons
            | OneShot::KillAll
            | OneShot::OpenDoors
            | OneShot::AllPerks
            | OneShot::BoxWeapon
            | OneShot::DropMaxAmmo
            | OneShot::DropNuke
            | OneShot::DropInstakill
            | OneShot::DropDoubleMoney
            | OneShot::DropFireSale
            | OneShot::DropInfiniteAmmo
            | OneShot::NextCharacter
            | OneShot::Revive
            | OneShot::ThirdPerson
            | OneShot::ExplosiveBullets => self.send_to_script(p, action),
        }
    }

    /// Writes the action's command into the iw7-mod script's mailbox. Only on iw7-mod: the Steam game
    /// never loads the script, and nothing is written to it for this.
    fn send_to_script(&mut self, p: &Process, action: OneShot) -> Result<String, &'static str> {
        let code = action.script_code().ok_or(tr("not a script command", "pas une commande du script"))?;
        if !p.name.eq_ignore_ascii_case(config::MOD_CLIENT) {
            return Err(tr("iw7-mod only", "iw7-mod uniquement"));
        }
        let (index, current) = self
            .level_int(p, SCRIPT_MAILBOX_NAME, &SANE_MAILBOX)
            .ok_or(tr("iw7-mod script not loaded, see the README", "script iw7-mod non chargé, voir le README"))?;
        if current != SCRIPT_READY {
            return Err(tr("the script is busy, try again", "le script est occupé, réessayez"));
        }
        if !write_child_int(p, index, SCRIPT_READY + code) {
            return Err(tr("write refused", "écriture refusée"));
        }
        Ok(tr("sent to the script", "envoyé au script").to_string())
    }

    /// To call when the match ends: restores what does not belong to the match itself (engine
    /// speed, suit setting) and would otherwise stay in place in the menus.
    pub fn match_ended(&mut self, p: &Process) {
        if self.slow_motion && write_timescale(p, 1.0) {
            self.slow_motion = false;
        }
        self.restore_jump_height(p);
        self.restore_move_speed(p);
        // The player's flags and the `level` fields go away with the match: nothing of ours is left
        // in the game.
        self.god_mode = false;
        self.noclip = false;
        self.cash_scalar = None;
        self.paused_zombies = None;
        self.level_slots.clear();
    }

    /// What this session changed in the game and has not restored yet: what must be undone if the
    /// tool is closed now and reopened on the same game.
    pub fn pending(&self) -> Pending {
        Pending {
            god_mode: self.god_mode,
            noclip: self.noclip,
            jump_pack: self.jump_pack,
            no_target: self.no_target,
            timescale: self.slow_motion,
            move_speed: self.move_speed,
            jump_height: self.jump_height.map(|(original, _)| original),
            cash_scalar: self.cash_scalar.is_some(),
            paused_zombies: self.paused_zombies,
        }
    }

    /// Restores the original jump height, wherever the value we wrote is still in place.
    fn restore_jump_height(&mut self, p: &Process) {
        let Some((original, written)) = self.jump_height.take() else { return };
        for (suit, height) in zombie_suits(p) {
            if height == written {
                let _ = p.write_bytes(suit + SUIT_JUMP_HEIGHT, &original.to_le_bytes());
            }
        }
    }

    /// Puts movement speed back to 190 if this session changed it. The setting outlives the match:
    /// without this, it would stay in place in the menus and in the next match.
    fn restore_move_speed(&mut self, p: &Process) {
        if !self.move_speed {
            return;
        }
        self.move_speed = match move_speed_dvar(p) {
            Some((addr, speed)) => speed != MOVE_SPEED_DEFAULT && !write_move_speed(p, addr, MOVE_SPEED_DEFAULT),
            // Setting not found: nothing to restore, and nothing to retry.
            None => false,
        };
    }

    /// Toggles an option; returns its new state.
    pub fn toggle(&mut self, p: &Process, feature: Feature) -> Result<bool, &'static str> {
        match feature {
            Feature::GodMode => flip_native(p, host_entity_flags(p), FL_GODMODE).inspect(|on| self.god_mode = *on),
            Feature::NoTarget => {
                if self.no_target {
                    self.no_target = false;
                    let _ = set_ignore_me(p, false);
                } else if !in_match(p) {
                    return Err(not_in_match());
                } else if set_ignore_me(p, true).is_some() {
                    self.no_target = true;
                } else {
                    return Err(tr("player structure not found, or unexpected value", "structure du joueur introuvable, ou valeur inattendue"));
                }
                Ok(self.no_target)
            }
            Feature::Noclip => flip_native(p, host_client_flags(p), CLIENT_NOCLIP).inspect(|on| self.noclip = *on),
            Feature::InfiniteAmmo => flip_frozen(p, &mut self.infinite_ammo),
            Feature::OneHitKill => flip_frozen(p, &mut self.one_hit_kill),
            Feature::InfiniteMoney => {
                if self.infinite_money {
                    self.infinite_money = false;
                } else if !in_match(p) {
                    return Err(not_in_match());
                } else if write_if_sane(p, money_address(p).ok_or(not_in_match())?, self.money, &SANE_MONEY) {
                    self.infinite_money = true;
                } else {
                    return Err(tr("unexpected value at the money address", "valeur inattendue à l'adresse de l'argent"));
                }
                Ok(self.infinite_money)
            }
            Feature::SlowMotion => {
                let target = if self.slow_motion { 1.0 } else { self.timescale };
                // Slowing down is only possible in a match; going back to normal speed always is.
                if (!self.slow_motion && !in_match(p)) || !write_timescale(p, target) {
                    return Err(not_in_match());
                }
                self.slow_motion = !self.slow_motion;
                Ok(self.slow_motion)
            }
            Feature::NoRecoil => flip_code(p, &NO_RECOIL),
            Feature::RapidFire => flip_code(p, &RAPID_FIRE),
            Feature::LowGravity => flip_code(p, &LOW_GRAVITY),
            Feature::JumpPack => {
                if host_client(p).is_none() || !in_match(p) {
                    return Err(not_in_match());
                }
                let (offset, _, highest) = DOUBLE_JUMP_FORBIDDEN;
                if !p.read_i32(HOST_CLIENT + offset).is_ok_and(|word| (0..=highest).contains(&word)) {
                    return Err(tr("unexpected value at the player movement address", "valeur inattendue à l'adresse des mouvements du joueur"));
                }
                // Turning the option on lifts the ban; turning it off sets it back.
                set_moves_forbidden(p, self.jump_pack);
                self.jump_pack = !self.jump_pack;
                Ok(self.jump_pack)
            }
            Feature::PauseZombies => {
                if self.paused_zombies.is_some() {
                    self.resume_zombies(p);
                    return Ok(false);
                }
                if !in_match(p) {
                    return Err(not_in_match());
                }
                let (index, current) = self.level_int(p, MAX_ZOMBIES_NAME, &SANE_MAX_ZOMBIES).ok_or(tr("zombie count not found, or unexpected value", "nombre de zombies introuvable, ou valeur inattendue"))?;
                if !write_child_int(p, index, 0) {
                    return Err(tr("write refused", "écriture refusée"));
                }
                self.paused_zombies = Some(current);
                Ok(true)
            }
        }
    }

    /// Real state of the option: read from the game for flags and patches.
    pub fn is_on(&self, p: &Process, feature: Feature) -> bool {
        let flag = |addr: Option<usize>, bit: i32| matches!(addr.map(|a| p.read_i32(a)), Some(Ok(v)) if v & bit != 0);
        let site = |site: &CodeSite| site.state(p) == SiteState::Patched;
        match feature {
            Feature::GodMode => flag(host_entity_flags(p), FL_GODMODE),
            Feature::NoTarget => self.no_target,
            Feature::Noclip => flag(host_client_flags(p), CLIENT_NOCLIP),
            Feature::InfiniteAmmo => self.infinite_ammo,
            Feature::OneHitKill => self.one_hit_kill,
            Feature::InfiniteMoney => self.infinite_money,
            Feature::SlowMotion => self.slow_motion,
            Feature::NoRecoil => site(&NO_RECOIL),
            Feature::RapidFire => site(&RAPID_FIRE),
            Feature::LowGravity => site(&LOW_GRAVITY),
            Feature::JumpPack => self.jump_pack,
            Feature::PauseZombies => self.paused_zombies.is_some(),
        }
    }

    /// Saves the player's current position.
    pub fn save_position(&mut self, p: &Process) -> Result<[f32; 3], &'static str> {
        let position = read_position(p).ok_or(not_in_match())?;
        self.saved_position = Some(position);
        Ok(position)
    }

    pub fn saved_position(&self) -> Option<[f32; 3]> {
        self.saved_position
    }

    /// Brings the player back to the saved position. Only a position where they really stood is ever written.
    pub fn teleport(&self, p: &Process) -> Result<[f32; 3], &'static str> {
        let position = self.saved_position.ok_or(tr("no saved position", "aucune position enregistrée"))?;
        if read_position(p).is_none() || !write_position(p, position) {
            return Err(not_in_match());
        }
        Ok(position)
    }

    /// Rewrites the values the game keeps overwriting. To call several times per second.
    pub fn apply_frozen_values(&mut self, p: &Process) {
        let held = self.cash_scalar.is_some() || self.paused_zombies.is_some();
        if !(self.infinite_ammo || self.infinite_money || self.no_target || self.one_hit_kill || self.jump_pack || held) || !in_match(p) {
            return;
        }
        if self.paused_zombies.is_some() {
            // Each wave sets its own number: it is remembered for resuming, then set back to zero.
            if let Some((index, wave)) = self.level_int(p, MAX_ZOMBIES_NAME, &SANE_MAX_ZOMBIES).filter(|&(_, value)| value != 0) {
                self.paused_zombies = Some(wave);
                write_child_int(p, index, 0);
            }
        }
        if let Some(scalar) = self.cash_scalar {
            // The end of Double Points puts it back to 1.
            if let Some((index, _)) = self.level_int(p, CASH_SCALAR_NAME, &SANE_CASH_SCALAR).filter(|&(_, value)| value != scalar) {
                write_child_int(p, index, scalar);
            }
        }
        if self.jump_pack {
            // The mode's scripts may set the ban again: it is lifted again.
            set_moves_forbidden(p, false);
        }
        if self.no_target {
            // The scripts reset the field to false (after a revive, for example): it is set again.
            let _ = set_ignore_me(p, true);
        }
        if self.infinite_money {
            if let Some(addr) = money_address(p) {
                write_if_sane(p, addr, self.money, &SANE_MONEY);
            }
        }
        if self.infinite_ammo {
            for slot in CLIP_SLOTS {
                write_if_sane(p, BASE + slot, CLIP_VALUE, &SANE_AMMO);
            }
            for slot in RESERVE_SLOTS {
                write_if_sane(p, BASE + slot, RESERVE_VALUE, &SANE_AMMO);
            }
        }
        if self.one_hit_kill {
            weaken_agents(p);
        }
    }

    /// Turns every option off and restores what was changed. Leaving a match with options still on
    /// tends to crash the game.
    pub fn all_off(&mut self, p: &Process) {
        self.infinite_ammo = false;
        self.infinite_money = false;
        self.one_hit_kill = false;
        if self.slow_motion && write_timescale(p, 1.0) {
            self.slow_motion = false;
        }
        self.restore_jump_height(p);
        self.restore_move_speed(p);
        self.restore_cash_scalar(p);
        self.resume_zombies(p);
        if self.jump_pack {
            self.jump_pack = false;
            if in_match(p) {
                set_moves_forbidden(p, true);
            }
        }
        if self.no_target {
            self.no_target = false;
            let _ = set_ignore_me(p, false);
        }
        if let Some(flags) = host_entity_flags(p) {
            clear_bits(p, flags, FL_GODMODE);
        }
        if let Some(flags) = host_client_flags(p) {
            clear_bits(p, flags, CLIENT_NOCLIP);
        }
        self.god_mode = false;
        self.noclip = false;
        for site in &CODE_SITES {
            site.restore(p);
        }
    }

    /// Restores what a previous session of the tool left in an already running game. To call only
    /// once, when the game version has just been recognised and before any user action. Returns what
    /// was restored, so nothing if the game was clean.
    ///
    /// Only two things are restored regardless, because their exact value can only come from the
    /// tool: one of the three instructions it changes itself, still in its changed state, and the old
    /// "no target" health. Everything else (native flags, game speed, jump height) is only restored if
    /// `pending` names it: the game can set these values by itself, and they are then not ours. Each
    /// value is read again before being written.
    pub fn reset_leftovers(&mut self, p: &Process, pending: &Pending) -> Vec<&'static str> {
        let mut restored = Vec::new();
        for site in &OWN_CODE_SITES {
            if site.state(p) == SiteState::Patched && p.write_bytes(site.addr, site.original).is_ok() {
                restored.push(site.name());
            }
        }
        if p.read_i32(PLAYER_HEALTH).ok() == Some(NO_TARGET_HEALTH) && host_client(p).is_some() {
            restore_health(p);
            restored.push(Feature::NoTarget.label());
        }
        if pending.timescale && read_timescale(p).is_some_and(|speed| speed != 1.0) && write_timescale(p, 1.0) {
            restored.push(Tweak::Timescale.label());
        }
        if pending.no_target && host_client(p).is_some() && set_ignore_me(p, false) == Some(true) {
            restored.push(Feature::NoTarget.label());
        }
        if pending.move_speed && move_speed_dvar(p).is_some_and(|(addr, speed)| speed != MOVE_SPEED_DEFAULT && write_move_speed(p, addr, MOVE_SPEED_DEFAULT)) {
            restored.push(Tweak::MoveSpeed.label());
        }
        if let Some(original) = pending.jump_height.filter(|height| SANE_JUMP_HEIGHT.contains(height)) {
            let mut written = false;
            for (suit, height) in zombie_suits(p) {
                written |= height != original && p.write_bytes(suit + SUIT_JUMP_HEIGHT, &original.to_le_bytes()).is_ok();
            }
            if written {
                restored.push(Tweak::JumpHeight.label());
            }
        }
        // The `level` fields: money multiplier back to 1, and zombies given back to the game if they are
        // still paused (otherwise the round would never end).
        if pending.cash_scalar && level_int(p, CASH_SCALAR_NAME, &SANE_CASH_SCALAR, None).is_some_and(|(index, scalar)| scalar != 1 && write_child_int(p, index, 1)) {
            restored.push(Tweak::CashScalar.label());
        }
        if let Some(saved) = pending.paused_zombies.filter(|saved| SANE_MAX_ZOMBIES.contains(saved)) {
            if level_int(p, MAX_ZOMBIES_NAME, &SANE_MAX_ZOMBIES, None).is_some_and(|(index, current)| current == 0 && write_child_int(p, index, saved)) {
                restored.push(Feature::PauseZombies.label());
            }
        }
        // The movement bans the tool had lifted are set back, during a match only.
        if pending.jump_pack && host_client_flags(p).is_some() && set_moves_forbidden(p, true) {
            restored.push(Feature::JumpPack.label());
        }
        // The player's flags only make sense during a match.
        let flags = [
            (pending.god_mode, host_entity_flags(p), FL_GODMODE, Feature::GodMode),
            (pending.noclip, host_client_flags(p), CLIENT_NOCLIP, Feature::Noclip),
        ];
        for (ours, addr, bit, feature) in flags {
            let Some(addr) = addr.filter(|_| ours) else { continue };
            if matches!(p.read_i32(addr), Ok(current) if current & bit != 0 && p.write_bytes(addr, &(current & !bit).to_le_bytes()).is_ok()) {
                restored.push(feature.label());
            }
        }
        restored
    }
}

const fn agent_health(slot: usize) -> usize {
    G_ENTITIES + (FIRST_AGENT + slot) * ENTITY_SIZE + ENTITY_HEALTH
}

/// Sets the health of the zombies present to 1; returns how many were affected.
fn weaken_agents(p: &Process) -> usize {
    let mut weakened = 0;
    for slot in 0..AGENT_SLOTS {
        let addr = agent_health(slot);
        // Neither empty slots (0) nor already weakened zombies (1) are touched.
        if matches!(p.read_i32(addr), Ok(hp) if hp > 1 && SANE_HEALTH.contains(&hp)) && p.write_bytes(addr, &1i32.to_le_bytes()).is_ok() {
            weakened += 1;
        }
    }
    weakened
}

/// The host player's `gclient_s`, read from their entity. The pointer is only set during a match.
fn host_client(p: &Process) -> Option<usize> {
    let client = p.read_usize(G_ENTITIES + ENTITY_CLIENT).ok()?;
    (client == HOST_CLIENT).then_some(client)
}

/// True during a match: entity 0 is linked to the host player and their health is plausible.
pub fn in_match(p: &Process) -> bool {
    // The old "no target" negative health is ours: it must not make the match look over.
    host_client(p).is_some() && matches!(p.read_i32(PLAYER_HEALTH), Ok(hp) if SANE_HEALTH.contains(&hp) || hp == NO_TARGET_HEALTH)
}

/// Sets or clears `ignoreme` for the host player. Returns `Some(true)` if the field was changed,
/// `Some(false)` if it already had the wanted value, `None` if it could not be read or written.
/// Nothing is written if the structure is not the player's (its first word must point to its
/// entity) or if the field is neither 0 nor 1.
fn set_ignore_me(p: &Process, on: bool) -> Option<bool> {
    let sentient = p.read_usize(G_ENTITIES + ENTITY_SENTIENT).ok().filter(|addr| (BASE..IMAGE_END).contains(addr))?;
    if p.read_usize(sentient).ok() != Some(G_ENTITIES) {
        return None;
    }
    let addr = sentient + SENTIENT_IGNORE_ME;
    match p.read_u8(addr).ok()? {
        current if current == u8::from(on) => Some(false),
        0 | 1 => p.write_bytes(addr, &[u8::from(on)]).is_ok().then_some(true),
        _ => None,
    }
}

/// Restores the player's health left negative by the old "no target" method.
fn restore_health(p: &Process) {
    if p.read_i32(PLAYER_HEALTH).ok() == Some(NO_TARGET_HEALTH) {
        let _ = p.write_bytes(PLAYER_HEALTH, &DEFAULT_HEALTH.to_le_bytes());
    }
}

/// A field of a script object, as stored in `childVariableValue`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChildVariable {
    value: i32,
    kind: u8,
    /// Id of the field's name, on 24 bits.
    name: u32,
    parent: u16,
}

fn parse_child(raw: &[u8]) -> ChildVariable {
    let word = |at: usize| u16::from_le_bytes([raw[at], raw[at + 1]]);
    ChildVariable { value: i32::from_le_bytes(raw[..4].try_into().unwrap()), kind: raw[10], name: u32::from(raw[11]) | (u32::from(word(12)) << 8), parent: word(14) }
}

fn read_child(p: &Process, index: usize) -> Option<ChildVariable> {
    if index >= CHILD_COUNT {
        return None;
    }
    let mut raw = [0u8; CHILD_SIZE];
    p.read_bytes(CHILD_VARIABLES + index * CHILD_SIZE, &mut raw).ok()?;
    Some(parse_child(&raw))
}

/// Every field of a script object (`level`, a player…), with the number of its slot. The part of
/// the table where this object keeps its fields is read in one block, then filtered by parent:
/// this assumes nothing about how the engine links fields together.
fn object_fields(p: &Process, object: u32) -> Vec<(usize, ChildVariable)> {
    let first = CHILD_PARTITION * (object as usize & 3);
    let raw = p.read_lossy(CHILD_VARIABLES + first * CHILD_SIZE, CHILD_PARTITION * CHILD_SIZE);
    raw.as_chunks::<CHILD_SIZE>().0.iter()
        .map(|raw| parse_child(raw))
        .enumerate()
        .filter(|(_, var)| is_field_of(var, object))
        .map(|(slot, var)| (first + slot, var))
        .collect()
}

fn is_field_of(var: &ChildVariable, object: u32) -> bool {
    var.parent == object as u16 && var.kind != VAR_UNDEFINED && var.kind < VAR_FREE
}

/// `level` integer named by its name, if it is within bounds: its slot and value. The `cached`
/// slot, if it still points to this field, avoids reading the whole partition again.
fn level_int(p: &Process, name: u32, sane: &std::ops::RangeInclusive<i32>, cached: Option<usize>) -> Option<(usize, i32)> {
    let level = level_object(p)?;
    let still_there = cached.and_then(|index| read_child(p, index).map(|var| (index, var))).filter(|(_, var)| var.name == name && is_field_of(var, level));
    let (index, var) = still_there.or_else(|| object_fields(p, level).into_iter().find(|(_, var)| var.name == name))?;
    (var.kind == VAR_INTEGER && sane.contains(&var.value)).then_some((index, var.value))
}

/// Writes an integer into the slot of a script field already read.
fn write_child_int(p: &Process, index: usize, value: i32) -> bool {
    p.write_bytes(CHILD_VARIABLES + index * CHILD_SIZE, &value.to_le_bytes()).is_ok()
}

/// Value of an integer or float field.
fn number(var: &ChildVariable) -> Option<f32> {
    match var.kind {
        VAR_INTEGER => Some(var.value as f32),
        VAR_FLOAT => Some(f32::from_bits(var.value as u32)),
        _ => None,
    }
}

fn no_player_object() -> &'static str {
    tr("player script object not found", "objet de script du joueur introuvable")
}

/// Script object of the host player: among the objects pointing to entity 0, the one with an
/// integer `maxcurrency` field, which the mode's scripts give every player. Captured in a match on
/// 2026-10-14: this object does hold the player's maximum money, tickets and card meter.
fn player_object(p: &Process) -> Option<u32> {
    let table = p.read_lossy(SCR_VAR_GLOB, OBJECT_COUNT as usize * 8);
    table.as_chunks::<8>().0.iter()
        .enumerate()
        .filter(|(_, raw)| raw[4] == VAR_ENTITY && raw[2] == 0 && raw[3] == 0)
        .map(|(object, _)| object as u32)
        .find(|&object| object_fields(p, object).iter().any(|(_, var)| var.name == MAX_CURRENCY_NAME && var.kind == VAR_INTEGER))
}

/// True if this field can be the round: a `level` integer, within the bounds of a round number.
fn is_round_field(var: &ChildVariable, level: u32) -> bool {
    var.parent == level as u16 && var.kind == VAR_INTEGER && SANE_ROUND.contains(&var.value)
}

/// Slot of the `level` field carrying the round's known name, if it really is a plausible integer.
fn known_round_field(fields: &[(usize, ChildVariable)], level: u32) -> Option<usize> {
    fields.iter().find(|(_, var)| var.name == ROUND_NAME && is_round_field(var, level)).map(|&(index, _)| index)
}

/// Gaps between `anchor` and the nearest name numbers, from nearest to farthest.
fn neighbour_deltas(names: impl Iterator<Item = u32>, anchor: u32) -> Vec<i32> {
    let mut deltas: Vec<i32> = names.filter(|&name| name != anchor).map(|name| name as i32 - anchor as i32).collect();
    deltas.sort_by_key(|delta| (delta.unsigned_abs(), *delta));
    deltas.dedup();
    deltas.truncate(ROUND_NEIGHBOURS);
    deltas
}

#[derive(Debug, PartialEq)]
enum Alignment {
    /// A single `level` integer field has the captured neighbours around it.
    Found { index: usize, name: u32, matched: usize },
    /// Several fields fit: none is kept.
    Ambiguous,
    /// No field fits; `best` is the highest number of neighbours found.
    Missing { best: usize },
}

/// Looks among the `level` fields for the one with the same neighbours as the captured variable:
/// for each plausible integer, counts the captured gaps that land on an existing field. Two thirds
/// are needed, and a single field must reach them.
fn align_round(variable: &RoundVariable, fields: &[(usize, ChildVariable)]) -> Alignment {
    let of = variable.neighbours.len();
    if of < ROUND_MIN_NEIGHBOURS {
        return Alignment::Missing { best: 0 };
    }
    let names: std::collections::HashSet<u32> = fields.iter().map(|(_, var)| var.name).collect();
    let required = (of * 2).div_ceil(3);
    let (mut best, mut hits) = (0, Vec::new());
    for &(index, var) in fields.iter().filter(|(_, var)| var.kind == variable.kind && SANE_ROUND.contains(&var.value)) {
        let matched = variable.neighbours.iter().filter(|&&delta| u32::try_from(var.name as i32 + delta).is_ok_and(|name| names.contains(&name))).count();
        best = best.max(matched);
        if matched >= required {
            hits.push(Alignment::Found { index, name: var.name, matched });
        }
    }
    match hits.len() {
        0 => Alignment::Missing { best },
        1 => hits.remove(0),
        _ => Alignment::Ambiguous,
    }
}

/// Slot of `childVariableValue` that the round's fixed address points to.
const fn round_fixed_index() -> usize {
    (ROUND - CHILD_VARIABLES) / CHILD_SIZE
}

/// Id of the `level` object, or `None` if no script is running.
pub(crate) fn level_object(p: &Process) -> Option<u32> {
    p.read_i32(LEVEL_ENTITY_ID).ok().map(|id| id as u32).filter(|id| (1..OBJECT_COUNT).contains(id))
}

/// Address of the host player's money, or `None` if no server is running.
fn money_address(p: &Process) -> Option<usize> {
    server_clients(p).map(|clients| clients + SERVER_CLIENT_MONEY)
}

/// Address of the host player's `client_t` (the first in the array), or `None` if no server is running.
fn server_clients(p: &Process) -> Option<usize> {
    p.read_usize(SVS_CLIENTS).ok().filter(|&clients| clients > 0x1_0000 && clients < 0x7FFF_0000_0000)
}

/// Address of `g_entities[0].flags`, or `None` outside a match.
fn host_entity_flags(p: &Process) -> Option<usize> {
    in_match(p).then_some(G_ENTITIES + ENTITY_FLAGS)
}

/// Address of `g_entities[0].client->flags`, or `None` outside a match.
fn host_client_flags(p: &Process) -> Option<usize> {
    in_match(p).then_some(HOST_CLIENT + CLIENT_FLAGS)
}

/// Sets or lifts, for the host player, the bans of `FORBIDDEN_MOVES` and the climbing ban. True if
/// a word was changed. An unreadable word, or one that does not look like what was captured, is
/// left as is: the other moves are handled anyway.
fn set_moves_forbidden(p: &Process, forbidden: bool) -> bool {
    let mut changed = false;
    for (offset, bit, highest) in FORBIDDEN_MOVES {
        let addr = HOST_CLIENT + offset;
        let Some(current) = p.read_i32(addr).ok().filter(|word| (0..=highest).contains(word)) else { continue };
        let word = if forbidden { current | bit } else { current & !bit };
        changed |= word != current && p.write_bytes(addr, &word.to_le_bytes()).is_ok();
    }
    let (addr, mut byte) = (HOST_CLIENT + MANTLE_FORBIDDEN_BYTE, [0u8; 1]);
    if p.read_bytes(addr, &mut byte).is_ok() {
        let wanted = if forbidden { byte[0] | MANTLE_FORBIDDEN_BIT } else { byte[0] & !MANTLE_FORBIDDEN_BIT };
        changed |= wanted != byte[0] && p.write_bytes(addr, &[wanted]).is_ok();
    }
    changed
}

/// Flips `bits` in the flag word at `addr`; returns whether they are now set.
fn flip_bits(p: &Process, addr: usize, bits: i32) -> Option<bool> {
    let flags = p.read_i32(addr).ok()? ^ bits;
    p.write_bytes(addr, &flags.to_le_bytes()).ok()?;
    Some(flags & bits != 0)
}

fn clear_bits(p: &Process, addr: usize, bits: i32) {
    if let Ok(flags) = p.read_i32(addr) {
        if flags & bits != 0 {
            let _ = p.write_bytes(addr, &(flags & !bits).to_le_bytes());
        }
    }
}

/// Toggles a native engine flag, as its own cheat commands do.
fn flip_native(p: &Process, flags: Option<usize>, bit: i32) -> Result<bool, &'static str> {
    flags.and_then(|addr| flip_bits(p, addr, bit)).ok_or(not_in_match())
}

/// Toggles a code patch. Applying it is refused outside a match; removing it is always possible.
fn flip_code(p: &Process, site: &CodeSite) -> Result<bool, &'static str> {
    if site.state(p) == SiteState::Original && !in_match(p) {
        return Err(not_in_match());
    }
    site.toggle(p)
}

/// Toggles an option kept up by periodic rewriting; refused outside a match.
fn flip_frozen(p: &Process, state: &mut bool) -> Result<bool, &'static str> {
    if !*state && !in_match(p) {
        return Err(not_in_match());
    }
    *state = !*state;
    Ok(*state)
}

/// Only writes `value` if the address currently holds a value in `sane`; returns whether it now holds `value`.
fn write_if_sane(p: &Process, addr: usize, value: i32, sane: &std::ops::RangeInclusive<i32>) -> bool {
    match p.read_i32(addr) {
        Ok(current) if current == value => true,
        Ok(current) if sane.contains(&current) => p.write_bytes(addr, &value.to_le_bytes()).is_ok(),
        _ => false,
    }
}

/// Current game speed, if it really is one.
fn read_timescale(p: &Process) -> Option<f32> {
    p.read_f32(TIMESCALE).ok().filter(|current| SANE_TIMESCALE.contains(current))
}

fn write_timescale(p: &Process, value: f32) -> bool {
    read_timescale(p).is_some() && p.write_bytes(TIMESCALE, &value.to_le_bytes()).is_ok()
}

/// X, Y, Z of the host player, or `None` outside a match or if the values are not coordinates.
fn read_position(p: &Process) -> Option<[f32; 3]> {
    if !in_match(p) {
        return None;
    }
    let mut position = [0f32; 3];
    for (axis, value) in position.iter_mut().enumerate() {
        *value = p.read_f32(HOST_CLIENT + CLIENT_ORIGIN + axis * 4).ok()?;
    }
    position.iter().all(|v| v.is_finite() && v.abs() < 1.0e6).then_some(position)
}

fn write_position(p: &Process, position: [f32; 3]) -> bool {
    let bytes: Vec<u8> = position.iter().flat_map(|v| v.to_le_bytes()).collect();
    p.write_bytes(HOST_CLIENT + CLIENT_ORIGIN, &bytes).is_ok()
}

/// A soft two-note chime, rising for "on", falling for "off" or "refused": the only feedback
/// possible in fullscreen. Made once, played without waiting, at a low volume.
pub fn beep(on: bool) {
    static CHIMES: std::sync::OnceLock<[Vec<u8>; 2]> = std::sync::OnceLock::new();
    // E5 then B5, and D5 then G4.
    let chimes = CHIMES.get_or_init(|| [chime(&[659.25, 987.77]), chime(&[587.33, 392.0])]);
    let wav = &chimes[if on { 0 } else { 1 }];
    unsafe { PlaySoundW(wav.as_ptr().cast(), std::ptr::null_mut(), SND_MEMORY | SND_ASYNC | SND_NODEFAULT) };
}

const CHIME_RATE: u32 = 44_100;
const CHIME_NOTE_MS: usize = 70;
const CHIME_VOLUME: f32 = 0.2;

/// A WAV file in memory, 16-bit mono: one short note per frequency, with a soft attack, a fading
/// tail and a short release so that nothing clicks.
fn chime(notes: &[f32]) -> Vec<u8> {
    let rate = CHIME_RATE as f32;
    let length = CHIME_RATE as usize * CHIME_NOTE_MS / 1000;
    let samples: Vec<i16> = notes
        .iter()
        .flat_map(|&frequency| {
            (0..length).map(move |i| {
                let t = i as f32 / rate;
                let attack = (t / 0.004).min(1.0);
                let release = ((length - i) as f32 / (rate * 0.012)).min(1.0);
                let tone = (std::f32::consts::TAU * frequency * t).sin() * 0.8 + (2.0 * std::f32::consts::TAU * frequency * t).sin() * 0.2;
                (tone * attack * release * (-t * 28.0).exp() * CHIME_VOLUME * i16::MAX as f32) as i16
            })
        })
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    // PCM, one channel, the rate, bytes per second, bytes per sample, bits per sample.
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&CHIME_RATE.to_le_bytes());
    wav.extend_from_slice(&(CHIME_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
    wav
}

// --- Read-only check --------------------------------------------------------------------------
/// Describes what the running game holds at each address the trainer uses. Read-only.
/// Returns whether the game version was recognised, and the report line by line.
pub fn verify(p: &Process) -> (bool, Vec<String>) {
    let unreadable = || tr("UNREADABLE", "ILLISIBLE").to_string();
    let unexpected = |v: &dyn std::fmt::Display| trs(format!("{v}  UNEXPECTED"), format!("{v}  INATTENDU"));
    let mut out = vec![tr("Game code (valid from the main menu)", "Code du jeu (valable dès le menu principal)").to_string()];
    let mut code_ok = true;
    for site in &CODE_SITES {
        let verdict = match site.state(p) {
            SiteState::Original => tr("OK (original)", "OK (d'origine)").to_string(),
            SiteState::Patched => tr("OK (already changed by another program)", "OK (déjà modifié par un autre programme)").to_string(),
            SiteState::Unknown(bytes) => {
                code_ok = false;
                trs(format!("DIFFERENT: {}", hex(&bytes)), format!("DIFFÉRENT : {}", hex(&bytes)))
            }
            SiteState::Unreadable => {
                code_ok = false;
                unreadable()
            }
        };
        out.push(format!("  {:<24} {:#011x}  {verdict}", site.name(), site.addr));
    }
    let scan = p.find_pattern(BASE, IMAGE_END, &AMMO_PATTERN, true);
    let verdict = match scan.as_slice() {
        [addr] if *addr == AMMO_DECREMENT.addr => tr("OK (same address as above)", "OK (même adresse que ci-dessus)").to_string(),
        [] => tr("not found (normal if another program already changed the ammo)", "introuvable (normal si un autre programme a déjà modifié les munitions)").to_string(),
        other => trs(format!("DIFFERENT: {other:x?}"), format!("DIFFÉRENT : {other:x?}")),
    };
    out.push(format!("  {:<24} {:<11}  {verdict}", tr("Pattern search", "Recherche par motif"), ""));

    out.push(String::new());
    out.push(tr("FPS limit (to check once a game mode is loaded)", "Limite de FPS (à contrôler une fois un mode de jeu chargé)").to_string());
    out.push(match read_fps_dvar(p) {
        FpsRead::Valid(dvar) => trs(
            format!("  {:<24} {:#011x}  OK: limit {}, bounds {}..{}, hash matches", "Dvar", dvar.addr, dvar.limit, dvar.min, dvar.max),
            format!("  {:<24} {:#011x}  OK : limite {}, bornes {}..{}, empreinte conforme", "Dvar", dvar.addr, dvar.limit, dvar.min, dvar.max),
        ),
        FpsRead::NotADvar { addr } => trs(format!("  {:<24} {addr:#011x}  not registered here yet", "Dvar"), format!("  {:<24} {addr:#011x}  pas encore enregistrée ici", "Dvar")),
        FpsRead::Unreadable => format!("  {:<24} {:<11}  {}", "Dvar", "", unreadable()),
    });
    let pointer = tr("Pointer to the dvar", "Pointeur vers la dvar");
    out.push(match p.read_usize(FPS_DVAR_PTR) {
        Ok(ptr) => trs(
            format!("  {pointer:<24} {FPS_DVAR_PTR:#011x}  {ptr:#x} (expected {FPS_DVAR_DIRECT:#x})"),
            format!("  {pointer:<24} {FPS_DVAR_PTR:#011x}  {ptr:#x} (attendu {FPS_DVAR_DIRECT:#x})"),
        ),
        Err(_) => format!("  {pointer:<24} {FPS_DVAR_PTR:#011x}  {}", unreadable()),
    });

    out.push(String::new());
    out.push(tr("Match data (to check in game, in solo, after the round started)", "Données de partie (à contrôler en jeu, en solo, après le début de la manche)").to_string());
    if !in_match(p) {
        out.push(tr("  No match detected: the values below are not meaningful.", "  Aucune partie en cours détectée : les valeurs ci-dessous ne sont pas significatives.").to_string());
    }
    let client_slot = G_ENTITIES + ENTITY_CLIENT;
    let host = tr("Host player client", "Client du joueur hôte");
    out.push(match p.read_usize(client_slot) {
        Ok(ptr) if ptr == HOST_CLIENT => format!("  {host:<24} {client_slot:#011x}  {ptr:#x}  OK"),
        Ok(ptr) => trs(format!("  {host:<24} {client_slot:#011x}  {ptr:#x}  expected {HOST_CLIENT:#x}"), format!("  {host:<24} {client_slot:#011x}  {ptr:#x}  attendu {HOST_CLIENT:#x}")),
        Err(_) => format!("  {host:<24} {client_slot:#011x}  {}", unreadable()),
    });
    let int = |label: &str, addr: usize, sane: &std::ops::RangeInclusive<i32>| {
        let verdict = match p.read_i32(addr) {
            Ok(v) if sane.contains(&v) => format!("{v}  {}", tr("plausible", "plausible")),
            Ok(v) => unexpected(&v),
            Err(_) => unreadable(),
        };
        format!("  {label:<24} {addr:#011x}  {verdict}")
    };
    let flags = |label: &str, addr: usize| match p.read_i32(addr) {
        Ok(v) => format!("  {label:<24} {addr:#011x}  {v:#x}"),
        Err(_) => format!("  {label:<24} {addr:#011x}  {}", unreadable()),
    };
    out.push(int(tr("Player health", "Vie du joueur"), PLAYER_HEALTH, &SANE_HEALTH));
    out.push(flags(tr("Entity flags", "Drapeaux de l'entité"), G_ENTITIES + ENTITY_FLAGS));
    out.push(flags(tr("Client flags", "Drapeaux du client"), HOST_CLIENT + CLIENT_FLAGS));
    out.push(match read_position(p) {
        Some(position) => format!("  {:<24} {:#011x}  {position:.1?}  plausible", "Position", HOST_CLIENT + CLIENT_ORIGIN),
        None => format!("  {:<24} {:#011x}  {}", "Position", HOST_CLIENT + CLIENT_ORIGIN, tr("unavailable", "indisponible")),
    });
    out.push(int(tr("Round (fixed address)", "Manche (adresse fixe)"), ROUND, &SANE_ROUND));
    let round = tr("Round variable", "Variable de la manche");
    out.push(match (level_object(p), read_child(p, round_fixed_index())) {
        (Some(level), Some(var)) => {
            let field = var.parent == level as u16 && var.kind != VAR_UNDEFINED;
            trs(
                format!(
                    "  {round:<24} {:<11}  name {:#x}, type {}, parent {}; level object {}: {}",
                    "",
                    var.name,
                    var.kind,
                    var.parent,
                    level,
                    if field { "it is a level field" } else { "it is NOT a level field here" }
                ),
                format!(
                    "  {round:<24} {:<11}  nom {:#x}, type {}, parent {} ; objet level {} : {}",
                    "",
                    var.name,
                    var.kind,
                    var.parent,
                    level,
                    if field { "c'est bien un champ de level" } else { "ce n'est PAS un champ de level ici" }
                ),
            )
        }
        _ => format!("  {round:<24} {:<11}  {}", "", unreadable()),
    });
    for (i, slot) in CLIP_SLOTS.iter().enumerate().filter(|(i, _)| [0, 3, 5].contains(i)) {
        out.push(int(&trs(format!("Magazine (slot {i})"), format!("Chargeur (emplacement {i})")), BASE + slot, &SANE_AMMO));
    }
    out.push(int(tr("Reserve ammo", "Réserve de munitions"), BASE + RESERVE_SLOTS[0], &SANE_AMMO));
    let speed = tr("Game speed", "Vitesse du jeu");
    out.push(match p.read_f32(TIMESCALE) {
        Ok(v) if (0.01..=100.0).contains(&v) => format!("  {speed:<24} {TIMESCALE:#011x}  {v}  plausible"),
        Ok(v) => format!("  {speed:<24} {TIMESCALE:#011x}  {}", unexpected(&v)),
        Err(_) => format!("  {speed:<24} {TIMESCALE:#011x}  {}", unreadable()),
    });
    let zombies: Vec<i32> = (0..AGENT_SLOTS).filter_map(|slot| p.read_i32(agent_health(slot)).ok()).collect();
    let alive = zombies.iter().filter(|hp| SANE_HEALTH.contains(hp)).count();
    let label = tr("Zombie health", "Vie des zombies");
    out.push(trs(
        format!("  {label:<24} {:#011x}  {alive} alive out of {} slots read: {zombies:?}", agent_health(0), zombies.len()),
        format!("  {label:<24} {:#011x}  {alive} vivants sur {} emplacements lus : {zombies:?}", agent_health(0), zombies.len()),
    ));

    out.push(String::new());
    out.extend(describe_server(p));

    out.push(String::new());
    out.push(if code_ok {
        trs(
            format!("Game version recognised: the code matches the {} known places.", CODE_SITES.len()),
            format!("Version du jeu reconnue : le code correspond aux {} emplacements connus.", CODE_SITES.len()),
        )
    } else {
        tr("Game version NOT recognised: the trainer will refuse to write.", "Version du jeu NON reconnue : le trainer refusera d'écrire.").to_string()
    });
    (code_ok, out)
}

/// The server's player array and where the money is. Read-only.
fn describe_server(p: &Process) -> Vec<String> {
    let mut out = vec![tr("Server-side players (money)", "Joueurs côté serveur (argent)").to_string()];
    let array = tr("Player array", "Tableau des joueurs");
    let Some(clients) = server_clients(p) else {
        out.push(trs(format!("  {array:<24} {SVS_CLIENTS:#011x}  no server running"), format!("  {array:<24} {SVS_CLIENTS:#011x}  aucun serveur en cours")));
        return out;
    };
    out.push(format!("  {array:<24} {SVS_CLIENTS:#011x}  {clients:#x}"));
    let money = clients + SERVER_CLIENT_MONEY;
    let verdict = match p.read_i32(money) {
        Ok(value) if SANE_MONEY.contains(&value) => format!("{value}  plausible"),
        Ok(value) => trs(format!("{value}  UNEXPECTED"), format!("{value}  INATTENDU")),
        Err(_) => tr("UNREADABLE", "ILLISIBLE").to_string(),
    };
    out.push(format!("  {:<24} {money:#011x}  {verdict}", tr("Money", "Argent")));
    let legacy = if money == LEGACY_MONEY {
        tr("the same", "la même")
    } else {
        tr("DIFFERENT: the array is not at its most common place", "DIFFÉRENTE : le tableau n'est pas à son emplacement le plus courant")
    };
    out.push(format!("  {:<24} {LEGACY_MONEY:#011x}  {legacy}", tr("Fixed address", "Adresse fixe")));
    out
}

/// Game functions whose code is captured once per session, to be disassembled offline.
/// Addresses from iw7-mod (`symbols.hpp` and its IDA script of script functions).
const RESEARCH_FUNCTIONS: [(&str, usize); 13] = [
    // Gives the text of a script string: gives the string table.
    ("SL_ConvertToString", BASE + 0xC0_3300),
    // Finds a string by its text, then gives a field's name number: the two tables needed to
    // find `level.wave_num` or `self.ignoreme` by their name.
    ("SL_FindString", BASE + 0xC0_35F0),
    ("SL_GetCanonicalString", BASE + 0xBF_D340),
    // Finds a field in an object: confirms how fields are stored.
    ("FindVariable", BASE + 0xC0_5100),
    // What the scripts call to allow or forbid a move: gives the player's flag.
    ("PlayerCmd_allowdoublejump", BASE + 0xB0_1DA0),
    ("PlayerCmd_allowwallrun", BASE + 0xB0_23D0),
    ("PlayerCmd_allowslide", BASE + 0xB0_2790),
    ("PlayerCmd_allowdodge", BASE + 0xB0_2D10),
    // Ledge climbing, and three unnamed methods the Zombies mode calls at the same time.
    ("PlayerCmd_allowmantle", BASE + 0xB0_9760),
    ("méthode 0x8425", BASE + 0xB0_2500),
    ("méthode 0x8426", BASE + 0xB0_2690),
    ("méthode 0x8454", BASE + 0xB0_3020),
    // Sets the "player_damaged" bit that turns the screen red when health is low.
    ("PlayerCmd_setclientomnvarbit", BASE + 0xB0_8840),
];
const RESEARCH_BYTES: usize = 256;
/// Bytes captured for each function called by a function of the list, and at most how many per function.
const CALLEE_BYTES: usize = 64;
const MAX_CALLEES: usize = 12;
/// Extent of the game code: a call target outside it is not one.
pub(crate) const CODE_RANGE: std::ops::Range<usize> = BASE + 0x1000..BASE + 0x200_0000;

/// Targets of the direct calls (`E8` followed by a 32-bit offset) found in this code, in order and
/// without duplicates. An `E8` byte that is not a call gives a target outside the code, or one
/// capture too many: harmless, all of this is only reading.
pub(crate) fn call_targets(code: &[u8], addr: usize) -> Vec<usize> {
    let mut targets = Vec::new();
    for at in 0..code.len().saturating_sub(4) {
        if code[at] != 0xE8 {
            continue;
        }
        let offset = i32::from_le_bytes(code[at + 1..at + 5].try_into().unwrap());
        let target = (addr + at + 5).wrapping_add_signed(offset as isize);
        if CODE_RANGE.contains(&target) && !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets.truncate(MAX_CALLEES);
    targets
}

/// Bytes of a few game functions. The code is encrypted on disk: it can only be read in the
/// running game. Read-only; nothing captured here is written.
pub fn describe_code(p: &Process) -> Vec<String> {
    let mut out = vec!["--- Code relevé pour la recherche ---".to_string()];
    for (name, addr) in RESEARCH_FUNCTIONS {
        let mut code = [0u8; RESEARCH_BYTES];
        if p.read_bytes(addr, &mut code).is_err() {
            out.push(format!("{name} {addr:#011x} : ILLISIBLE"));
            continue;
        }
        out.push(format!("{name} {addr:#011x}"));
        for (row, chunk) in code.chunks(32).enumerate() {
            out.push(format!("  +{:#05x}  {}", row * 32, hex(chunk)));
        }
        // What this function calls: that is often where the work is done (a bit set or cleared).
        for target in call_targets(&code, addr) {
            let mut callee = [0u8; CALLEE_BYTES];
            if p.read_bytes(target, &mut callee).is_ok() {
                out.push(format!("  appel vers {target:#011x}"));
                for (row, chunk) in callee.chunks(32).enumerate() {
                    out.push(format!("    +{:#05x}  {}", row * 32, hex(chunk)));
                }
            }
        }
    }
    out.push("--- Fin du code relevé ---".to_string());
    out
}

// --- Suits (`SuitDef`, 0x2C0 bytes): captured to find how to enable the jump pack --------------
pub(crate) const ASSET_POOLS: usize = BASE + 0x146_63D0;
pub(crate) const ASSET_POOL_SIZES: usize = BASE + 0x146_6290;
pub(crate) const ASSET_TYPE_SUIT: usize = 67;
pub(crate) const SUIT_SIZE: usize = 0x2C0;
/// `SuitDef::jump_height`. Captured in a match: 39 in both Zombies suits, the engine's original value.
const SUIT_JUMP_HEIGHT: usize = 56;
/// Suits the Zombies mode gives the player.
const ZOMBIE_SUITS: [&str; 2] = ["zom_suit", "zom_suit_sprint"];

/// Address and jump height of the loaded Zombies suits. A suit is only returned if everything
/// cross-checks: pool in the game image, plausible number of slots, exact name, plausible jump
/// height. Entries follow the first 8 bytes of the pool (captured in a match).
fn zombie_suits(p: &Process) -> Vec<(usize, f32)> {
    let pool = p.read_usize(ASSET_POOLS + ASSET_TYPE_SUIT * 8).unwrap_or(0);
    let count = p.read_i32(ASSET_POOL_SIZES + ASSET_TYPE_SUIT * 4).unwrap_or(0);
    if !(BASE..IMAGE_END).contains(&pool) || !(1..=256).contains(&count) {
        return Vec::new();
    }
    let pool_end = pool + 8 + count as usize * SUIT_SIZE;
    (0..count as usize)
        .map(|index| pool + 8 + index * SUIT_SIZE)
        .filter(|&suit| {
            // A free slot starts with a pointer to the next free slot, inside the pool itself.
            let name = p.read_usize(suit).ok().filter(|name| !(pool..pool_end).contains(name)).and_then(|name| p.read_c_string(name, 64));
            name.is_some_and(|name| ZOMBIE_SUITS.contains(&name.as_str()))
        })
        .filter_map(|suit| p.read_f32(suit + SUIT_JUMP_HEIGHT).ok().filter(|height| SANE_JUMP_HEIGHT.contains(height)).map(|height| (suit, height)))
        .collect()
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Access;

    fn this_process() -> Process {
        Process::open(std::process::id(), "self".into(), Access::Memory).unwrap()
    }

    #[test]
    fn the_chime_is_a_quiet_wav_of_two_short_notes() {
        let wav = chime(&[659.25, 987.77]);
        let samples = 2 * CHIME_RATE as usize * CHIME_NOTE_MS / 1000;
        assert_eq!((&wav[..4], &wav[8..16], &wav[36..40]), (&b"RIFF"[..], &b"WAVEfmt "[..], &b"data"[..]));
        assert_eq!(wav.len(), 44 + samples * 2);
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize, wav.len() - 8);
        let loudest = wav[44..].chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]]).unsigned_abs()).max().unwrap();
        assert!(loudest > 1000 && (loudest as f32) <= CHIME_VOLUME * i16::MAX as f32 + 1.0);
    }

    #[test]
    fn integer_hash_matches_bytes_captured_from_the_game() {
        // Bytes captured in the game by IW FPS Unlocker: 85 fps originally, 1000 fps unlocked.
        assert_eq!(hashed_value(85), [0x55, 0x00, 0x00, 0x00, 0x85, 0x52, 0xE3, 0xC3]);
        assert_eq!(hashed_value(1000), [0xE8, 0x03, 0x00, 0x00, 0x38, 0x51, 0xE3, 0xC3]);
        assert_ne!(hash_integer(240), hash_integer(241));
    }

    /// Builds in local memory an integer dvar as the engine stores it.
    fn fake_dvar(limit: i32, min: i32, max: i32) -> Box<[u8; 96]> {
        let mut dvar = Box::new([0u8; 96]);
        dvar[DVAR_TYPE] = DVAR_TYPE_INTEGER;
        dvar[DVAR_CURRENT..DVAR_CURRENT + 8].copy_from_slice(&hashed_value(limit));
        dvar[DVAR_LATCHED..DVAR_LATCHED + 8].copy_from_slice(&hashed_value(limit));
        dvar[DVAR_DOMAIN..DVAR_DOMAIN + 4].copy_from_slice(&min.to_le_bytes());
        dvar[DVAR_DOMAIN + 4..DVAR_DOMAIN + 8].copy_from_slice(&max.to_le_bytes());
        dvar
    }

    #[test]
    fn dvar_layout_matches_the_fps_unlocker() {
        // The FPS unlocker writes at pointer + 0x10, which amounts to base + 0x75D0A30.
        assert_eq!(FPS_DVAR_DIRECT + DVAR_CURRENT, 0x1_475D_0A30);
        // The address falls exactly on an entry of `dvarPool` (0x147595E60, 96-byte entries).
        assert_eq!((FPS_DVAR_DIRECT - 0x1_4759_5E60) % 96, 0);
        let dvar = fake_dvar(85, 0, 1000);
        assert_eq!(dvar[DVAR_CURRENT + 4], 0x85);
    }

    #[test]
    fn addresses_are_consistent_with_the_engine_structures() {
        // Host player's health, and first / last zombie slot.
        assert_eq!(PLAYER_HEALTH, 0x1_43D2_27F0);
        assert_eq!(agent_health(0), 0x1_43D2_6F60);
        assert_eq!(agent_health(AGENT_SLOTS - 1), 0x1_43D2_CAA8);
        // Player position, and session state (`sess.sessionState`, offset 19092 from iw7-mod).
        assert_eq!(HOST_CLIENT + CLIENT_ORIGIN, 0x1_43CA_2088);
        assert_eq!(HOST_CLIENT + 19092, 0x1_43CA_6AF4);
        assert_eq!(CLIENT_FLAGS, 20924);
        assert_eq!((ENTITY_CLIENT, ENTITY_FLAGS, ENTITY_SIZE), (368, 456, 1016));
        // The ammo decrement instruction is the one the search pattern finds.
        assert_eq!(AMMO_DECREMENT.addr, 0x1_4073_323D);
        assert_eq!(AMMO_DECREMENT.original, &AMMO_PATTERN[..4]);
        // 32-bit word of no recoil: 0xAE840F <-> 0xAE850F.
        assert_eq!(u32::from_le_bytes(NO_RECOIL.original.try_into().unwrap()), 0xAE_840F);
        assert_eq!(u32::from_le_bytes(NO_RECOIL.patched.try_into().unwrap()), 0xAE_850F);
        // Old "no target": player health at 0x80000001. Fixed address of the round.
        assert_eq!(NO_TARGET_HEALTH as u32, 0x8000_0001);
        assert_eq!(ROUND, 0x1_4685_4DC0);
        // The round's fixed address falls exactly on the start of a slot of the script table.
        assert_eq!(CHILD_VARIABLES, 0x1_4672_9180);
        assert_eq!((ROUND - CHILD_VARIABLES) % CHILD_SIZE, 0);
        assert_eq!(round_fixed_index(), 51_160);
        // The gap between two players' money is the size of iw7-mod's `client_t`.
        assert_eq!(SERVER_CLIENT_SIZE, 715_416);
        // Money offset captured in a match, inside the first player.
        assert_eq!(SERVER_CLIENT_MONEY, 0xA_9DCE);
        assert_eq!(SVS_CLIENTS, 0x1_46B2_2950);
        for site in &CODE_SITES {
            assert_eq!(site.original.len(), site.patched.len(), "{}", site.name());
            assert!((BASE..IMAGE_END).contains(&site.addr));
        }
    }

    /// A process that is not the game must never be changed.
    #[test]
    fn unrecognised_process_is_left_alone() {
        let p = this_process();
        assert!(!build_is_recognised(&p));
        assert!(!in_match(&p));
        assert_eq!(host_entity_flags(&p), None);
        assert_eq!(host_client_flags(&p), None);
        assert_eq!(read_position(&p), None);
        assert_eq!(NO_RECOIL.toggle(&p), Err("unexpected code at this address"));
        assert_eq!(set_fps_limit(&p, 240), FpsOutcome::NotReady);
        let mut trainer = Trainer::new(0.5, 100_000);
        assert_eq!(trainer.set_round(&p, 5), Err(not_in_match().to_string()));
        assert!(trainer.toggle(&p, Feature::NoTarget).is_err() && trainer.toggle(&p, Feature::InfiniteMoney).is_err());
        // Settings and actions: refused the same way, and no suit is found.
        for tweak in Tweak::ALL {
            assert_eq!(trainer.set_tweak(&p, tweak, tweak.default()), Err(not_in_match()));
            assert_eq!(trainer.tweak_value(&p, tweak), None);
        }
        assert_eq!(trainer.set_tweak(&p, Tweak::Timescale, f32::NAN), Err("invalid value"));
        for action in OneShot::ALL {
            assert_eq!(trainer.run_action(&p, action), Err(not_in_match()));
        }
        assert!(zombie_suits(&p).is_empty());
        // The opening restore finds nothing to restore, so writes nothing.
        assert!(trainer.reset_leftovers(&p, &Pending::default()).is_empty());
        let everything = Pending { god_mode: true, noclip: true, jump_pack: true, no_target: true, timescale: true, move_speed: true, jump_height: Some(39.0), cash_scalar: true, paused_zombies: Some(24) };
        assert!(trainer.reset_leftovers(&p, &everything).is_empty());
        assert!(trainer.pending().is_empty());
    }

    #[test]
    fn script_variable_table_fits_before_the_next_engine_global() {
        // Four partitions of 0xA000 slots: the table ends before `g_script_error_level` (0x146B16298).
        assert_eq!(CHILD_COUNT, 163_840);
        const { assert!(CHILD_VARIABLES + CHILD_COUNT * CHILD_SIZE <= 0x1_46B1_6298) };
        // The round's fixed slot is in partition 1, the one of the `level` object captured in a match (125 then 3949).
        assert_eq!(round_fixed_index() / CHILD_PARTITION, 1);
        assert_eq!((125 & 3, 3949 & 3), (1, 1));
    }

    fn field(name: u32, kind: u8, value: i32) -> ChildVariable {
        ChildVariable { value, kind, name, parent: 5 }
    }

    /// Fields of a fake `level`: the round under the name `round`, surrounded by neighbours at the given gaps.
    fn level_with(round: u32, value: i32, gaps: &[i32]) -> Vec<(usize, ChildVariable)> {
        let mut fields = vec![field(round, VAR_INTEGER, value)];
        // Neighbours have various types, including plausible integers: they must not be taken for the round.
        fields.extend(gaps.iter().enumerate().map(|(i, gap)| field((round as i32 + gap) as u32, if i % 3 == 0 { VAR_INTEGER } else { 9 }, 1)));
        fields.into_iter().enumerate().map(|(slot, var)| (CHILD_PARTITION + 100 + slot, var)).collect()
    }

    #[test]
    fn the_round_is_found_by_its_known_name_only_when_it_is_a_plausible_integer() {
        let fields = level_with(ROUND_NAME, 7, &[-5, 3, 12]);
        assert_eq!(known_round_field(&fields, 5), Some(CHILD_PARTITION + 100));
        // Same name, but not an integer, value out of bounds, or a field of another object: refused.
        assert_eq!(known_round_field(&[(1, field(ROUND_NAME, 9, 7))], 5), None);
        assert_eq!(known_round_field(&[(1, field(ROUND_NAME, VAR_INTEGER, 60_266))], 5), None);
        assert_eq!(known_round_field(&[(1, field(ROUND_NAME, VAR_INTEGER, 7))], 6), None);
        // A map where this name does not exist: nothing, the neighbour search takes over.
        assert_eq!(known_round_field(&level_with(0x1234, 7, &[1, 2]), 5), None);
    }

    #[test]
    fn call_targets_are_computed_from_the_bytes_captured_in_the_game() {
        // Excerpts captured on the game: the call in `allowdoublejump` (at +0x79) leads to the function that sets the bit.
        let mut code = [0x90u8; 0x90];
        code[0x79..0x7E].copy_from_slice(&[0xE8, 0xC2, 0x7B, 0x53, 0xFF]);
        assert_eq!(call_targets(&code, BASE + 0xB0_1DA0), [BASE + 0x03_99E0]);
        // Those of `allowwallrun` (at +0x84 and +0x90).
        let mut code = [0x90u8; 0xA0];
        code[0x84..0x89].copy_from_slice(&[0xE8, 0xE7, 0x94, 0x56, 0xFF]);
        code[0x90..0x95].copy_from_slice(&[0xE8, 0x8B, 0x8E, 0x56, 0xFF]);
        assert_eq!(call_targets(&code, BASE + 0xB0_23D0), [BASE + 0x06_B940, BASE + 0x06_B2F0]);
        // An `E8` whose target is outside the code is not kept, nor the same target twice.
        let mut code = [0x90u8; 0x20];
        code[0..5].copy_from_slice(&[0xE8, 0xFF, 0xFF, 0xFF, 0x7F]);
        code[8..13].copy_from_slice(&[0xE8, 0x10, 0x00, 0x00, 0x00]);
        code[13..18].copy_from_slice(&[0xE8, 0x0B, 0x00, 0x00, 0x00]);
        assert_eq!(call_targets(&code, BASE + 0x10_0000), [BASE + 0x10_001D]);
    }

    #[test]
    fn an_engine_setting_is_found_by_its_hash_and_checked_before_use() {
        // Table sorted by hash: the address serves as an index here, the hash is ten times the index.
        let pointers: Vec<usize> = (0..7).collect();
        let name_of = |addr: usize| Some(addr as u32 * 10);
        assert_eq!(search_dvar(&pointers, 40, name_of), Some(4));
        assert_eq!(search_dvar(&pointers, 0, name_of), Some(0));
        assert_eq!(search_dvar(&pointers, 45, name_of), None);
        assert_eq!(search_dvar(&[], 40, name_of), None);
        assert_eq!(search_dvar(&pointers, 40, |_| None), None, "a failed read stops the search");

        let mut raw = [0u8; 0x48];
        raw[DVAR_TYPE] = DVAR_TYPE_INTEGER;
        raw[DVAR_CURRENT..DVAR_CURRENT + 8].copy_from_slice(&hashed_value(250));
        raw[DVAR_RESET..DVAR_RESET + 8].copy_from_slice(&hashed_value(190));
        assert_eq!(parse_int_dvar(&raw), Some((250, 190)));
        // A value hash that does not match, or another type: it is not the expected setting.
        raw[DVAR_CURRENT + 4] ^= 1;
        assert_eq!(parse_int_dvar(&raw), None);
        raw[DVAR_CURRENT + 4] ^= 1;
        raw[DVAR_TYPE] = 1;
        assert_eq!(parse_int_dvar(&raw), None);
    }

    #[test]
    fn the_round_variable_is_recognised_by_its_neighbours() {
        let gaps: Vec<i32> = (1..=30).map(|i| if i % 2 == 0 { i * 7 } else { -i * 5 }).collect();
        let learned = level_with(0x1_3BD3, 1, &gaps);
        let variable = RoundVariable { name: 0x1_3BD3, kind: VAR_INTEGER, neighbours: neighbour_deltas(learned.iter().map(|(_, var)| var.name), 0x1_3BD3) };
        assert_eq!(variable.neighbours.len(), 30);
        assert_eq!(&variable.neighbours[..2], &[-5, 14], "du plus proche au plus lointain");

        // Same match: recognised under the same number, with all its neighbours.
        assert_eq!(align_round(&variable, &learned), Alignment::Found { index: CHILD_PARTITION + 100, name: 0x1_3BD3, matched: 30 });

        // Next match: every number has shifted, and a quarter of the neighbours do not exist yet.
        let mut later = level_with(0x2_0000, 7, &gaps[..23]);
        // Other plausible `level` integers, unrelated.
        later.extend((0..40).map(|i| (CHILD_PARTITION + 900 + i, field(0x900 + i as u32 * 131, VAR_INTEGER, 1))));
        assert_eq!(align_round(&variable, &later), Alignment::Found { index: CHILD_PARTITION + 100, name: 0x2_0000, matched: 23 });

        // Too few neighbours found: refusal, with the best score for the log.
        let sparse = level_with(0x2_0000, 7, &gaps[..10]);
        assert_eq!(align_round(&variable, &sparse), Alignment::Missing { best: 10 });

        // A field with the same name but not a plausible integer is never kept.
        let mut wrong_kind = level_with(0x2_0000, 7, &gaps);
        wrong_kind[0].1.kind = 9;
        assert!(matches!(align_round(&variable, &wrong_kind), Alignment::Missing { .. }));
        let mut wrong_value = level_with(0x2_0000, 60_266, &gaps);
        assert!(matches!(align_round(&variable, &wrong_value), Alignment::Missing { .. }));
        wrong_value[0].1.value = -1;
        assert!(matches!(align_round(&variable, &wrong_value), Alignment::Missing { .. }));

        // Two fields surrounded by the same neighbours: nothing is chosen.
        let mut twice = level_with(0x2_0000, 7, &gaps);
        twice.extend(level_with(0x5_0000, 3, &gaps));
        assert_eq!(align_round(&variable, &twice), Alignment::Ambiguous);

        // A variable captured with too few neighbours is not used for cross-checking.
        let weak = RoundVariable { neighbours: variable.neighbours[..ROUND_MIN_NEIGHBOURS - 1].to_vec(), ..variable.clone() };
        assert_eq!(align_round(&weak, &learned), Alignment::Missing { best: 0 });
    }

    #[test]
    fn tweaks_and_actions_have_distinct_keys_and_sane_bounds() {
        for tweak in Tweak::ALL {
            let (min, max, step) = tweak.range();
            assert!(min < max && step > 0.0 && (min..=max).contains(&tweak.default()), "{}", tweak.key());
        }
        let mut keys: Vec<&str> = Tweak::ALL.iter().map(|tweak| tweak.key()).chain(OneShot::ALL.iter().map(|action| action.key())).chain(Feature::ALL.iter().map(|feature| feature.key())).collect();
        let count = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), count);
        // Jump height is the `jump_height` field of the suits, captured at 39 in a match.
        assert_eq!((SUIT_JUMP_HEIGHT, Tweak::JumpHeight.default()), (56, 39.0));
    }

    #[test]
    fn code_site_toggles_and_restores() {
        let p = this_process();
        let buf = Box::new([0x0Fu8, 0x84, 0xAE, 0x00]);
        let site = CodeSite { addr: buf.as_ptr() as usize, ..NO_RECOIL };
        assert_eq!(site.toggle(&p), Ok(true));
        assert_eq!(site.state(&p), SiteState::Patched);
        site.restore(&p);
        assert_eq!(site.state(&p), SiteState::Original);
        p.write_bytes(site.addr, &[0xCC]).unwrap();
        assert_eq!(site.state(&p), SiteState::Unknown(vec![0xCC, 0x84, 0xAE, 0x00]));
        assert!(site.toggle(&p).is_err());
    }

    #[test]
    fn native_flags_flip_one_bit_and_keep_the_others() {
        let p = this_process();
        let flags = Box::new(0x1000i32);
        let addr = &*flags as *const i32 as usize;
        assert_eq!(flip_bits(&p, addr, FL_GODMODE), Some(true));
        assert_eq!(p.read_i32(addr).unwrap(), 0x1001);
        assert_eq!(flip_bits(&p, addr, FL_GODMODE), Some(false));
        assert_eq!(flip_bits(&p, addr, FL_GODMODE), Some(true));
        clear_bits(&p, addr, FL_GODMODE);
        assert_eq!(p.read_i32(addr).unwrap(), 0x1000);
    }

    #[test]
    fn sane_writes_only() {
        let p = this_process();
        let value = Box::new(250i32);
        let addr = &*value as *const i32 as usize;
        assert!(write_if_sane(&p, addr, 500, &SANE_AMMO));
        assert_eq!(p.read_i32(addr).unwrap(), 500);
        p.write_bytes(addr, &(-7i32).to_le_bytes()).unwrap();
        assert!(!write_if_sane(&p, addr, 500, &SANE_AMMO));
        assert_eq!(p.read_i32(addr).unwrap(), -7);
    }
}
