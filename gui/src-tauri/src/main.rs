//! Graphical interface: a Tauri window. The engine (the `iw_helper` library) runs inside this
//! program; the pages, written in TypeScript in gui/src, ask it for the state to show and pass the
//! clicks on to it. This is the program the Steam launch options start.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod view;

use iw_helper::config::{self, Config, Priority, Setting};
use iw_helper::engine::{Command, Engine, Launch, Snapshot};
use iw_helper::lang::{self, tr, trs, Lang};
use iw_helper::logbook::Logbook;
use iw_helper::trainer::{Feature, OneShot, Tweak};
use iw_helper::winsettings::{self, GraphicsSettings};
use iw_helper::{cpu, profile};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Manager, State};

/// How long the result of the last action stays shown at the top.
const FEEDBACK_SHOWN: Duration = Duration::from_secs(7);
const FINISHED_CHECK: Duration = Duration::from_millis(500);
const TABS: [&str; 6] = ["accueil", "salon", "partie", "compte", "reglages", "journal"];

struct App {
    engine: Option<Engine>,
    log: Arc<Logbook>,
    config_path: PathBuf,
    /// Session log folder.
    logs: PathBuf,
    cores: Vec<usize>,
    start_tab: String,
    /// Interface accent color, taken from Windows'.
    accent: String,
    /// Fake state of `--demo`: nothing is started, read or saved.
    demo: bool,
    inner: Mutex<Inner>,
}

/// What changes while the window is open.
struct Inner {
    config: Config,
    game_exe: Option<String>,
    graphics: GraphicsSettings,
    /// Last log line already handled, so an action is only reported once.
    last_log_line: Option<String>,
    feedback: Option<(view::Feedback, Instant)>,
    /// Saving or settings error, shown until the next one.
    notice: Option<String>,
    on_top: bool,
    demo: Option<Snapshot>,
}

impl App {
    fn send(&self, command: Command) {
        if let Some(engine) = &self.engine {
            engine.send(command);
        }
    }

    /// In demo mode there is no engine to answer: the line stands for the result, and shows at the
    /// top as a real action's would.
    fn demo_line(&self, text: impl AsRef<str>) {
        if self.demo {
            self.log.line(text);
        }
    }

    /// Changes the configuration in memory then saves it; any error shows at the top.
    fn set(&self, table: &str, key: &str, value: Setting, change: impl FnOnce(&mut Config)) {
        let mut inner = self.inner.lock().unwrap();
        change(&mut inner.config);
        if !self.demo {
            inner.notice = config::save(&self.config_path, table, key, value).err().map(|e| trs(format!("Setting not saved: {e}"), format!("Réglage non enregistré : {e}")));
        }
    }
}

impl Inner {
    /// Finds the last action's result in the log, to show it at the top.
    fn watch_journal(&mut self, lines: &[String]) {
        if lines.last() == self.last_log_line.as_ref() {
            return;
        }
        let seen = self.last_log_line.as_ref().and_then(|last| lines.iter().rposition(|line| line == last));
        let fresh = &lines[seen.map_or(0, |index| index + 1)..];
        if let Some(feedback) = fresh.iter().rev().find_map(|line| view::outcome(line)) {
            self.feedback = Some((feedback, Instant::now()));
        }
        self.last_log_line = lines.last().cloned();
    }
}

// --- What the pages read ----------------------------------------------------------------------

#[tauri::command]
fn view(app: State<App>) -> view::View {
    let mut inner = app.inner.lock().unwrap();
    let snapshot = match (&inner.demo, &app.engine) {
        (Some(demo), _) => demo.clone(),
        (None, Some(engine)) => engine.snapshot(),
        (None, None) => Snapshot::default(),
    };
    // The game was just seen somewhere other than usual: its Windows settings are read again.
    if let Some(exe) = snapshot.game.as_ref().and_then(|game| game.exe.clone()) {
        if inner.game_exe.as_deref() != Some(exe.as_str()) && exe.to_ascii_lowercase().ends_with(config::STEAM_CLIENT) {
            inner.graphics = winsettings::read(&exe);
            inner.game_exe = Some(exe);
        }
    }
    inner.watch_journal(&app.log.recent());
    view::build(&view::Input {
        snapshot: &snapshot,
        config: &inner.config,
        cores: &app.cores,
        start_tab: &app.start_tab,
        accent: &app.accent,
        notice: inner.notice.as_deref(),
        feedback: inner.feedback.as_ref().filter(|(_, at)| at.elapsed() < FEEDBACK_SHOWN).map(|(feedback, _)| feedback),
        graphics: inner.game_exe.as_ref().map(|_| &inner.graphics),
        on_top: inner.on_top,
        log_path: app.log.path().map(|path| path.display().to_string()),
    })
}

#[tauri::command]
fn journal(app: State<App>) -> Vec<String> {
    app.log.recent()
}

// --- In a match -------------------------------------------------------------------------------

#[tauri::command]
fn toggle(app: State<App>, feature: String) {
    let Some(feature) = Feature::ALL.into_iter().find(|candidate| candidate.key() == feature) else {
        return;
    };
    app.send(Command::Toggle(feature));
    // In demo mode there is no engine: the option flips in the fake state.
    if let Some(state) = app.inner.lock().unwrap().demo.as_mut().and_then(|demo| demo.features.iter_mut().find(|state| state.feature == feature)) {
        state.on = !state.on;
    }
}

#[tauri::command]
fn all_off(app: State<App>) {
    app.send(Command::AllOff);
    if let Some(demo) = app.inner.lock().unwrap().demo.as_mut() {
        demo.features.iter_mut().for_each(|state| state.on = false);
        demo.tweaks.iter_mut().for_each(|(tweak, value)| *value = Some(tweak.default()));
    }
}

#[tauri::command]
fn save_position(app: State<App>) {
    app.send(Command::SavePosition);
}

#[tauri::command]
fn teleport(app: State<App>) {
    app.send(Command::Teleport);
}

#[tauri::command]
fn set_round(app: State<App>, round: i32) {
    app.send(Command::SetRound(round.clamp(1, 999)));
}

#[tauri::command]
fn set_tweak(app: State<App>, key: String, value: f32) {
    let Some(tweak) = Tweak::ALL.into_iter().find(|candidate| candidate.key() == key) else {
        return;
    };
    let (min, max, _) = tweak.range();
    if !value.is_finite() {
        return;
    }
    let value = value.clamp(min, max);
    app.send(Command::SetTweak { tweak, value });
    // In demo mode there is no engine: the setting changes in the fake state.
    if let Some(state) = app.inner.lock().unwrap().demo.as_mut().and_then(|demo| demo.tweaks.iter_mut().find(|(candidate, _)| *candidate == tweak)) {
        state.1 = Some(value);
    }
}

#[tauri::command]
fn run_action(app: State<App>, key: String) {
    if let Some(action) = OneShot::ALL.into_iter().find(|candidate| candidate.key() == key) {
        app.send(Command::RunAction(action));
        app.demo_line(trs(format!("{}: done (demo, nothing is written)", action.label()), format!("{} : fait (démonstration, rien n'est écrit)", action.label())));
    }
}

/// Full survey of everything readable, into the log folder. Read-only.
#[tauri::command]
fn survey(app: State<App>) {
    app.send(Command::Survey);
    app.demo_line(tr("Survey written: no file in demo mode", "Relevé écrit : aucun fichier en démonstration"));
}

// --- Settings ---------------------------------------------------------------------------------

#[tauri::command]
fn set_cpu_enabled(app: State<App>, on: bool) {
    app.set("cpu", "enabled", Setting::Bool(on), |config| config.cpu.enabled = on);
}

#[tauri::command]
fn set_smt(app: State<App>, on: bool) {
    app.set("cpu", "smt", Setting::Bool(on), |config| config.cpu.smt = on);
}

#[tauri::command]
fn set_cores(app: State<App>, cores: usize) {
    // Zero in the file means "all cores": the setting then follows a processor change.
    let max_cores = if cores >= app.cores.len() { 0 } else { cores.max(1) };
    app.set("cpu", "max_cores", Setting::Int(max_cores as i64), |config| config.cpu.max_cores = max_cores);
}

#[tauri::command]
fn set_priority(app: State<App>, priority: String) {
    if let Some(priority) = Priority::ALL.into_iter().find(|candidate| candidate.key() == priority) {
        app.set("cpu", "priority", Setting::Text(priority.key()), |config| config.cpu.priority = priority);
    }
}

#[tauri::command]
fn set_fps(app: State<App>, enabled: bool, limit: i32) {
    let limit = limit.clamp(30, 1000);
    app.set("fps", "enabled", Setting::Bool(enabled), |config| config.fps.enabled = enabled);
    app.set("fps", "limit", Setting::Int(limit as i64), |config| config.fps.limit = limit);
    app.send(Command::SetFps { enabled, limit });
}

/// Confirmation sound on actions and shortcuts, saved in the configuration; takes effect immediately.
#[tauri::command]
fn set_sound(app: State<App>, on: bool) {
    app.set("trainer", "sound", Setting::Bool(on), |config| config.trainer.sound = on);
    app.send(Command::SetSound(on));
    // Heard right away: the switch says what it will sound like.
    if on {
        iw_helper::trainer::beep(true);
    }
}

#[tauri::command]
fn apply_windows_settings(app: State<App>) {
    let mut inner = app.inner.lock().unwrap();
    let Some(exe) = inner.game_exe.clone() else {
        return;
    };
    match winsettings::apply_recommended(&exe) {
        Ok(()) => {
            inner.graphics = winsettings::read(&exe);
            app.log.line(tr("Windows graphics settings applied for the game (takes effect next launch).", "Réglages graphiques Windows appliqués pour le jeu (effet au prochain lancement)."));
        }
        Err(e) => inner.notice = Some(trs(format!("Windows settings not applied: {e}"), format!("Réglages Windows non appliqués : {e}"))),
    }
}

#[tauri::command]
fn set_on_top(app: State<App>, window: tauri::WebviewWindow, on: bool) {
    let mut inner = app.inner.lock().unwrap();
    match window.set_always_on_top(on) {
        Ok(()) => inner.on_top = on,
        Err(e) => inner.notice = Some(trs(format!("Always on top not applied: {e}"), format!("Premier plan non appliqué : {e}"))),
    }
}

/// Language of the interface and the log, saved in the configuration; takes effect immediately.
#[tauri::command]
fn set_language(app: State<App>, language: String) {
    if let Some(choice) = Lang::parse(&language) {
        lang::set(choice);
        app.set("interface", "language", Setting::Text(choice.key()), |config| config.interface.language = choice);
    }
}

// --- Lobby, account and log -------------------------------------------------------------------

#[tauri::command]
fn profile(app: State<App>, action: String) {
    let Some(action) = profile::Action::parse(&action) else {
        return;
    };
    app.send(Command::Profile(action));
    // In demo mode, the character choice shows in the fake state.
    if let (profile::Action::Character(code), Some(demo)) = (action, app.inner.lock().unwrap().demo.as_mut()) {
        demo.character = Some(code);
    }
}

#[tauri::command]
fn verify(app: State<App>) {
    app.send(Command::Verify);
}

#[tauri::command]
fn set_watch(app: State<App>, on: bool) {
    app.send(Command::SetWatch(on));
}

#[tauri::command]
fn open_logs(app: State<App>) {
    let _ = std::process::Command::new("explorer").arg(&app.logs).spawn();
}

fn main() {
    let mut config_path = None;
    let mut game_command: Vec<String> = Vec::new();
    let mut demo = false;
    let mut start_tab = TABS[0].to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => game_command = args.by_ref().collect(),
            "--config" => config_path = args.next().map(PathBuf::from),
            "--demo" => demo = true,
            // Page opened at startup: accueil, salon, partie, compte, reglages or journal.
            "--tab" => {
                let wanted = args.next().unwrap_or_default().to_lowercase();
                if TABS.contains(&wanted.as_str()) {
                    start_tab = wanted;
                }
            }
            _ => {}
        }
    }
    let (config, source, load_error) = match config::load(config_path.as_deref()) {
        Ok((config, source)) => (config, source, None),
        Err(e) => (Config::default(), None, Some(trs(format!("Invalid configuration, default settings used: {e}"), format!("Configuration invalide, réglages par défaut utilisés : {e}")))),
    };
    lang::set(config.interface.language);
    let config_path = source.unwrap_or_else(config::default_path);
    let logs = config_path.parent().map(Path::to_path_buf).unwrap_or_default().join("logs");
    let cores = cpu::physical_cores().unwrap_or_else(|_| vec![1]);
    let log = Logbook::open((!demo).then_some(logs.as_path()), false);
    if let Some(error) = &load_error {
        log.line(error);
    }
    let wrapped = !game_command.is_empty();
    // The engine starts, and starts the game if needed, before the window and its web view exist:
    // the game does not wait for the interface, and the web view does not inherit the game's affinity.
    let engine = (!demo).then(|| {
        let launch = if wrapped { Launch::Wrap(game_command) } else { Launch::Attach };
        Engine::start(config.clone(), cores.clone(), log.clone(), launch)
    });
    let game_exe = winsettings::default_game_exe().map(|path| path.to_string_lossy().into_owned());
    let app = App {
        engine,
        inner: Mutex::new(Inner {
            last_log_line: log.recent().last().cloned(),
            graphics: game_exe.as_deref().map(winsettings::read).unwrap_or_default(),
            game_exe,
            config,
            feedback: None,
            notice: load_error,
            on_top: false,
            demo: demo.then(view::demo_snapshot),
        }),
        log,
        config_path,
        logs,
        cores,
        start_tab,
        accent: view::accent(winsettings::accent_shades()),
        demo,
    };

    tauri::Builder::default()
        .manage(app)
        .setup(move |tauri_app| {
            if wrapped {
                // Started by Steam: as long as this program lives, Steam considers the game open.
                // So it closes by itself once the game is closed.
                let handle = tauri_app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(FINISHED_CHECK);
                    if handle.state::<App>().engine.as_ref().is_some_and(|engine| engine.snapshot().finished) {
                        handle.exit(0);
                        break;
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            view,
            journal,
            toggle,
            all_off,
            save_position,
            teleport,
            set_round,
            set_tweak,
            run_action,
            survey,
            set_cpu_enabled,
            set_smt,
            set_cores,
            set_priority,
            set_fps,
            set_sound,
            apply_windows_settings,
            set_on_top,
            set_language,
            profile,
            verify,
            set_watch,
            open_logs,
        ])
        .run(tauri::generate_context!())
        .expect("tool window");
}
