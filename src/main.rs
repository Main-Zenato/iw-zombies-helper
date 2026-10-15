//! Command line: one-off checks (info, verify, profile) and following the game without a window.

use iw_helper::config::{self, Config};
use iw_helper::engine::{Engine, Launch};
use iw_helper::logbook::Logbook;
use iw_helper::process::{self, Access, Process};
use iw_helper::lang::{self, tr, trs};
use iw_helper::{cpu, profile, survey, trainer};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread::sleep;
use std::time::Duration;

const STEAM_LAUNCH_URL: &str = "steam://rungameid/292730";

const USAGE: &str = "\
iw-helper - CPU fix, FPS limit and Zombies trainer for Call of Duty: Infinite Warfare

The graphical interface is iw-helper-gui.exe; this program does the same without a window.

USAGE:
    iw-helper [--config <file>] -- <game command>
    iw-helper [run] [--config <file>] [--launch]
    iw-helper info [--config <file>]
    iw-helper verify [--config <file>]
    iw-helper survey [--config <file>]
    iw-helper profile <action> [--confirm] [--config <file>]

    -- <cmd>   Starts the game itself, with the CPU affinity from the start (Steam launch options)
    run        Waits for a running game and applies the configuration (default)
    info       Shows the CPU topology and what would be applied, without changing anything
    verify     Read-only check of the trainer addresses on the running game
    survey     Full read-only survey: writes everything the tool can read from the running game
               (profile, engine settings, scripts, player) into the logs folder. Written in French.
    profile    Reads or changes the Zombies profile; changes are permanent.
               Actions: show | max-level | weapon-xp | directors-cut | contracts | reroll-contracts |
               character-0 to character-5 (character for the next match) | reset
    --launch   With `run`: asks Steam to start the game (affinity applied afterwards)
    --confirm  Required for `profile reset`
";

const USAGE_FR: &str = "\
iw-helper - correctif CPU, limite de FPS et trainer Zombies pour Infinite Warfare

L'interface graphique est iw-helper-gui.exe ; ce programme-ci fait la même chose sans fenêtre.

USAGE:
    iw-helper [--config <fichier>] -- <commande du jeu>
    iw-helper [run] [--config <fichier>] [--launch]
    iw-helper info [--config <fichier>]
    iw-helper verify [--config <fichier>]
    iw-helper survey [--config <fichier>]
    iw-helper profile <action> [--confirm] [--config <fichier>]

    -- <cmd>   Lance le jeu lui-même, avec l'affinité CPU dès le démarrage (options de lancement Steam)
    run        Attend un jeu déjà lancé et applique la configuration (par défaut)
    info       Affiche la topologie CPU et ce qui serait appliqué, sans rien modifier
    verify     Contrôle en lecture seule les adresses du trainer sur le jeu en cours
    survey     Relevé complet, en lecture seule : écrit dans le dossier des journaux tout ce que
               l'outil sait lire du jeu en cours (profil, réglages du moteur, scripts, joueur)
    profile    Lit ou modifie le profil Zombies ; modifications définitives.
               Actions : show | max-level | weapon-xp | directors-cut | contracts | reroll-contracts |
               character-0 à character-5 (personnage de la prochaine partie) | reset
    --launch   Avec `run` : demande à Steam de lancer le jeu (affinité appliquée après coup)
    --confirm  Requis pour `profile reset`
";

fn usage() -> &'static str {
    tr(USAGE, USAGE_FR)
}

fn main() -> ExitCode {
    let mut command = "run".to_string();
    let mut config_path = None;
    let mut launch = false;
    let mut confirm = false;
    let mut profile_action = None;
    let mut game_command: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "run" | "info" | "verify" | "survey" => command = arg,
            "profile" => {
                profile_action = args.next().as_deref().and_then(profile::Action::parse);
                if profile_action.is_none() {
                    return usage_error(&trs(format!("profile expects an action: {}", profile::Action::NAMES), format!("profile attend une action : {}", profile::Action::NAMES)));
                }
                command = arg;
            }
            "--" => {
                game_command = args.by_ref().collect();
                if game_command.is_empty() {
                    return usage_error(tr("-- expects the game command", "-- attend la commande du jeu"));
                }
            }
            "--launch" => launch = true,
            "--confirm" => confirm = true,
            "--config" => match args.next() {
                Some(path) => config_path = Some(PathBuf::from(path)),
                None => return usage_error(tr("--config expects a path", "--config attend un chemin")),
            },
            "-h" | "--help" => {
                print!("{}", usage());
                return ExitCode::SUCCESS;
            }
            other => return usage_error(&trs(format!("unknown argument: {other}"), format!("argument inconnu : {other}"))),
        }
    }

    let (config, source) = match config::load(config_path.as_deref()) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("{}", trs(format!("Invalid configuration - {e}"), format!("Configuration invalide - {e}")));
            return ExitCode::FAILURE;
        }
    };
    lang::set(config.interface.language);
    match &source {
        Some(path) => println!("{}", trs(format!("Configuration: {}", path.display()), format!("Configuration : {}", path.display()))),
        None => println!("{}", trs(format!("No {} found: default settings (CPU fix only).", config::FILE_NAME), format!("Aucun {} trouvé : réglages par défaut (correctif CPU seul).", config::FILE_NAME))),
    }
    let logs = logs_dir(source.as_deref());
    if command == "verify" {
        return verify(&config);
    }
    if command == "survey" {
        return take_survey(&config, &logs);
    }
    if let Some(action) = profile_action {
        return edit_profile(&config, action, confirm);
    }

    let cores = match cpu::physical_cores() {
        Ok(cores) => cores,
        Err(e) => {
            eprintln!("{}", trs(format!("Could not read the CPU topology: {e}"), format!("Lecture de la topologie CPU impossible : {e}")));
            return ExitCode::FAILURE;
        }
    };
    print_plan(&config, &cores);
    if command == "info" {
        return ExitCode::SUCCESS;
    }

    let wrapped = !game_command.is_empty();
    if launch && !wrapped {
        launch_game();
    }
    let log = Logbook::open(Some(&logs), true);
    let mode = if wrapped { Launch::Wrap(game_command) } else { Launch::Attach };
    let engine = Engine::start(config, cores, log, mode);
    // Keyboard shortcuts stay active; in launcher mode, we return once the game closes.
    while !engine.snapshot().finished {
        sleep(Duration::from_millis(250));
    }
    ExitCode::SUCCESS
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}\n\n{}", usage());
    ExitCode::FAILURE
}

/// Log folder: next to the configuration file, otherwise next to the executable.
fn logs_dir(config_source: Option<&Path>) -> PathBuf {
    let base = config_source.and_then(Path::parent).map(Path::to_path_buf).or_else(|| config::default_path().parent().map(Path::to_path_buf));
    base.unwrap_or_default().join("logs")
}

fn print_plan(config: &Config, cores: &[usize]) {
    let mask = cpu::build_mask(cores, config.cpu.smt, config.cpu.max_cores);
    let logical: u32 = cores.iter().map(|c| c.count_ones()).sum();
    println!("{}", trs(format!("CPU: {} physical cores, {logical} logical threads", cores.len()), format!("CPU : {} coeurs physiques, {logical} threads logiques", cores.len())));
    if config.cpu.enabled {
        println!("{}", trs(format!("CPU fix: affinity {mask:#x} ({} threads), {} priority", mask.count_ones(), config.cpu.priority.label()), format!("Correctif CPU : affinité {mask:#x} ({} threads), priorité {}", mask.count_ones(), config.cpu.priority.label())));
    } else {
        println!("{}", tr("CPU fix: off", "Correctif CPU : désactivé"));
    }
    let memory = |on: bool| match (on, config.memory.allow_steam_client) {
        (false, _) => tr("off", "désactivé"),
        (true, true) => tr("on (Steam client included: VAC ban risk)", "activé (client Steam inclus : risque de ban VAC)"),
        (true, false) => tr("on for iw7-mod only", "activé pour iw7-mod uniquement"),
    };
    let limit = if config.fps.enabled { format!("{} fps, {}", config.fps.limit, memory(true)) } else { memory(false).to_string() };
    println!("{}", trs(format!("FPS limit: {limit}"), format!("Limite de FPS : {limit}")));
    println!("{}", trs(format!("Zombies trainer: {}", memory(config.trainer.enabled)), format!("Trainer Zombies : {}", memory(config.trainer.enabled))));
}

fn launch_game() {
    println!("{}", tr("Starting the game through Steam...", "Lancement du jeu via Steam..."));
    if let Err(e) = std::process::Command::new("cmd").args(["/C", "start", "", STEAM_LAUNCH_URL]).status() {
        eprintln!("{}", trs(format!("Could not start: {e}"), format!("Lancement impossible : {e}")));
    }
}

/// Opens the running game for a one-off command, after the permission check.
fn open_game(config: &Config, access: Access) -> Result<Process, ExitCode> {
    let Some((pid, name)) = process::find(&config.process_names) else {
        eprintln!("{}", trs(format!("Game not found: start {} first.", config.process_names.join(" or ")), format!("Jeu introuvable : lancer {} d'abord.", config.process_names.join(" ou "))));
        return Err(ExitCode::FAILURE);
    };
    if !config.memory_allowed_for(&name) {
        eprintln!("{}", trs(format!("{name} is VAC-protected: accessing it requires memory.allow_steam_client = true."), format!("{name} est protégé par VAC : y accéder demande memory.allow_steam_client = true.")));
        return Err(ExitCode::FAILURE);
    }
    Process::open(pid, name, access).map_err(|e| {
        eprintln!("{}", trs(format!("Could not open process {pid}: {e}"), format!("Ouverture du processus {pid} impossible : {e}")));
        ExitCode::FAILURE
    })
}

/// Read-only check, on the running game, of every address the trainer uses.
fn verify(config: &Config) -> ExitCode {
    let game = match open_game(config, Access::MemoryRead) {
        Ok(game) => game,
        Err(code) => return code,
    };
    println!("{}", trs(format!("Checking {} (pid {}), read-only.\n", game.name, game.pid), format!("Vérification de {} (pid {}), lecture seule.\n", game.name, game.pid)));
    let (recognised, report) = trainer::verify(&game);
    report.iter().for_each(|line| println!("{line}"));
    if recognised {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Full survey of what can be read from the running game, written into the log folder.
/// Read-only. Each call stands alone: comparing with the previous survey only exists in the
/// interface, which keeps the last survey in memory.
fn take_survey(config: &Config, logs: &Path) -> ExitCode {
    let game = match open_game(config, Access::MemoryRead) {
        Ok(game) => game,
        Err(code) => return code,
    };
    let recognised = trainer::build_is_recognised(&game);
    match survey::write(logs, &survey::take(&game), None, &game.name, game.pid, recognised) {
        Ok(name) => {
            println!("{}", trs(format!("Survey written: {}", logs.join(&name).display()), format!("Relevé écrit : {}", logs.join(&name).display())));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", trs(format!("Survey impossible: {e}"), format!("Relevé impossible : {e}")));
            ExitCode::FAILURE
        }
    }
}

/// Reads or changes the running game's persistent Zombies profile.
fn edit_profile(config: &Config, action: profile::Action, confirm: bool) -> ExitCode {
    if action == profile::Action::Reset && !confirm {
        eprintln!("{}", tr("`profile reset` permanently erases the account progress. Add --confirm to do it.", "`profile reset` efface définitivement la progression du compte. Ajouter --confirm pour le faire."));
        return ExitCode::FAILURE;
    }
    let access = if action.writes() { Access::Memory } else { Access::MemoryRead };
    let game = match open_game(config, access) {
        Ok(game) => game,
        Err(code) => return code,
    };
    if !trainer::build_is_recognised(&game) {
        eprintln!("{}", tr("Game version not recognised: nothing is changed (see `iw-helper verify`).", "Version du jeu non reconnue : rien n'est modifié (voir `iw-helper verify`)."));
        return ExitCode::FAILURE;
    }
    match profile::run(&game, action) {
        Ok(lines) => {
            lines.iter().for_each(|line| println!("{line}"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
