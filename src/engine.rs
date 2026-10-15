//! Engine shared by the graphical interface and the command line: it starts or waits for the game,
//! applies the settings, runs the requested actions and publishes the current state.

use crate::config::{self, Config, Priority};
use crate::lang::{tr, trs};
use crate::logbook::Logbook;
use crate::process::{self, Access, Process};
use crate::trainer::{self, Feature, FpsOutcome, OneShot, RoundSource, Trainer, Tweak};
use crate::watch::{self, Watch};
use crate::state::{self, Pending};
use crate::survey::{self, Survey};
use crate::{cpu, profile};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows_sys::Win32::System::Threading::{ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS};

const TICK: Duration = Duration::from_millis(25);
const SEARCH_INTERVAL: Duration = Duration::from_secs(2);
const RELAUNCH_GRACE: Duration = Duration::from_secs(3);
const FREEZE_INTERVAL: Duration = Duration::from_millis(100);
const WATCH_INTERVAL: Duration = Duration::from_millis(250);
const PUBLISH_INTERVAL: Duration = Duration::from_millis(200);
const STATE_INTERVAL: Duration = Duration::from_millis(500);
const AFFINITY_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const FPS_INTERVAL: Duration = Duration::from_secs(10);
/// Time given to the game to decrypt its code before concluding the version is unknown.
const RECOGNITION_DEADLINE: Duration = Duration::from_secs(90);

/// What the interface or the command line can ask the engine.
pub enum Command {
    Toggle(Feature),
    AllOff,
    SavePosition,
    Teleport,
    SetRound(i32),
    /// Sets a numeric value for the current match.
    SetTweak { tweak: Tweak, value: f32 },
    /// Runs a one-off action.
    RunAction(OneShot),
    /// Writes a full survey of everything readable into the log folder. Read-only.
    Survey,
    /// Starts or stops watching the player's structures (research).
    SetWatch(bool),
    SetFps { enabled: bool, limit: i32 },
    Verify,
    Profile(profile::Action),
}

/// How the engine gets the game.
pub enum Launch {
    /// Waits for a game started elsewhere; the affinity is then applied afterwards.
    Attach,
    /// Starts this command itself (the one Steam gives in `%command%`).
    Wrap(Vec<String>),
}

/// Starting point of the engine thread, once the game has possibly been started.
enum Start {
    Attach,
    /// The game was started by the tool: its pid and name, or nothing if starting failed.
    Launched(Option<(u32, String)>),
}

#[derive(Clone)]
pub struct Game {
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    /// Memory features (trainer, FPS limit) are allowed on this process.
    pub memory: bool,
    pub in_match: bool,
    /// `None` while the game's code could not be checked yet.
    pub recognised: Option<bool>,
    pub affinity: usize,
}

#[derive(Clone, Copy)]
pub struct FeatureState {
    pub feature: Feature,
    pub on: bool,
    pub patches_code: bool,
}

/// State published by the engine, read by the interface at every refresh.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub game: Option<Game>,
    pub trainer_enabled: bool,
    pub features: Vec<FeatureState>,
    pub saved_position: Option<[f32; 3]>,
    /// Current value of each numeric setting, read from the game; `None` outside a match.
    pub tweaks: Vec<(Tweak, Option<f32>)>,
    /// Watching of the player's structures is running.
    pub watching: bool,
    /// Code of the character requested for the next match, when the profile is readable.
    pub character: Option<u8>,
    /// In launcher mode: the game is closed, there is nothing left to follow.
    pub finished: bool,
}

pub struct Engine {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    pub log: Arc<Logbook>,
}

impl Engine {
    pub fn start(config: Config, cores: Vec<usize>, log: Arc<Logbook>, launch: Launch) -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let worker = Worker::new(config, cores, log.clone(), rx, snapshot.clone());
        // The game is started here, before returning: our own process affinity is restricted
        // during this start, and the caller must only create its own processes (the interface's
        // web view) after that moment, otherwise they would inherit it.
        let start = match launch {
            Launch::Attach => Start::Attach,
            Launch::Wrap(command) => Start::Launched(worker.spawn(&command)),
        };
        std::thread::Builder::new().name("engine".into()).spawn(move || worker.run(start)).expect("engine thread");
        Self { tx, snapshot, log }
    }

    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }
}

pub fn priority_class(priority: Priority) -> u32 {
    match priority {
        Priority::Normal => NORMAL_PRIORITY_CLASS,
        Priority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
        Priority::High => HIGH_PRIORITY_CLASS,
    }
}

#[derive(Clone, Copy)]
enum Action {
    Toggle(Feature),
    AllOff,
    SavePosition,
    Teleport,
    SetTweak(Tweak, f32),
    Run(OneShot),
}

struct Hotkey {
    vk: u16,
    action: Action,
    was_down: bool,
}

/// Everything that lives as long as a game session.
struct Session {
    game: Process,
    memory: bool,
    trainer: Option<Trainer>,
    recognised: Option<bool>,
    in_match: bool,
    started: Instant,
    /// Watching of the player's structures, started on request and stopped with the match.
    watch: Option<Watch>,
    /// Process creation time: with the pid, it says which game the state file is about.
    created: Option<u64>,
    /// What is written in the state file, so it is only rewritten when it changes.
    saved: Pending,
    /// Last full survey taken in the menus and last taken in a match, to say what changed since.
    surveys: [Option<Survey>; 2],
}

/// A game found this soon after its creation cannot have played a match yet: loading and the
/// menus take longer (measured: about twelve minutes between launch and the first match, never
/// less than a minute).
const FRESH_GAME: Duration = Duration::from_secs(60);

/// True if the game is followed from its launch: the tool started it itself, or found it right
/// after its creation. That is the condition to trust the round's fixed address, which only holds
/// for the first match: attached later, the tool cannot know how many matches took place.
/// Times are in hundreds of nanoseconds, as Windows gives them.
fn followed_from_launch(late: bool, created: Option<u64>, now: u64) -> bool {
    !late || created.is_some_and(|created| now.saturating_sub(created) <= FRESH_GAME.as_nanos() as u64 / 100)
}

struct Worker {
    config: Config,
    mask: usize,
    log: Arc<Logbook>,
    rx: Receiver<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    hotkeys: Vec<Hotkey>,
}

impl Worker {
    fn new(config: Config, cores: Vec<usize>, log: Arc<Logbook>, rx: Receiver<Command>, snapshot: Arc<Mutex<Snapshot>>) -> Self {
        let mask = cpu::build_mask(&cores, config.cpu.smt, config.cpu.max_cores);
        let mut hotkeys = Vec::new();
        let mut bind = |name: &str, action: Action| {
            if let Some(vk) = config.hotkey(name).and_then(config::key_code) {
                hotkeys.push(Hotkey { vk, action, was_down: false });
            }
        };
        for feature in Feature::ALL {
            bind(feature.key(), Action::Toggle(feature));
        }
        for action in OneShot::ALL {
            bind(action.key(), Action::Run(action));
        }
        bind(config::ACTION_ALL_OFF, Action::AllOff);
        bind(config::ACTION_SAVE_POSITION, Action::SavePosition);
        bind(config::ACTION_TELEPORT, Action::Teleport);
        Self { config, mask, log, rx, snapshot, hotkeys }
    }

    fn run(mut self, start: Start) {
        match start {
            Start::Launched(game) => {
                if let Some((pid, name)) = game {
                    self.follow(pid, name, false);
                }
                // The game may have restarted itself: we follow the new process if there is one.
                sleep(RELAUNCH_GRACE);
                while let Some((pid, name)) = process::find(&self.config.process_names) {
                    if !self.follow(pid, name, true) {
                        sleep(SEARCH_INTERVAL);
                    }
                }
                self.snapshot.lock().unwrap().finished = true;
            }
            Start::Attach => {
                self.log.line(trs(format!("Waiting for {}.", self.config.process_names.join(" or ")), format!("En attente de {}.", self.config.process_names.join(" ou "))));
                loop {
                    match process::find(&self.config.process_names) {
                        Some((pid, name)) => {
                            if !self.follow(pid, name, true) {
                                sleep(SEARCH_INTERVAL);
                            }
                        }
                        None => self.wait_without_game(),
                    }
                }
            }
        }
    }

    /// Starts the game as a child process, so it inherits the affinity from its creation.
    ///
    /// Applied afterwards, the affinity makes the game engine measure two different CPUs (at
    /// startup, then during play) and it asks for "optimal settings" again at every launch.
    fn spawn(&self, command: &[String]) -> Option<(u32, String)> {
        let mut game = std::process::Command::new(&command[0]);
        game.args(&command[1..]);
        if self.config.cpu.enabled {
            if let Err(e) = process::set_current_affinity(self.mask) {
                self.log.line(trs(format!("Affinity not applied: {e}"), format!("Affinité non appliquée : {e}")));
            }
            game.creation_flags(priority_class(self.config.cpu.priority));
        }
        let spawned = game.spawn();
        // Our own process has no reason to stay restricted to the game's cores anymore.
        if let Ok(all) = process::system_affinity() {
            let _ = process::set_current_affinity(all);
        }
        match spawned {
            Ok(child) => {
                let name = Path::new(&command[0]).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                self.log.line(trs(format!("{name} started (pid {}), affinity set from the start.", child.id()), format!("{name} lancé (pid {}), affinité en place dès le démarrage.", child.id())));
                Some((child.id(), name))
            }
            Err(e) => {
                self.log.line(trs(format!("Could not start {}: {e}", command[0]), format!("Lancement de {} impossible : {e}", command[0])));
                None
            }
        }
    }

    /// Waits for a game to appear, answering the requests that do not need one.
    fn wait_without_game(&mut self) {
        match self.rx.recv_timeout(SEARCH_INTERVAL) {
            Ok(Command::SetFps { enabled, limit }) => {
                self.config.fps.enabled = enabled;
                self.config.fps.limit = limit;
            }
            Ok(_) => self.log.line(tr("The game is not running.", "Le jeu n'est pas lancé.")),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => sleep(SEARCH_INTERVAL),
        }
    }

    /// Follows the game until it closes. Returns false if the process could not be opened.
    fn follow(&mut self, pid: u32, name: String, late: bool) -> bool {
        let memory = self.config.memory_allowed_for(&name) && self.config.wants_memory();
        let access = if memory { Access::Memory } else { Access::SchedulingOnly };
        let game = match Process::open(pid, name, access) {
            Ok(game) => game,
            Err(e) => {
                self.log.line(trs(format!("Could not open process {pid}: {e}"), format!("Ouverture du processus {pid} impossible : {e}")));
                return false;
            }
        };
        self.log.line(trs(format!("{} found (pid {}).", game.name, game.pid), format!("{} détecté (pid {}).", game.name, game.pid)));
        if self.config.wants_memory() && !memory {
            self.log.line(trs(format!("FPS limit and trainer skipped: {} is VAC-protected (memory.allow_steam_client).", game.name), format!("FPS et trainer ignorés : {} est protégé par VAC (memory.allow_steam_client).", game.name)));
        }
        if self.config.cpu.enabled {
            if late {
                self.log.line(
                    "Affinité appliquée après le démarrage : le jeu risque de redemander les « paramètres optimaux ». \
                     Pour l'éviter, passer par les options de lancement Steam.",
                );
            }
            match game.set_priority_class(priority_class(self.config.cpu.priority)) {
                Ok(()) => self.log.line(trs(format!("{} priority applied.", self.config.cpu.priority.label()), format!("Priorité {} appliquée.", self.config.cpu.priority.label()))),
                Err(e) => self.log.line(trs(format!("Priority not applied: {e}"), format!("Priorité non appliquée : {e}"))),
            }
        }
        let created = game.creation_time();
        let from_launch = followed_from_launch(late, created, process::now());
        let trainer = (memory && self.config.trainer.enabled).then(|| Trainer::new(self.config.trainer.timescale, self.config.trainer.money).followed_from_launch(from_launch));
        if trainer.is_some() && !from_launch {
            self.log.line(tr("Game was already running before the tool: to change the round on maps where the round is not found by name, restart the game with the tool open, then play a first match.", "Jeu déjà lancé avant l'outil : pour changer de manche, il faudra relancer le jeu avec l'outil ouvert, puis faire une première partie."));
        }
        let mut session =
            Session { game, memory, trainer, recognised: None, in_match: false, started: Instant::now(), watch: None, created, saved: Pending::default(), surveys: [None, None] };
        self.attend(&mut session);
        self.close(session);
        true
    }

    fn attend(&mut self, s: &mut Session) {
        let start = Instant::now();
        let reapply = Duration::from_secs(self.config.cpu.reapply_minutes * 60);
        let mut next_affinity_check = start;
        let mut next_reapply = start + reapply;
        let mut next_fps = start;
        let mut next_freeze = start;
        let mut next_watch = start;
        let mut next_state = start;
        let mut next_publish = start;

        while s.game.is_running() {
            let now = Instant::now();
            if self.config.cpu.enabled && now >= next_affinity_check {
                next_affinity_check = now + AFFINITY_CHECK_INTERVAL;
                // The mask is checked again rather than set once: the engine may reset it while loading.
                if s.game.affinity().ok() != Some(self.mask) {
                    match s.game.set_affinity(self.mask) {
                        Ok(()) => self.log.line(trs(format!("Affinity {:#x} applied ({} threads).", self.mask, self.mask.count_ones()), format!("Affinité {:#x} appliquée ({} threads).", self.mask, self.mask.count_ones()))),
                        Err(e) => self.log.line(trs(format!("Affinity not applied: {e}"), format!("Affinité non appliquée : {e}"))),
                    }
                }
                if !reapply.is_zero() && now >= next_reapply {
                    next_reapply = now + reapply;
                    self.kick_affinity(&s.game);
                }
            }
            if now >= next_state {
                next_state = now + STATE_INTERVAL;
                self.update_state(s);
            }
            if s.memory && self.config.fps.enabled && s.recognised == Some(true) && now >= next_fps {
                next_fps = now + FPS_INTERVAL;
                if let FpsOutcome::Set { previous, limit } = trainer::set_fps_limit(&s.game, self.config.fps.limit) {
                    self.log.line(trs(format!("FPS limit set ({previous} -> {limit})."), format!("Limite de FPS réglée ({previous} -> {limit}).")));
                }
            }
            self.poll_hotkeys(s);
            while let Ok(command) = self.rx.try_recv() {
                if matches!(command, Command::SetFps { .. }) {
                    next_fps = now;
                }
                self.handle(s, command);
                // The interface reads the state shortly after a click: it must already reflect the request.
                next_publish = now;
            }
            if now >= next_freeze {
                next_freeze = now + FREEZE_INTERVAL;
                if let Some(trainer) = &mut s.trainer {
                    trainer.apply_frozen_values(&s.game);
                }
            }
            if now >= next_watch {
                next_watch = now + WATCH_INTERVAL;
                if let Some(watch) = s.watch.as_mut() {
                    for line in watch.poll(&s.game) {
                        self.log.line(line);
                    }
                }
            }
            if now >= next_publish {
                next_publish = now + PUBLISH_INTERVAL;
                self.publish(s);
            }
            sleep(TICK);
        }
    }

    /// Recognising the game version, and detecting when matches start and end.
    fn update_state(&mut self, s: &mut Session) {
        if !s.memory {
            return;
        }
        if s.recognised != Some(true) {
            if trainer::build_is_recognised(&s.game) {
                s.recognised = Some(true);
                self.log.line(tr("Game version recognised.", "Version du jeu reconnue."));
                // Once per followed game, and before the user can turn anything on (nothing
                // responds until the version is recognised): on opening, everything is off.
                if let Some(trainer) = s.trainer.as_mut() {
                    // The state file says what the tool itself left in THIS process; it is deleted
                    // as soon as it is read. Without it, only the two exact-value restores happen.
                    let pending = match (self.log.dir(), s.created) {
                        (Some(dir), Some(created)) => state::take(&dir, s.game.pid, created),
                        _ => Pending::default(),
                    };
                    let restored = trainer.reset_leftovers(&s.game, &pending);
                    if !restored.is_empty() {
                        self.log.line(trs(format!("Options left on by a previous session: restored ({}).", restored.join(", ")), format!("Options restées actives d'une session précédente : remises à l'état d'origine ({}).", restored.join(", "))));
                    }
                }
            } else if s.recognised.is_none() && s.started.elapsed() > RECOGNITION_DEADLINE {
                s.recognised = Some(false);
                self.log.line(tr("Game version not recognised: the trainer and FPS limit stay off.", "Version du jeu non reconnue : le trainer et la limite de FPS restent inactifs."));
            }
        }
        let in_match = trainer::in_match(&s.game);
        if in_match && !s.in_match {
            self.log.line(tr("Match detected.", "Partie détectée."));
            if let Some(trainer) = s.trainer.as_mut() {
                trainer.match_started();
            }
        } else if !in_match && s.in_match {
            s.watch = None;
            // The engine speed and the suit setting do not go away with the match: they are restored.
            if let Some(trainer) = s.trainer.as_mut() {
                trainer.match_ended(&s.game);
            }
            self.sync_state(s);
            self.log.line(tr("Match ended.", "Partie terminée."));
        }
        s.in_match = in_match;
        if in_match {
            // The round variable can only be captured during the first match: we watch for it.
            if let Some((name, neighbours)) = s.trainer.as_mut().and_then(|trainer| trainer.learn_round_variable(&s.game)) {
                self.log.line(trs(
                    format!("Round variable captured (name {name:#x}) with {neighbours} neighbouring fields: they will be used to find it in later matches."),
                    format!("Variable de la manche relevée (nom {name:#x}), avec {neighbours} champs voisins : elle sera cherchée par eux aux parties suivantes."),
                ));
            }
        }
    }

    fn log_verification(&self, game: &Process) {
        let (_, report) = trainer::verify(game);
        self.log.line(tr("--- Address check ---", "--- Contrôle des adresses ---"));
        for line in report {
            self.log.line(if line.is_empty() { " " } else { &line });
        }
        self.log.line(tr("--- End of check ---", "--- Fin du contrôle ---"));
    }

    fn poll_hotkeys(&mut self, s: &mut Session) {
        let focused = process::is_foreground(s.game.pid);
        let mut pressed = Vec::new();
        for hotkey in &mut self.hotkeys {
            let down = process::key_down(hotkey.vk);
            // Keys only count while the game is in front.
            if down && !hotkey.was_down && focused {
                pressed.push(hotkey.action);
            }
            hotkey.was_down = down;
        }
        for action in pressed {
            self.perform(s, action);
        }
    }

    fn handle(&mut self, s: &mut Session, command: Command) {
        match command {
            Command::Toggle(feature) => self.perform(s, Action::Toggle(feature)),
            Command::AllOff => self.perform(s, Action::AllOff),
            Command::SavePosition => self.perform(s, Action::SavePosition),
            Command::Teleport => self.perform(s, Action::Teleport),
            Command::SetFps { enabled, limit } => {
                self.config.fps.enabled = enabled;
                self.config.fps.limit = limit;
            }
            Command::Verify => {
                if s.memory {
                    self.log_verification(&s.game);
                } else {
                    self.log.line(tr("Check impossible: memory reading is not allowed on this process.", "Contrôle impossible : la lecture mémoire n'est pas autorisée sur ce processus."));
                }
            }
            Command::SetRound(round) => self.set_round(s, round),
            Command::SetTweak { tweak, value } => self.perform(s, Action::SetTweak(tweak, value)),
            Command::RunAction(action) => self.perform(s, Action::Run(action)),
            Command::Survey => self.survey(s),
            Command::SetWatch(on) => {
                if on && !(s.memory && s.in_match) {
                    self.log.line(tr("Watching is only possible during a match.", "Surveillance impossible hors partie."));
                } else if on {
                    s.watch = Some(Watch::default());
                    self.log.line(trs(format!("Watching started: wait {} seconds before doing anything in game.", watch::LEARNING.as_secs()), format!("Surveillance démarrée : attendre {} secondes avant d'agir en jeu.", watch::LEARNING.as_secs())));
                } else if s.watch.take().is_some() {
                    self.log.line(tr("Watching stopped.", "Surveillance arrêtée."));
                }
            }
            Command::Profile(action) => self.edit_profile(s, action),
        }
    }

    fn perform(&mut self, s: &mut Session, action: Action) {
        let Some(trainer) = s.trainer.as_mut() else {
            self.log.line(tr("Trainer off: check trainer.enabled and memory.allow_steam_client.", "Trainer inactif : vérifier trainer.enabled et memory.allow_steam_client."));
            return;
        };
        if s.recognised != Some(true) {
            self.log.line(tr("Game version not recognised (or not checked yet): nothing is changed.", "Version du jeu non reconnue (ou pas encore contrôlée) : rien n'est modifié."));
            return;
        }
        let (message, tone) = match action {
            Action::Toggle(feature) => match trainer.toggle(&s.game, feature) {
                Ok(on) => (trs(format!("{}: {}", feature.label(), if on { "ON" } else { "OFF" }), format!("{} : {}", feature.label(), if on { "ON" } else { "OFF" })), on),
                Err(why) => (trs(format!("{} unavailable ({why})", feature.label()), format!("{} indisponible ({why})", feature.label())), false),
            },
            Action::AllOff => {
                trainer.all_off(&s.game);
                (tr("Everything is off.", "Tout est désactivé.").to_string(), false)
            }
            Action::SavePosition => match trainer.save_position(&s.game) {
                Ok(position) => (trs(format!("Position saved: {position:.1?}"), format!("Position enregistrée : {position:.1?}")), true),
                Err(why) => (trs(format!("Position unavailable ({why})"), format!("Position indisponible ({why})")), false),
            },
            Action::Teleport => match trainer.teleport(&s.game) {
                Ok(position) => (trs(format!("Teleported to {position:.1?}"), format!("Téléporté en {position:.1?}")), true),
                Err(why) => (trs(format!("Teleport unavailable ({why})"), format!("Téléportation indisponible ({why})")), false),
            },
            Action::SetTweak(tweak, value) => match trainer.set_tweak(&s.game, tweak, value) {
                Ok((previous, written)) => (trs(format!("{}: {previous} -> {written}", tweak.label()), format!("{} : {previous} -> {written}", tweak.label())), true),
                Err(why) => (trs(format!("{} unavailable ({why})", tweak.label()), format!("{} indisponible ({why})", tweak.label())), false),
            },
            Action::Run(action) => match trainer.run_action(&s.game, action) {
                Ok(detail) => (trs(format!("{}: done ({detail})", action.label()), format!("{} : fait ({detail})", action.label())), true),
                Err(why) => (trs(format!("{} unavailable ({why})", action.label()), format!("{} indisponible ({why})", action.label())), false),
            },
        };
        self.log.line(message);
        self.sync_state(s);
        if self.config.trainer.sound {
            trainer::beep(tone);
        }
    }

    /// Full survey of what can be read, in the menus or in a match. The game is opened again for
    /// it with read access only: this pass cannot write anything.
    fn survey(&mut self, s: &mut Session) {
        let refusal = if !s.memory {
            Some(tr("memory reading is not allowed on this process", "la lecture mémoire n'est pas autorisée sur ce processus").to_string())
        } else if s.recognised != Some(true) {
            Some(tr("game version not recognised, or not checked yet", "version du jeu non reconnue, ou pas encore contrôlée").to_string())
        } else {
            None
        };
        let outcome = match (refusal, self.log.dir()) {
            (Some(why), _) => Err(why),
            (None, None) => Err(tr("no log folder to write it to", "aucun dossier de journaux où l'écrire").to_string()),
            (None, Some(dir)) => Process::open(s.game.pid, s.game.name.clone(), Access::MemoryRead).map_err(|e| e.to_string()).and_then(|reader| {
                let taken = survey::take(&reader);
                let slot = usize::from(taken.in_match);
                let written = survey::write(&dir, &taken, s.surveys[slot].as_ref(), &s.game.name, s.game.pid, true);
                s.surveys[slot] = Some(taken);
                written
            }),
        };
        match outcome {
            Ok(name) => self.log.line(trs(format!("Survey written: {name}"), format!("Relevé écrit : {name}"))),
            Err(why) => self.log.line(trs(format!("Survey impossible ({why})."), format!("Relevé impossible ({why})."))),
        }
    }

    /// Keeps the state file up to date: what the tool turned on in this game and has not restored
    /// yet. It is only rewritten when that list changes, and deleted when it is empty.
    fn sync_state(&self, s: &mut Session) {
        let Some(trainer) = &s.trainer else { return };
        let pending = trainer.pending();
        if pending == s.saved {
            return;
        }
        if let (Some(dir), Some(created)) = (self.log.dir(), s.created) {
            state::save(&dir, s.game.pid, created, &pending);
        }
        s.saved = pending;
    }

    fn set_round(&mut self, s: &mut Session, round: i32) {
        let Some(trainer) = s.trainer.as_mut().filter(|_| s.recognised == Some(true)) else {
            self.log.line(tr("Round change impossible: trainer off or game version not recognised.", "Changement de manche impossible : trainer inactif ou version du jeu non reconnue."));
            return;
        };
        match trainer.set_round(&s.game, round) {
            Ok((previous, source)) => {
                let how = match source {
                    RoundSource::Known | RoundSource::Fixed => String::new(),
                    RoundSource::Name => tr(" Variable found elsewhere in the script table, under the same name.", " Variable retrouvée ailleurs dans la table des scripts, sous le même nom.").to_string(),
                    RoundSource::Neighbours { shift, matched, of } => {
                        trs(format!(" Variable recognised by its neighbouring fields ({matched} of {of}), its name having shifted by {shift:+}."), format!(" Variable reconnue à ses champs voisins ({matched} sur {of}), son nom ayant changé de {shift:+}."))
                    }
                };
                self.log.line(trs(format!("Round: {previous} -> {round}.{how}"), format!("Manche : {previous} -> {round}.{how}")));
            }
            Err(why) => self.log.line(trs(format!("Round change impossible ({why})."), format!("Changement de manche impossible ({why})."))),
        }
    }

    fn edit_profile(&mut self, s: &mut Session, action: profile::Action) {
        if !s.memory || s.recognised != Some(true) {
            self.log.line(tr("Profile unavailable: memory access not allowed or game version not recognised.", "Profil inaccessible : accès mémoire non autorisé ou version du jeu non reconnue."));
            return;
        }
        self.log.line(trs(format!("Profile: {}", action.label()), format!("Profil : {}", action.label())));
        match profile::run(&s.game, action) {
            Ok(lines) => lines.iter().for_each(|line| self.log.line(line)),
            Err(why) => self.log.line(why),
        }
    }

    /// Forces the scheduler to move the game's threads by briefly removing one logical processor.
    fn kick_affinity(&self, game: &Process) {
        let without_lowest = self.mask & (self.mask - 1);
        if without_lowest != 0 && game.set_affinity(without_lowest).is_ok() {
            sleep(Duration::from_millis(200));
            if game.set_affinity(self.mask).is_ok() {
                self.log.line(tr("Affinity re-applied.", "Affinité ré-appliquée."));
            }
        }
    }

    fn publish(&self, s: &Session) {
        let features = match &s.trainer {
            Some(trainer) => Feature::ALL
                .iter()
                .map(|&feature| FeatureState { feature, on: trainer.is_on(&s.game, feature), patches_code: feature.patches_code() })
                .collect(),
            None => Vec::new(),
        };
        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.game = Some(Game {
            pid: s.game.pid,
            name: s.game.name.clone(),
            exe: s.game.image_path(),
            memory: s.memory,
            in_match: s.in_match,
            recognised: s.recognised,
            affinity: s.game.affinity().unwrap_or(0),
        });
        snapshot.trainer_enabled = s.trainer.is_some();
        snapshot.features = features;
        snapshot.saved_position = s.trainer.as_ref().and_then(Trainer::saved_position);
        snapshot.tweaks = match &s.trainer {
            Some(trainer) => Tweak::ALL.iter().map(|&tweak| (tweak, trainer.tweak_value(&s.game, tweak))).collect(),
            None => Vec::new(),
        };
        snapshot.watching = s.watch.is_some();
        snapshot.character = if s.memory && s.recognised == Some(true) { profile::read_character(&s.game) } else { None };
    }

    fn close(&mut self, s: Session) {
        self.log.line(trs(format!("{} closed.", s.game.name), format!("{} fermé.", s.game.name)));
        // The game is closed: there is nothing left in it to restore.
        if let (Some(dir), Some(created), false) = (self.log.dir(), s.created, s.saved.is_empty()) {
            state::save(&dir, s.game.pid, created, &Pending::default());
        }
        *self.snapshot.lock().unwrap() = Snapshot::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_game_followed_from_its_launch_can_teach_the_round() {
        const SECOND: u64 = 10_000_000;
        let created = 133_000_000_000_000_000;
        // Started by the tool: followed from the start, whatever the clock says.
        assert!(followed_from_launch(false, None, 0));
        // Found right after its creation (the game restarts itself at startup): same.
        assert!(followed_from_launch(true, Some(created), created + 30 * SECOND));
        // Found later, or without being able to read its creation time: matches may have happened.
        assert!(!followed_from_launch(true, Some(created), created + 61 * SECOND));
        assert!(!followed_from_launch(true, None, created));
        // This test's process exists since recently, and its creation time is readable.
        let me = Process::open(std::process::id(), "self".into(), Access::SchedulingOnly).unwrap();
        assert!(me.creation_time().is_some_and(|created| created <= process::now()));
    }

    #[test]
    fn engine_reports_no_game_and_accepts_settings() {
        let config = Config { process_names: vec!["jeu-qui-n-existe-pas.exe".into()], ..Config::default() };
        let engine = Engine::start(config, vec![1, 2], Logbook::open(None, false), Launch::Attach);
        // A setting is accepted silently; an action on the game says it is not running.
        engine.send(Command::SetFps { enabled: true, limit: 200 });
        engine.send(Command::Verify);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && engine.log.recent().len() < 2 {
            sleep(Duration::from_millis(20));
        }
        let snapshot = engine.snapshot();
        assert!(snapshot.game.is_none() && !snapshot.finished);
        let refusals = engine.log.recent().iter().filter(|line| line.contains("The game is not running")).count();
        assert_eq!(refusals, 1);
    }
}
