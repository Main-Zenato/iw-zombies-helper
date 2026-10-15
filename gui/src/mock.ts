// Fake engine to see the interface in a browser, without the game or the Tauri window.
// `?tab=reglages` picks the open page, `?etat=absent|menu|match` the game state, `?lang=fr` the language.
import type { Action, Group, TabId, Tweak, View } from "./api";
import { type Lang, setLang, t } from "./i18n";

const params = new URLSearchParams(location.search);
const game = (params.get("etat") ?? "match") as View["game"];
const playing = game === "match";
// On opening, everything is off, as in the real tool. `?actives=1` turns three options on in a
// match, so previews can show the on state too.
const lit = playing && params.get("actives") === "1";

/** The whole view in one language: the text is what the engine would send. */
function makeView(language: Lang): View {
  setLang(language);
  const groups: Group[] = [
    {
      title: t("Player", "Joueur"),
      options: [
        { key: "god_mode", label: t("God mode", "Invincibilité"), hint: t("You no longer take damage.", "Vous ne prenez plus de dégâts."), hotkey: "F1", on: lit, code: false, trial: false },
        {
          key: "no_target",
          label: t("No target", "Sans cible"),
          hint: t("Zombies ignore you, but still hit you if you bump into them.", "Les zombies vous ignorent, mais frappent encore si vous les touchez."),
          hotkey: "F6",
          on: false,
          code: false,
          trial: false,
        },
        { key: "noclip", label: "Noclip", hint: t("Walk through walls. Turn it off with your feet on the ground.", "Traversez les murs. À couper les pieds au sol."), hotkey: "F5", on: false, code: false, trial: false },
        {
          key: "jump_pack",
          label: "Jump pack",
          hint: t("Brings back double jump, wall running and ledge climbing.", "Rend le double saut, la course sur les murs et l'escalade des rebords."),
          hotkey: "F11",
          on: false,
          code: false,
          trial: false,
        },
      ],
    },
    {
      title: t("Weapons", "Armes"),
      options: [
        {
          key: "infinite_ammo",
          label: t("Infinite ammo", "Munitions infinies"),
          hint: t("Magazine and reserve stay full; the magazine shows 511.", "Chargeur et réserve pleins ; le chargeur affiche 511."),
          hotkey: "F2",
          on: false,
          code: false,
          trial: false,
        },
        { key: "one_hit_kill", label: t("One-hit kill", "Un coup = un mort"), hint: t("Any hit kills a zombie.", "Le moindre coup tue un zombie."), hotkey: "F3", on: false, code: false, trial: false },
        { key: "no_recoil", label: t("No recoil", "Sans recul"), hint: t("The weapon no longer climbs when you fire.", "L'arme ne monte plus quand vous tirez."), hotkey: "F8", on: lit, code: true, trial: false },
        { key: "rapid_fire", label: t("Rapid fire", "Tir rapide"), hint: t("Faster rate of fire.", "Cadence de tir accélérée."), hotkey: "F9", on: false, code: true, trial: false },
      ],
    },
    {
      title: t("Match", "Partie"),
      options: [
        { key: "infinite_money", label: t("Infinite money", "Argent infini"), hint: t("Money kept at 100,000.", "Argent maintenu à 100 000."), hotkey: "F4", on: lit, code: false, trial: false },
        { key: "slow_motion", label: t("Game speed", "Vitesse du jeu"), hint: t("Game speed multiplied by 0.5.", "Vitesse du jeu multipliée par 0,5."), hotkey: "F7", on: false, code: false, trial: false },
        { key: "low_gravity", label: t("Low gravity", "Faible gravité"), hint: t("Higher, slower jumps.", "Sauts plus hauts et plus lents."), hotkey: "F10", on: false, code: true, trial: false },
        {
          key: "pause_zombies",
          label: t("Pause zombies", "Pause des zombies"),
          hint: t("Current zombies vanish and no new ones spawn. They come back when you turn it off.", "Les zombies présents disparaissent et plus aucun n'apparaît. Ils reviennent en coupant."),
          hotkey: "",
          on: false,
          code: false,
          trial: false,
        },
      ],
    },
  ];

  // The same values and actions as the engine (src/trainer.rs: `Tweak` and `OneShot`), in the same order.
  const tweaks: Tweak[] = [
    {
      key: "timescale",
      label: t("Game speed", "Vitesse du jeu"),
      hint: t("Slows down or speeds up the whole match. Back to 1 when the match ends.", "Ralentit ou accélère toute la partie. Revient à 1 en fin de partie."),
      value: 1,
      min: 0.1,
      max: 3,
      step: 0.05,
      default: 1,
      unit: "×",
      code: false,
      trial: false,
    },
    {
      key: "move_speed",
      label: t("Movement speed", "Vitesse de déplacement"),
      hint: t("How fast the player runs. Back to 190 when the match ends.", "Vitesse de course du joueur. Revient à 190 en fin de partie."),
      value: 190,
      min: 50,
      max: 600,
      step: 10,
      default: 190,
      unit: "",
      code: false,
      trial: false,
    },
    {
      key: "jump_height",
      label: t("Jump height", "Hauteur de saut"),
      hint: t("How high a jump goes, set in the Zombies suit. Back to 39 when the match ends.", "Hauteur d'un saut, réglée dans la tenue Zombies. Revient à 39 en fin de partie."),
      value: 39,
      min: 10,
      max: 400,
      step: 1,
      default: 39,
      unit: "",
      code: false,
      trial: false,
    },
    {
      key: "cash_scalar",
      label: t("Money multiplier", "Multiplicateur d'argent"),
      hint: t(
        "Money earned per zombie killed, like Double Points but at the rate you pick. Lasts for the current match.",
        "Argent gagné par zombie tué, comme les points doubles mais au choix. Vaut pour la partie en cours.",
      ),
      value: 1,
      min: 1,
      max: 10,
      step: 1,
      default: 1,
      unit: "×",
      code: false,
      trial: true,
    },
  ];

  const actions: Action[] = [
    { key: "refill_ammo", label: t("Refill ammo", "Remplir les munitions"), hint: t("Magazines to 511 and reserves to 9999, once.", "Chargeurs à 511 et réserves à 9999, une seule fois."), button: t("Refill", "Remplir"), trial: false },
    {
      key: "add_money",
      label: t("Add $10,000", "Ajouter 10 000 $"),
      hint: t("Once. The in-game counter catches up on your next gain or purchase.", "Une seule fois. L'affichage du jeu suit au prochain gain ou achat."),
      button: t("Add", "Ajouter"),
      trial: false,
    },
    {
      key: "weaken_zombies",
      label: t("Weaken current zombies", "Affaiblir les zombies présents"),
      hint: t("Health set to 1 for zombies already spawned. For later ones too, use One-hit kill.", "Vie à 1 pour ceux déjà apparus. Pour les suivants aussi : « Un coup = un mort »."),
      button: t("Weaken", "Affaiblir"),
      trial: false,
    },
    {
      key: "end_round",
      label: t("End the round", "Finir la manche"),
      hint: t("The round counts as won and the next one starts. Zombies still alive stay.", "La manche compte comme gagnée et la suivante commence. Les zombies encore là restent."),
      button: t("End", "Finir"),
      trial: false,
    },
    {
      key: "charge_cards",
      label: t("Recharge Fate & Fortune cards", "Recharger les cartes Fate & Fortune"),
      hint: t("The card meter fills up on your next money gain.", "La jauge des cartes se remplit au prochain gain d'argent."),
      button: t("Recharge", "Recharger"),
      trial: false,
    },
    {
      key: "add_tickets",
      label: t("Add 1,000 tickets", "Ajouter 1 000 tickets"),
      hint: t("Zombies in Spaceland arcade tickets. The counter catches up on your next ticket won.", "Tickets d'arcade de Zombies in Spaceland. Le compteur suit au prochain ticket gagné."),
      button: t("Add", "Ajouter"),
      trial: false,
    },
  ];

  const keys = { allOff: t("End", "Fin"), savePosition: t("Page Up", "Page préc."), teleport: t("Page Down", "Page suiv.") };
  return {
    startTab: (params.get("tab") ?? "accueil") as TabId,
    language,
    accent: "#60cdff",
    game,
    state: { absent: t("Game not running", "Jeu non lancé"), menu: t("Game running, not in a match", "Jeu lancé, hors partie"), match: t("In a match", "En partie") }[game],
    message: {
      absent: { text: t("Start Infinite Warfare from Steam: this window will follow it.", "Lancez Infinite Warfare depuis Steam : cette fenêtre le suivra."), tone: "guide" as const },
      menu: {
        text: t("Options unlock once the round has started. The game runs on 12 threads.", "Les options se débloquent une fois la manche commencée. Le jeu tourne sur 12 threads."),
        tone: "guide" as const,
      },
      match: { text: t("Click an option, or use its key in game. The game runs on 12 threads.", "Cliquez une option, ou utilisez sa touche en jeu. Le jeu tourne sur 12 threads."), tone: "guide" as const },
    }[game],
    ready: game !== "absent",
    profileReady: game !== "absent",
    groups,
    tweaks,
    actions,
    keys,
    savedPosition: playing ? [1204, -388, 64] : null,
    character: game === "absent" ? null : 0,
    characters: [
      { code: 0, name: t("Normal draw", "Tirage normal"), map: t("All maps", "Toutes les cartes") },
      { code: 1, name: "David Hasselhoff", map: "Zombies in Spaceland" },
      { code: 5, name: "Willard Wyler", map: "Zombies in Spaceland" },
      { code: 2, name: "Kevin Smith", map: "Rave in the Redwoods" },
      { code: 3, name: "Pam Grier", map: "Shaolin Shuffle" },
      { code: 4, name: "Elvira", map: "Attack of the Radioactive Thing" },
    ],
    hotkeys: [
      ...groups.flatMap((group) => group.options).map((option) => ({ label: option.label, key: option.hotkey })),
      // One-off actions have no key until the configuration gives them one.
      ...actions.map((action) => ({ label: action.label, key: "" })),
      { label: t("Save position", "Enregistrer la position"), key: keys.savePosition },
      { label: t("Back to saved position", "Revenir à la position"), key: keys.teleport },
      { label: t("Turn everything off", "Tout désactiver"), key: keys.allOff },
    ],
    watching: false,
    settings: {
      cpu: {
        enabled: true,
        smt: false,
        cores: 12,
        coreCount: 12,
        priority: "high",
        priorities: [
          { key: "normal", label: t("Normal", "Normale") },
          { key: "above_normal", label: t("Above", "Supérieure") },
          { key: "high", label: t("High", "Haute") },
        ],
        outcome: t("The game will start with 12 of 24 threads.", "Le jeu démarrera avec 12 threads sur 24."),
        threads: 12,
        totalThreads: 24,
      },
      fps: { enabled: true, limit: 240, effective: 250 },
      windows: { dedicatedGpu: true, windowedOptimizationsOff: false },
      onTop: false,
    },
    logPath: "C:\\iw-helper\\logs\\session-20261013-190214.log",
  };
}

let view = makeView(params.get("lang") === "fr" ? "fr" : "en");

const journal = [
  t("19:02:14  iw7_ship.exe started (pid 4242), affinity set from the start.", "19:02:14  iw7_ship.exe lancé (pid 4242), affinité en place dès le démarrage."),
  t("19:02:14  iw7_ship.exe found (pid 4242).", "19:02:14  iw7_ship.exe détecté (pid 4242)."),
  t("19:02:14  High priority applied.", "19:02:14  Priorité Haute appliquée."),
  t("19:02:31  Game version recognised.", "19:02:31  Version du jeu reconnue."),
  t("19:02:31  FPS limit set (91 -> 240).", "19:02:31  Limite de FPS réglée (91 -> 240)."),
  t("19:04:02  Match detected.", "19:04:02  Partie détectée."),
  t("19:04:40  God mode: ON", "19:04:40  Invincibilité : ON"),
  t("19:04:51  Infinite money: ON", "19:04:51  Argent infini : ON"),
  t("19:05:20  No recoil: ON", "19:05:20  Sans recul : ON"),
  t(
    "19:05:44  Round change impossible (round variable not found in this match).",
    "19:05:44  Changement de manche impossible (variable de la manche introuvable dans cette partie).",
  ),
];

function setMessage(text: string, tone: View["message"]["tone"]) {
  view.message = { text, tone };
  journal.push(`19:06:00  ${text}`);
}

export function respond(command: string, args: Record<string, unknown>): unknown {
  const options = view.groups.flatMap((group) => group.options);
  switch (command) {
    case "view":
      return structuredClone(view);
    case "journal":
      return [...journal];
    case "toggle": {
      const option = options.find((candidate) => candidate.key === args.feature);
      if (option) {
        option.on = !option.on;
        setMessage(t(`${option.label}: ${option.on ? "on" : "off"}`, `${option.label} : ${option.on ? "activé" : "coupé"}`), "done");
      }
      return;
    }
    case "all_off":
      options.forEach((option) => (option.on = false));
      view.tweaks.forEach((tweak) => (tweak.value = tweak.default));
      return setMessage(t("Everything is off.", "Tout est désactivé."), "done");
    case "set_tweak": {
      // Like the engine: the value is brought within bounds, and the message gives the old and new ones.
      const tweak = view.tweaks.find((candidate) => candidate.key === args.key);
      if (tweak) {
        const previous = tweak.value;
        tweak.value = Math.min(tweak.max, Math.max(tweak.min, Number(args.value)));
        setMessage(t(`${tweak.label}: ${previous} -> ${tweak.value}`, `${tweak.label} : ${previous} -> ${tweak.value}`), "done");
      }
      return;
    }
    case "run_action": {
      const action = view.actions.find((candidate) => candidate.key === args.key);
      return action ? setMessage(t(`${action.label}: done`, `${action.label} : fait`), "done") : undefined;
    }
    case "survey":
      return setMessage(t("Survey written: releve-20261014-190600.txt", "Relevé écrit : releve-20261014-190600.txt"), "done");
    case "set_round":
      return setMessage(
        t("Round change impossible (round variable not found in this match).", "Changement de manche impossible (variable de la manche introuvable dans cette partie)."),
        "refused",
      );
    case "save_position":
      view.savedPosition = [1204, -388, 64];
      return setMessage(t("Position saved: [1204.0, -388.0, 64.0]", "Position enregistrée : [1204.0, -388.0, 64.0]"), "done");
    case "teleport":
      return setMessage(t("Teleported to [1204.0, -388.0, 64.0]", "Téléporté en [1204.0, -388.0, 64.0]"), "done");
    case "profile": {
      const code = /^character-(\d)$/.exec(String(args.action))?.[1];
      if (code !== undefined) {
        view.character = Number(code);
        return setMessage(t("Character: choice saved for the next match", "Personnage : choix enregistré pour la prochaine partie"), "done");
      }
      return setMessage(t(`Profile: ${String(args.action)}`, `Profil : ${String(args.action)}`), "done");
    }
    case "set_language":
      // The whole view is rebuilt in the other language, as the engine would send it.
      view = makeView(args.language === "fr" ? "fr" : "en");
      return;
    case "set_watch":
      view.watching = Boolean(args.on);
      return;
    case "set_cpu_enabled":
      view.settings.cpu.enabled = Boolean(args.on);
      return;
    case "set_smt":
      view.settings.cpu.smt = Boolean(args.on);
      return;
    case "set_cores":
      view.settings.cpu.cores = Number(args.cores);
      return;
    case "set_priority":
      view.settings.cpu.priority = String(args.priority);
      return;
    case "set_fps": {
      const limit = Number(args.limit);
      const effective = Math.floor(1000 / Math.max(1, Math.floor(1000 / limit)));
      view.settings.fps = { enabled: Boolean(args.enabled), limit, effective };
      return;
    }
    case "apply_windows_settings":
      view.settings.windows = { dedicatedGpu: true, windowedOptimizationsOff: true };
      return;
    case "set_on_top":
      view.settings.onTop = Boolean(args.on);
      return;
    default:
      return;
  }
}
