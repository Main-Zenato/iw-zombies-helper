// What the interface exchanges with the engine (gui/src-tauri/src/view.rs describes the same data on the Rust side).
// Outside the Tauri window (preview in a browser), a fake engine answers instead of the real one.
import { invoke } from "@tauri-apps/api/core";
import type { Lang } from "./i18n";

export type TabId = "accueil" | "salon" | "partie" | "compte" | "reglages" | "journal";
export type GameState = "absent" | "menu" | "match";
export type Tone = "guide" | "done" | "refused" | "error";

export interface Option {
  key: string;
  label: string;
  hint: string;
  /** Keyboard key bound to it, already written as on the user's keyboard. Empty if there is none. */
  hotkey: string;
  on: boolean;
  /** The option changes the game's code. */
  code: boolean;
  /** Never seen working in game: marked "Trial". */
  trial: boolean;
}

/** A numeric setting that acts during the match (movement speed, jump height…). */
export interface Tweak {
  key: string;
  label: string;
  hint: string;
  /** Current value: read from the game during a match, otherwise the original value. */
  value: number;
  min: number;
  max: number;
  step: number;
  /** The game's original value, for "Reset". */
  default: number;
  /** Unit shown after the value; empty if there is none. */
  unit: string;
  code: boolean;
  trial: boolean;
}

/** A one-off action during the match (refill ammo, weaken current zombies…). */
export interface Action {
  key: string;
  label: string;
  hint: string;
  /** Button text. */
  button: string;
  trial: boolean;
}

export interface Group {
  title: string;
  options: Option[];
}

export interface Character {
  code: number;
  name: string;
  /** Map on which the game accepts this character. */
  map: string;
}

export interface Settings {
  cpu: {
    enabled: boolean;
    smt: boolean;
    cores: number;
    coreCount: number;
    priority: string;
    priorities: { key: string; label: string }[];
    /** What the game will get at its next launch, in one sentence. */
    outcome: string;
    /** Threads the game will get, out of the machine's. */
    threads: number;
    totalThreads: number;
  };
  fps: { enabled: boolean; limit: number; effective: number };
  /** Missing when the game was not found on disk. */
  windows: { dedicatedGpu: boolean; windowedOptimizationsOff: boolean } | null;
  onTop: boolean;
}

export interface View {
  startTab: TabId;
  /** Language of the text, the one from the configuration. */
  language: Lang;
  /** Accent color chosen in Windows, "#rrggbb". */
  accent: string;
  game: GameState;
  state: string;
  message: { text: string; tone: Tone };
  /** The trainer options respond. */
  ready: boolean;
  /** Profile actions (lobby, account) respond. */
  profileReady: boolean;
  groups: Group[];
  /** Numeric settings and one-off actions; empty lists if the engine offers none. */
  tweaks: Tweak[];
  actions: Action[];
  keys: { allOff: string; savePosition: string; teleport: string };
  savedPosition: [number, number, number] | null;
  /** Code of the character requested for the next match, when the profile is readable. */
  character: number | null;
  characters: Character[];
  /** Every keyboard shortcut; `key` is empty when the action has none. */
  hotkeys: { label: string; key: string }[];
  watching: boolean;
  settings: Settings;
  logPath: string | null;
}

export type ProfileAction = "show" | "max-level" | "weapon-xp" | "directors-cut" | "contracts" | "reroll-contracts" | `character-${number}` | "reset";

type Args = Record<string, unknown>;

const inTauri = "__TAURI_INTERNALS__" in window;
let fake: Promise<(command: string, args: Args) => unknown> | undefined;

async function call<T>(command: string, args: Args = {}): Promise<T> {
  if (inTauri) {
    return invoke<T>(command, args);
  }
  fake ??= import("./mock").then((module) => module.respond);
  return (await fake)(command, args) as T;
}

export const api = {
  view: () => call<View>("view"),
  journal: () => call<string[]>("journal"),
  toggle: (feature: string) => call<void>("toggle", { feature }),
  allOff: () => call<void>("all_off"),
  savePosition: () => call<void>("save_position"),
  teleport: () => call<void>("teleport"),
  setRound: (round: number) => call<void>("set_round", { round }),
  setTweak: (key: string, value: number) => call<void>("set_tweak", { key, value }),
  runAction: (key: string) => call<void>("run_action", { key }),
  setWatch: (on: boolean) => call<void>("set_watch", { on }),
  verify: () => call<void>("verify"),
  /** Full survey of everything readable, written into the log folder. Read-only. */
  survey: () => call<void>("survey"),
  profile: (action: ProfileAction) => call<void>("profile", { action }),
  setCpuEnabled: (on: boolean) => call<void>("set_cpu_enabled", { on }),
  setSmt: (on: boolean) => call<void>("set_smt", { on }),
  setCores: (cores: number) => call<void>("set_cores", { cores }),
  setPriority: (priority: string) => call<void>("set_priority", { priority }),
  setFps: (enabled: boolean, limit: number) => call<void>("set_fps", { enabled, limit }),
  applyWindowsSettings: () => call<void>("apply_windows_settings"),
  setOnTop: (on: boolean) => call<void>("set_on_top", { on }),
  setLanguage: (language: Lang) => call<void>("set_language", { language }),
  openLogs: () => call<void>("open_logs"),
};
