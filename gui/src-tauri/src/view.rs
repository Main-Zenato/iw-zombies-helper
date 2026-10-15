//! What the pages show: the engine and configuration state, turned into sentences and numbers
//! ready to display. The pages (gui/src) decide nothing; `gui/src/api.ts` describes the same data
//! on the TypeScript side.

use iw_helper::config::{self, Config, Priority};
use iw_helper::engine::{FeatureState, Game, Snapshot};
use iw_helper::lang::{self, tr, trs};
use iw_helper::trainer::{Feature, OneShot, Tweak};
use iw_helper::winsettings::GraphicsSettings;
use iw_helper::{cpu, profile};
use serde::Serialize;

/// Background of the interface cards, and Windows' accent when the user's does not stand out on it.
const CARD: [u8; 3] = [0x2B, 0x2B, 0x2B];
const DEFAULT_ACCENT: [u8; 3] = [0x60, 0xCD, 0xFF];
/// Wanted contrast between the accent and a card background: a switch that is on must be seen from afar.
const ACCENT_CONTRAST: f64 = 6.0;

/// The options, grouped by what they act on; title in English then in French.
const GROUPS: [([&str; 2], &[Feature]); 3] = [
    (["Player", "Joueur"], &[Feature::GodMode, Feature::NoTarget, Feature::Noclip, Feature::JumpPack]),
    (["Weapons", "Armes"], &[Feature::InfiniteAmmo, Feature::OneHitKill, Feature::NoRecoil, Feature::RapidFire]),
    (["Match", "Partie"], &[Feature::InfiniteMoney, Feature::SlowMotion, Feature::LowGravity, Feature::PauseZombies]),
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub start_tab: String,
    /// Language of the text: "en" or "fr". The pages pick their own text from it.
    pub language: &'static str,
    /// Accent color, "#rrggbb".
    pub accent: String,
    pub game: &'static str,
    pub state: &'static str,
    pub message: Message,
    pub ready: bool,
    pub profile_ready: bool,
    pub groups: Vec<Group>,
    /// Numeric settings and one-off actions, valid during a match.
    pub tweaks: Vec<TweakRow>,
    pub actions: Vec<ActionRow>,
    pub keys: Keys,
    pub saved_position: Option<[f32; 3]>,
    /// Code of the character requested for the next match, when the profile is readable.
    pub character: Option<u8>,
    pub characters: Vec<Character>,
    pub hotkeys: Vec<Hotkey>,
    pub watching: bool,
    pub settings: Settings,
    pub log_path: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Message {
    pub text: String,
    /// `guide` (what to do), `done` (action done), `refused` (action refused) or `error`.
    pub tone: &'static str,
}

#[derive(Serialize)]
pub struct Group {
    pub title: &'static str,
    pub options: Vec<OptionRow>,
}

#[derive(Serialize)]
pub struct OptionRow {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: String,
    pub hotkey: String,
    pub on: bool,
    pub code: bool,
    /// Never seen working in game.
    pub trial: bool,
}

/// A numeric setting that acts during the match.
#[derive(Serialize)]
pub struct TweakRow {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    /// Current value: read from the game during a match, otherwise the original value.
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// The game's original value.
    pub default: f32,
    pub unit: &'static str,
    pub code: bool,
    pub trial: bool,
}

/// A one-off action during the match.
#[derive(Serialize)]
pub struct ActionRow {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    /// Button text.
    pub button: &'static str,
    pub trial: bool,
}

#[derive(Serialize)]
pub struct Character {
    pub code: u8,
    pub name: &'static str,
    /// Map on which the game accepts this character.
    pub map: &'static str,
}

/// A keyboard shortcut, for the list on the Settings page.
#[derive(Serialize)]
pub struct Hotkey {
    pub label: &'static str,
    /// Empty when the action has no key.
    pub key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Keys {
    pub all_off: String,
    pub save_position: String,
    pub teleport: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub cpu: CpuSettings,
    pub fps: FpsSettings,
    pub windows: Option<WindowsSettings>,
    pub on_top: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSettings {
    pub enabled: bool,
    pub smt: bool,
    pub cores: usize,
    pub core_count: usize,
    pub priority: &'static str,
    pub priorities: Vec<Choice>,
    /// What the game will get at its next launch, in one sentence.
    pub outcome: String,
    /// Threads the game will get, out of the machine's: the Home page shows them in large type.
    pub threads: u32,
    pub total_threads: u32,
}

#[derive(Serialize)]
pub struct Choice {
    pub key: &'static str,
    pub label: &'static str,
}

#[derive(Serialize)]
pub struct FpsSettings {
    pub enabled: bool,
    pub limit: i32,
    pub effective: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsSettings {
    pub dedicated_gpu: bool,
    pub windowed_optimizations_off: bool,
}

/// Result of the last action, found in the log.
pub struct Feedback {
    pub text: String,
    pub refused: bool,
}

/// Everything the view is built from.
pub struct Input<'a> {
    pub snapshot: &'a Snapshot,
    pub config: &'a Config,
    pub cores: &'a [usize],
    pub start_tab: &'a str,
    pub accent: &'a str,
    /// Saving or settings error, shown until the next one.
    pub notice: Option<&'a str>,
    /// Result of the last action, as long as it is recent enough to be shown.
    pub feedback: Option<&'a Feedback>,
    /// Windows graphics settings for the game, if it was found on disk.
    pub graphics: Option<&'a GraphicsSettings>,
    pub on_top: bool,
    pub log_path: Option<String>,
}

pub fn build(input: &Input) -> View {
    let Input { snapshot, config, .. } = *input;
    let memory = snapshot.game.as_ref().is_some_and(|game| game.memory && game.recognised == Some(true));
    let (game, state) = match &snapshot.game {
        None => ("absent", tr("Game not running", "Jeu non lancé")),
        Some(game) if game.in_match => ("match", tr("In a match", "En partie")),
        Some(_) => ("menu", tr("Game running, not in a match", "Jeu lancé, hors partie")),
    };
    // A single line at the top: the current error, otherwise the last action's result, otherwise what to do.
    let message = match (input.notice, input.feedback) {
        (Some(notice), _) => Message { text: notice.to_string(), tone: "error" },
        (None, Some(feedback)) => Message { text: feedback.text.clone(), tone: if feedback.refused { "refused" } else { "done" } },
        (None, None) => Message { text: guidance(config, snapshot), tone: "guide" },
    };
    let key = |action: &str| config.hotkey(action).map(key_label).unwrap_or_default();
    View {
        start_tab: input.start_tab.to_string(),
        language: lang::current().key(),
        accent: input.accent.to_string(),
        game,
        state,
        message,
        ready: memory && snapshot.trainer_enabled,
        profile_ready: memory,
        groups: GROUPS
            .iter()
            .map(|([en, fr], features)| Group { title: tr(en, fr), options: features.iter().map(|&feature| option(feature, snapshot, config)).collect() })
            .collect(),
        tweaks: Tweak::ALL.iter().map(|&tweak| tweak_row(tweak, snapshot)).collect(),
        actions: OneShot::ALL
            .iter()
            .map(|&action| ActionRow { key: action.key(), label: action.label(), hint: action.hint(), button: action.button(), trial: action.trial() })
            .collect(),
        keys: Keys { all_off: key(config::ACTION_ALL_OFF), save_position: key(config::ACTION_SAVE_POSITION), teleport: key(config::ACTION_TELEPORT) },
        saved_position: snapshot.saved_position,
        character: snapshot.character,
        characters: profile::CHARACTERS
            .iter()
            // The normal draw applies to every map: only its name and map get translated.
            .map(|&(code, name, map)| if code == 0 { Character { code, name: tr("Normal draw", name), map: tr("All maps", map) } } else { Character { code, name, map } })
            .collect(),
        hotkeys: hotkeys(config),
        watching: snapshot.watching,
        settings: settings(input),
        log_path: input.log_path.clone(),
    }
}

fn option(feature: Feature, snapshot: &Snapshot, config: &Config) -> OptionRow {
    let on = snapshot.features.iter().any(|state| state.feature == feature && state.on);
    OptionRow {
        key: feature.key(),
        label: feature.label().trim_end_matches(" (essai)"),
        hint: hint(feature, config),
        hotkey: config.hotkey(feature.key()).map(key_label).unwrap_or_default(),
        on,
        code: feature.patches_code(),
        trial: feature.trial(),
    }
}

fn tweak_row(tweak: Tweak, snapshot: &Snapshot) -> TweakRow {
    let (min, max, step) = tweak.range();
    // Outside a match, or if the game could not be read: the original value.
    let value = snapshot.tweaks.iter().find(|(candidate, _)| *candidate == tweak).and_then(|(_, value)| *value).unwrap_or(tweak.default());
    TweakRow { key: tweak.key(), label: tweak.label(), hint: tweak.hint(), value, min, max, step, default: tweak.default(), unit: tweak.unit(), code: false, trial: tweak.trial() }
}

/// Every shortcut, in page order: the options, then position and turning everything off.
fn hotkeys(config: &Config) -> Vec<Hotkey> {
    let key = |action: &str| config.hotkey(action).map(key_label).unwrap_or_default();
    let features = GROUPS.iter().flat_map(|(_, features)| features.iter()).map(|feature| (feature.label().trim_end_matches(" (essai)"), feature.key()));
    let one_shots = OneShot::ALL.iter().map(|action| (action.label(), action.key()));
    let actions = [
        (tr("Save position", "Enregistrer la position"), config::ACTION_SAVE_POSITION),
        (tr("Back to saved position", "Revenir à la position"), config::ACTION_TELEPORT),
        (tr("Turn everything off", "Tout désactiver"), config::ACTION_ALL_OFF),
    ];
    features.chain(one_shots).chain(actions).map(|(label, action)| Hotkey { label, key: key(action) }).collect()
}

/// Windows' accent in its light shade, the one dark themes use; otherwise Windows' original accent.
/// An accent too dull to stand out on a card is lightened, in steps of 5% towards white, keeping its
/// hue. Written "#rrggbb".
pub fn accent(shades: Option<[[u8; 3]; 7]>) -> String {
    let base = shades.map_or(DEFAULT_ACCENT, |shades| shades[1]);
    let lightened = |steps: u32| base.map(|channel| (channel as f64 + (255.0 - channel as f64) * steps as f64 * 0.05).round() as u8);
    let [r, g, b] = (0..=20).map(lightened).find(|&color| contrast(color, CARD) >= ACCENT_CONTRAST).unwrap_or([255; 3]);
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Contrast ratio between two colors (WCAG formula), from 1 to 21.
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |[r, g, b]: [u8; 3]| {
        let linear = |channel: u8| {
            let value = channel as f64 / 255.0;
            if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn settings(input: &Input) -> Settings {
    let (config, cores) = (input.config, input.cores);
    let used = if config.cpu.max_cores == 0 { cores.len() } else { config.cpu.max_cores.min(cores.len()) };
    let threads = cpu::build_mask(cores, config.cpu.smt, config.cpu.max_cores).count_ones();
    let total: u32 = cores.iter().map(|core| core.count_ones()).sum();
    let outcome = if config.cpu.enabled {
        trs(format!("The game will start with {threads} of {total} threads."), format!("Le jeu démarrera avec {threads} threads sur {total}."))
    } else {
        trs(format!("The game will start without the tool's settings: {total} threads, normal priority."), format!("Le jeu démarrera sans réglage de l'outil : {total} threads, priorité normale."))
    };
    let effective = effective_fps(config.fps.limit);
    Settings {
        cpu: CpuSettings {
            enabled: config.cpu.enabled,
            smt: config.cpu.smt,
            cores: used,
            core_count: cores.len(),
            priority: config.cpu.priority.key(),
            priorities: Priority::ALL.iter().map(|&priority| Choice { key: priority.key(), label: priority_label(priority) }).collect(),
            outcome,
            threads,
            total_threads: total,
        },
        fps: FpsSettings {
            enabled: config.fps.enabled,
            limit: config.fps.limit,
            effective,
        },
        windows: input
            .graphics
            .map(|graphics| WindowsSettings { dedicated_gpu: graphics.dedicated_gpu, windowed_optimizations_off: graphics.windowed_optimizations_off }),
        on_top: input.on_top,
    }
}

/// Short name of a priority, to fit a three-choice selector.
fn priority_label(priority: Priority) -> &'static str {
    match priority {
        Priority::Normal => tr("Normal", "Normale"),
        Priority::AboveNormal => tr("Above", "Supérieure"),
        Priority::High => tr("High", "Haute"),
    }
}

/// What to do now, when no recent action is to be reported.
fn guidance(config: &Config, snapshot: &Snapshot) -> String {
    let Some(game) = &snapshot.game else {
        return tr("Start Infinite Warfare from Steam: this window will follow it.", "Lancez Infinite Warfare depuis Steam : cette fenêtre le suivra.").to_string();
    };
    if !config.trainer.enabled {
        return tr("The trainer is turned off in the configuration (trainer.enabled).", "Le trainer est désactivé dans la configuration (trainer.enabled).").to_string();
    }
    if !game.memory {
        return tr("Memory access is not allowed on this game (memory.allow_steam_client).", "L'accès mémoire n'est pas autorisé sur ce jeu (memory.allow_steam_client).").to_string();
    }
    let threads = game.affinity.count_ones();
    match game.recognised {
        Some(false) => tr("Game version not recognised: nothing will be changed.", "Version du jeu non reconnue : rien ne sera modifié.").to_string(),
        None => tr("Checking the game version…", "Contrôle de la version du jeu en cours…").to_string(),
        Some(true) if !game.in_match => trs(format!("Options unlock once the round has started. The game runs on {threads} threads."), format!("Les options se débloquent une fois la manche commencée. Le jeu tourne sur {threads} threads.")),
        Some(true) => trs(format!("Click an option, or use its key in game. The game runs on {threads} threads."), format!("Cliquez une option, ou utilisez sa touche en jeu. Le jeu tourne sur {threads} threads.")),
    }
}

/// Action result readable in a log line. `None` for survey lines.
pub fn outcome(line: &str) -> Option<Feedback> {
    let text = line.get(10..)?.trim();
    if text.is_empty() || line.as_bytes().get(10) == Some(&b' ') || text.starts_with("Surveillance, ") || text.starts_with("Watching, ") || text.starts_with("---") {
        return None;
    }
    // The words of a refusal and of a completed action, in French then in English: the log may mix
    // both languages if it changed during the session.
    let refused = [
        "indisponible", "impossible", "refus", "non reconnue", "inactif", "inaccessible", "non appliqu", "n'est pas lancé", "inattendue", "non confirmée",
        "unavailable", "not recognised", "Trainer off", "not applied", "not running", "unexpected", "not confirmed", "Could not",
    ]
    .iter()
    .any(|word| text.contains(word));
    let done = [
        " : ON", " : OFF", "Tout est désactivé", "Manche :", "Position enregistrée", "Téléporté", "Limite de FPS réglée", "Surveillance ", "appliqués", "Profil :", " -> ", " : fait", "remises à l'état d'origine", "Relevé écrit",
        ": ON", ": OFF", "Everything is off", "Round:", "Position saved", "Teleported", "FPS limit set", "Watching ", "applied", "Profile:", ": done", "restored", "Survey written",
    ]
    .iter()
    .any(|word| text.contains(word));
    if !refused && !done {
        return None;
    }
    let text = text.replace(" : ON", " : activé").replace(" : OFF", " : coupé").replace(": ON", ": on").replace(": OFF", ": off").replace(" (essai)", "");
    Some(Feedback { text, refused })
}

/// Name of a key as printed on a keyboard of the chosen language.
fn key_label(key: &str) -> String {
    match key {
        "PageUp" => tr("Page Up", "Page préc."),
        "PageDown" => tr("Page Down", "Page suiv."),
        "End" => tr("End", "Fin"),
        "Home" => tr("Home", "Début"),
        "Insert" => tr("Insert", "Inser"),
        "Delete" => tr("Delete", "Suppr"),
        other => other,
    }
    .to_string()
}

/// What an option does, in one line under its name.
fn hint(feature: Feature, config: &Config) -> String {
    match feature {
        Feature::GodMode => tr("You no longer take damage.", "Vous ne prenez plus de dégâts.").to_string(),
        Feature::NoTarget => tr("Zombies ignore you, but still hit you if you bump into them.", "Les zombies vous ignorent, mais frappent encore si vous les touchez.").to_string(),
        Feature::Noclip => tr("Walk through walls. Turn it off with your feet on the ground.", "Traversez les murs. À couper les pieds au sol.").to_string(),
        Feature::JumpPack => tr("Brings back double jump, wall running and ledge climbing.", "Rend le double saut, la course sur les murs et l'escalade des rebords.").to_string(),
        Feature::InfiniteAmmo => tr("Magazine and reserve stay full; the magazine shows 511.", "Chargeur et réserve pleins ; le chargeur affiche 511.").to_string(),
        Feature::OneHitKill => tr("Any hit kills a zombie.", "Le moindre coup tue un zombie.").to_string(),
        Feature::NoRecoil => tr("The weapon no longer climbs when you fire.", "L'arme ne monte plus quand vous tirez.").to_string(),
        Feature::RapidFire => tr("Faster rate of fire.", "Cadence de tir accélérée.").to_string(),
        Feature::InfiniteMoney => trs(format!("Money kept at {}.", thousands(config.trainer.money)), format!("Argent maintenu à {}.", thousands(config.trainer.money))),
        Feature::SlowMotion => trs(format!("Game speed multiplied by {}.", config.trainer.timescale), format!("Vitesse du jeu multipliée par {}.", config.trainer.timescale.to_string().replace('.', ","))),
        Feature::LowGravity => tr("Higher, slower jumps.", "Sauts plus hauts et plus lents.").to_string(),
        Feature::PauseZombies => tr("Current zombies vanish and no new ones spawn. They come back when you turn it off.", "Les zombies présents disparaissent et plus aucun n'apparaît. Ils reviennent en coupant.").to_string(),
    }
}

/// "100 000" in French (non-breaking space), "100,000" in English.
fn thousands(value: i32) -> String {
    let separator = lang::current().pick(',', '\u{a0}');
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

/// The engine rounds a frame's length down to the millisecond.
fn effective_fps(limit: i32) -> i32 {
    1000 / (1000 / limit.clamp(1, 1000)).max(1)
}

/// Fake state to see the interface without the game (`--demo`).
pub fn demo_snapshot() -> Snapshot {
    Snapshot {
        game: Some(Game { pid: 4242, name: config::STEAM_CLIENT.to_string(), exe: None, memory: true, in_match: true, recognised: Some(true), affinity: 0x55_5555 }),
        trainer_enabled: true,
        // As when the real tool opens: no option on, settings at their original value.
        features: Feature::ALL.iter().map(|&feature| FeatureState { feature, on: false, patches_code: feature.patches_code() }).collect(),
        saved_position: Some([1204.0, -388.0, 64.0]),
        tweaks: Tweak::ALL.iter().map(|&tweak| (tweak, Some(tweak.default()))).collect(),
        watching: false,
        character: Some(0),
        finished: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iw_helper::lang::Lang;

    /// The language is shared by the whole program: tests that read text set it, one at a time.
    static LANGUAGE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn in_language<T>(language: Lang, run: impl FnOnce() -> T) -> T {
        let _turn = LANGUAGE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        lang::set(language);
        let out = run();
        lang::set(Lang::En);
        out
    }

    fn text(line: &str) -> Option<(String, bool)> {
        outcome(line).map(|feedback| (feedback.text, feedback.refused))
    }

    #[test]
    fn journal_lines_become_feedback_only_for_actions() {
        assert_eq!(text("18:43:32  Argent infini : ON"), Some(("Argent infini : activé".to_string(), false)));
        assert_eq!(text("18:43:32  Jump pack (essai) : OFF"), Some(("Jump pack : coupé".to_string(), false)));
        assert_eq!(text("18:49:59  Invincibilité indisponible (pas en partie)"), Some(("Invincibilité indisponible (pas en partie)".to_string(), true)));
        assert_eq!(text("18:43:20  Manche : 1 -> 46."), Some(("Manche : 1 -> 46.".to_string(), false)));
        assert_eq!(text("18:43:20  Le jeu n'est pas lancé."), Some(("Le jeu n'est pas lancé.".to_string(), true)));
        assert_eq!(text("18:43:20  Personnage : Tirage normal -> Elvira (pour la prochaine partie)").map(|(_, refused)| refused), Some(false));
        assert_eq!(text("18:43:20  Personnage : valeur inattendue (77), le profil ne semble pas chargé. Rien n'a été écrit.").map(|(_, refused)| refused), Some(true));
        // Check and watch lines: not an action result.
        assert_eq!(text("18:42:10  Partie détectée."), None);
        assert_eq!(text("18:42:10    Sans recul               0x1408e188c  OK (d'origine)"), None);
        assert_eq!(text("18:51:41  Surveillance, entité +0x0c0 : 24 -> 99"), None);
        assert!(text("18:51:41  Surveillance prête : ce que le jeu change maintenant sur le joueur sera noté.").is_some());
        // Numeric settings and one-off actions.
        assert_eq!(text("18:43:20  Hauteur de saut : 39 -> 120").map(|(_, refused)| refused), Some(false));
        assert_eq!(text("18:43:20  Remplir les munitions : fait (9 chargeurs, 4 réserves)").map(|(_, refused)| refused), Some(false));
        assert_eq!(text("18:43:20  Affaiblir les zombies présents indisponible (aucun zombie présent)").map(|(_, refused)| refused), Some(true));
        // Full survey: the file name on success, the reason otherwise.
        assert_eq!(text("18:43:20  Relevé écrit : releve-20261014-184320.txt"), Some(("Relevé écrit : releve-20261014-184320.txt".to_string(), false)));
        assert_eq!(text("18:43:20  Relevé impossible (version du jeu non reconnue, ou pas encore contrôlée).").map(|(_, refused)| refused), Some(true));
        assert_eq!(text("18:43:20  Remplir les munitions : fait (démonstration, rien n'est écrit)").map(|(_, refused)| refused), Some(false));
        // The round search at the start of a match is not an action result.
        assert_eq!(text("18:42:10  Recherche de la manche : level 125, 900 champs dont 300 entiers plausibles, noms 0x12 à 0x14000 ; variable jamais relevée."), None);
        assert_eq!(text("18:42:51  Variable de la manche relevée (nom 0x13bd3), avec 48 champs voisins : elle sera cherchée par eux aux parties suivantes."), None);
        // The same in English, the default language.
        assert_eq!(text("18:43:32  Infinite money: ON"), Some(("Infinite money: on".to_string(), false)));
        assert_eq!(text("18:49:59  God mode unavailable (not in a match)"), Some(("God mode unavailable (not in a match)".to_string(), true)));
        assert_eq!(text("18:43:20  The game is not running.").map(|(_, refused)| refused), Some(true));
        assert_eq!(text("18:43:20  Round: 1 -> 46.").map(|(_, refused)| refused), Some(false));
        assert_eq!(text("18:43:20  Refill ammo: done (9 magazines, 4 reserves)").map(|(_, refused)| refused), Some(false));
        assert_eq!(text("18:43:20  Survey written: releve-20261014-184320.txt").map(|(_, refused)| refused), Some(false));
        assert_eq!(text("18:42:10  Match detected."), None);
        assert_eq!(text("18:51:41  Watching, entity +0x0c0: 24 -> 99"), None);
    }

    #[test]
    fn tweaks_and_actions_are_listed_with_the_names_the_pages_expect() {
        let (config, cores) = (Config::default(), [0b11]);
        let view = |snapshot: &Snapshot| {
            build(&Input { snapshot, config: &config, cores: &cores, start_tab: "partie", accent: "#60cdff", notice: None, feedback: None, graphics: None, on_top: false, log_path: None })
        };
        // Without a game: settings are listed at their original value.
        let idle = view(&Snapshot::default());
        assert_eq!(idle.tweaks.iter().map(|tweak| (tweak.key, tweak.value)).collect::<Vec<_>>(), [("timescale", 1.0), ("move_speed", 190.0), ("jump_height", 39.0), ("cash_scalar", 1.0)]);
        assert_eq!(idle.actions.iter().map(|action| action.key).collect::<Vec<_>>(), ["refill_ammo", "add_money", "weaken_zombies", "end_round", "charge_cards", "add_tickets"]);
        // In a match: the value read from the game; an unreadable value falls back to the original.
        let mut snapshot = demo_snapshot();
        snapshot.tweaks = vec![(Tweak::Timescale, Some(0.5)), (Tweak::JumpHeight, None)];
        let playing = view(&snapshot);
        assert_eq!((playing.tweaks[0].value, playing.tweaks[2].value), (0.5, 39.0));
        // Field names as the pages read them, and no toggle option is a trial anymore.
        let json = serde_json::to_value(&playing).unwrap();
        let tweak = &json["tweaks"][1];
        for name in ["key", "label", "hint", "value", "min", "max", "step", "default", "unit", "code", "trial"] {
            assert!(!tweak[name].is_null(), "value without field {name}");
        }
        for name in ["key", "label", "hint", "button", "trial"] {
            assert!(!json["actions"][0][name].is_null(), "action without field {name}");
        }
        let trials: Vec<&str> = playing.groups.iter().flat_map(|group| &group.options).filter(|option| option.trial).map(|option| option.key).collect();
        assert!(trials.is_empty(), "{trials:?}");
        assert!(json["groups"][0]["options"][0]["trial"].is_boolean());
    }

    /// The names sent to the pages are exactly the ones `gui/src/api.ts` reads, no more, no less: a
    /// field renamed on one side only would break the interface without any compile error.
    #[test]
    fn the_json_sent_to_the_pages_has_exactly_the_names_they_read() {
        use std::collections::BTreeSet;
        fn names(value: &serde_json::Value) -> BTreeSet<&str> {
            value.as_object().map(|object| object.keys().map(String::as_str).collect()).unwrap_or_default()
        }
        let (config, cores, graphics, snapshot) = (Config::default(), [0b11], GraphicsSettings::default(), demo_snapshot());
        let view = build(&Input { snapshot: &snapshot, config: &config, cores: &cores, start_tab: "partie", accent: "#60cdff", notice: None, feedback: None, graphics: Some(&graphics), on_top: false, log_path: None });
        let json = serde_json::to_value(&view).unwrap();
        let expected: [(&serde_json::Value, &[&str]); 13] = [
            (&json, &["startTab", "language", "accent", "game", "state", "message", "ready", "profileReady", "groups", "tweaks", "actions", "keys", "savedPosition", "character", "characters", "hotkeys", "watching", "settings", "logPath"]),
            (&json["message"], &["text", "tone"]),
            (&json["groups"][0], &["title", "options"]),
            (&json["groups"][0]["options"][0], &["key", "label", "hint", "hotkey", "on", "code", "trial"]),
            (&json["tweaks"][0], &["key", "label", "hint", "value", "min", "max", "step", "default", "unit", "code", "trial"]),
            (&json["actions"][0], &["key", "label", "hint", "button", "trial"]),
            (&json["keys"], &["allOff", "savePosition", "teleport"]),
            (&json["characters"][0], &["code", "name", "map"]),
            (&json["hotkeys"][0], &["label", "key"]),
            (&json["settings"], &["cpu", "fps", "windows", "onTop"]),
            (&json["settings"]["cpu"], &["enabled", "smt", "cores", "coreCount", "priority", "priorities", "outcome", "threads", "totalThreads"]),
            (&json["settings"]["fps"], &["enabled", "limit", "effective"]),
            (&json["settings"]["windows"], &["dedicatedGpu", "windowedOptimizationsOff"]),
        ];
        for (value, fields) in expected {
            assert_eq!(names(value), fields.iter().copied().collect::<BTreeSet<_>>());
        }
        assert_eq!(names(&json["settings"]["cpu"]["priorities"][0]), BTreeSet::from(["key", "label"]));
        // Values the pages compare or display as they are keep their type.
        assert!(json["tweaks"][0]["value"].is_number() && json["tweaks"][0]["trial"].is_boolean() && json["savedPosition"].is_array());
        assert!(json["logPath"].is_null() && json["character"].is_number());
    }

    #[test]
    fn keys_and_numbers_are_written_the_way_of_each_language() {
        assert_eq!(effective_fps(240), 250);
        in_language(Lang::Fr, || {
            assert_eq!(key_label("PageUp"), "Page préc.");
            assert_eq!(key_label("End"), "Fin");
            assert_eq!(key_label("F6"), "F6");
            assert_eq!(thousands(100_000), "100\u{a0}000");
            assert_eq!(thousands(950), "950");
            assert_eq!(hint(Feature::SlowMotion, &Config::default()), "Vitesse du jeu multipliée par 0,5.");
            let keys = hotkeys(&Config::default());
            assert_eq!(keys.len(), Feature::ALL.len() + OneShot::ALL.len() + 3);
            // One-off actions are listed, without a key until the configuration gives them one.
            assert!(keys.iter().any(|hotkey| hotkey.label == "Remplir les munitions" && hotkey.key.is_empty()));
            assert_eq!((keys[0].label, keys[0].key.as_str()), ("Invincibilité", "F1"));
            assert_eq!((keys[keys.len() - 1].label, keys[keys.len() - 1].key.as_str()), ("Tout désactiver", "Fin"));
        });
        in_language(Lang::En, || {
            assert_eq!(key_label("PageUp"), "Page Up");
            assert_eq!(thousands(100_000), "100,000");
            assert_eq!(hint(Feature::SlowMotion, &Config::default()), "Game speed multiplied by 0.5.");
            let keys = hotkeys(&Config::default());
            assert_eq!((keys[0].label, keys[0].key.as_str()), ("God mode", "F1"));
            assert_eq!((keys[keys.len() - 1].label, keys[keys.len() - 1].key.as_str()), ("Turn everything off", "End"));
        });
    }

    #[test]
    fn the_accent_must_stand_out_on_a_card() {
        // Windows' original accent, readable: kept as is.
        let mut shades = [[0u8; 3]; 7];
        shades[1] = [0x60, 0xCD, 0xFF];
        assert_eq!(accent(Some(shades)), "#60cdff");
        // A dark, dull accent is lightened until it stands out, without changing hue (green here).
        shades[1] = [0x4C, 0x66, 0x60];
        let lightened = accent(Some(shades));
        let channel = |at: usize| u8::from_str_radix(&lightened[at..at + 2], 16).unwrap();
        let (r, g, b) = (channel(1), channel(3), channel(5));
        assert!(contrast([r, g, b], CARD) >= ACCENT_CONTRAST, "{lightened}");
        assert!(g > b && b > r && r > 0x4C, "{lightened}");
        // Unreadable palette: original accent.
        assert_eq!(accent(None), "#60cdff");
    }

    #[test]
    fn the_view_follows_the_game_and_says_what_to_do() {
        in_language(Lang::En, view_follows_the_game);
    }

    fn view_follows_the_game() {
        let (config, cores, graphics) = (Config { trainer: config::Trainer { enabled: true, ..Default::default() }, ..Config::default() }, [0b11, 0b1100], GraphicsSettings::default());
        let mut snapshot = Snapshot::default();
        let view = |snapshot: &Snapshot, feedback: Option<&Feedback>| {
            build(&Input { snapshot, config: &config, cores: &cores, start_tab: "accueil", accent: "#60cdff", notice: None, feedback, graphics: Some(&graphics), on_top: false, log_path: None })
        };
        // Without a game: nothing responds, and the top line says to start it. Every option stays listed.
        let idle = view(&snapshot, None);
        assert_eq!((idle.game, idle.ready, idle.message.tone), ("absent", false, "guide"));
        assert_eq!(idle.groups.iter().map(|group| group.options.len()).sum::<usize>(), Feature::ALL.len());
        assert_eq!(idle.settings.cpu.outcome, "The game will start with 2 of 4 threads.");
        assert_eq!((idle.settings.cpu.threads, idle.settings.cpu.total_threads), (2, 4));
        assert!(!idle.settings.fps.enabled);

        // In a match: options respond, and a recent refusal comes before the guidance.
        snapshot = demo_snapshot();
        // On opening, everything is off and settings are at their original value.
        let opened = view(&snapshot, None);
        assert!(opened.groups.iter().flat_map(|group| &group.options).all(|option| !option.on));
        assert!(opened.tweaks.iter().all(|tweak| tweak.value == tweak.default));
        // Option states follow what the engine publishes.
        for state in snapshot.features.iter_mut().filter(|state| [Feature::GodMode, Feature::InfiniteMoney, Feature::NoRecoil].contains(&state.feature)) {
            state.on = true;
        }
        let playing = view(&snapshot, None);
        assert_eq!((playing.game, playing.ready), ("match", true));
        let lit: Vec<&str> = playing.groups.iter().flat_map(|group| &group.options).filter(|option| option.on).map(|option| option.key).collect();
        assert_eq!(lit, ["god_mode", "no_recoil", "infinite_money"]);
        // Only three options change the game's code; ammo and one-hit kill freeze values.
        let code: Vec<&str> = playing.groups.iter().flat_map(|group| &group.options).filter(|option| option.code).map(|option| option.key).collect();
        assert_eq!(code, ["no_recoil", "rapid_fire", "low_gravity"]);
        let refused = Feedback { text: "Changement de manche impossible".into(), refused: true };
        assert_eq!(view(&snapshot, Some(&refused)).message.tone, "refused");

        // In the menus: the trainer still responds (to turn off what stayed on), the page says so.
        snapshot.game.as_mut().unwrap().in_match = false;
        let menu = view(&snapshot, None);
        assert_eq!((menu.game, menu.ready), ("menu", true));

        // Game version not recognised: nothing responds.
        snapshot.game.as_mut().unwrap().recognised = Some(false);
        assert!(!view(&snapshot, None).ready);
    }
}
