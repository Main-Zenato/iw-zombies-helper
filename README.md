# iw-helper

An external companion tool for *Call of Duty: Infinite Warfare* on PC (Steam build 2715288): a CPU
stutter fix, an FPS limit unlock, and a Zombies trainer, in one small Windows app.

| Feature | What it does |
| --- | --- |
| CPU fix | Sets the game's CPU affinity and priority from the moment it starts. |
| FPS limit | Replaces the engine's internal limit (91 FPS in Zombies) with the value you pick. |
| Zombies trainer | God mode, no target, noclip, jump pack, infinite ammo and money, one-hit kill, pause zombies, game and movement speed, jump height, round change, teleport, and more. Solo and private matches, as the host. |
| Lobby | Special character for the next match, contracts (bounties). |
| Account | Read the profile, level 999, weapon XP, Director's Cut, account reset. |

It is a separate program: nothing is injected into the game and no game file is modified. The
interface is available in **English** and **French** (Settings page).

> [!WARNING]
> **Read [Game memory access](#game-memory-access-read-this-first) before using the trainer.** The Steam
> client is protected by VAC, and this tool makes no attempt to hide from it. Use it privately: solo,
> private matches as the host, or iw7-mod.

## Quick start

1. Download `iw-helper` from the [Releases](../../releases) page (the `steam` or `iw7-mod` zip, see
   [Installation](#1-get-it)) and unzip it anywhere, or [build it](#installation).
2. In Steam, set the game's launch options to `"C:\path\to\iw-helper-gui.exe" -- %command%`.
3. Start the game from Steam: the tool's window opens with it.
4. In a Zombies match, click an option or press its key (F1 to F11). Press **End** to turn everything
   off before leaving.

## Contents

- [Quick start](#quick-start)
- [Game memory access: read this first](#game-memory-access-read-this-first)
- [Requirements](#requirements)
- [Installation](#installation)
- [Using it](#using-it)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Configuration](#configuration)
- [Command line](#command-line)
- [Logs and full survey](#logs-and-full-survey)
- [iw7-mod extras](#iw7-mod-extras)
- [Known limits](#known-limits)
- [Troubleshooting](#troubleshooting)
- [Uninstall](#uninstall)
- [Development](#development)
- [Credits and license](#credits-and-license)

## Game memory access: read this first

The CPU fix does not touch the game's memory: it sets affinity and priority, like Task Manager does.
Everything else **reads or writes the game's memory**.

The Steam client (`iw7_ship.exe`) is protected by VAC, and `iw-helper` does nothing to hide from the
anti-cheat. A ban is therefore possible, even in solo, and a VAC ban is permanent. From what we could
find, players describe the risk as low for solo and private Zombies use on this game. That is not a
guarantee: the risk is not zero, and you use the tool at your own risk.

**Use it privately.** Recommended use, from safest to riskiest:

1. On [iw7-mod](https://github.com/auroramod/iw7-mod), the community client, which has no VAC.
2. On Steam, alone or in a private match where you are the host, with friends who know.
3. Never in public matchmaking, and never in multiplayer: other players did not agree to it, and that is
   where reports and bans happen.

This is decided by `allow_steam_client`, in the `[memory]` section of
[iw-helper.toml](iw-helper.toml#L37-L38):

- `false`: these features only work on [iw7-mod](https://github.com/auroramod/iw7-mod); on the Steam
  client, only the CPU fix applies;
- `true`: they also work on the Steam client.

The `iw7-mod` release zip ships with `false`, the `steam` zip with `true`. The file in this repository has
`true`. You can change it at any time in `iw-helper.toml`, next to `iw-helper-gui.exe`.

Safeguards:

- nothing is written unless the running game's code matches the six known instructions of this version;
- every write is preceded by a check of what is at the address;
- trainer options only respond during a match, and you must be the host.

The Lobby and Account pages write into your profile. Account actions are **permanent**: the game saves the
profile and sends it online, and the servers can reset or sanction an account independently of VAC.

## Requirements

- Windows 10 or 11, 64-bit.
- *Call of Duty: Infinite Warfare* on Steam, build 2715288 (the current one), or iw7-mod.
- To build from source: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20.19 or later and
  [pnpm](https://pnpm.io).
- The interface uses WebView2, which ships with Windows 11 (and recent Windows 10 updates).

## Installation

### 1. Get it

**Ready-made:** download the latest release from the [Releases](../../releases) page and unzip it into a
folder of your choice (not inside the game folder). There are two zips, which differ only by
`allow_steam_client` in `iw-helper.toml` (see [Game memory access](#game-memory-access-read-this-first)):

| Zip | For | `allow_steam_client` |
| --- | --- | --- |
| `iw-helper-vX.Y.Z-steam-windows-x64.zip` | the Steam game (`iw7_ship.exe`) | `true` |
| `iw-helper-vX.Y.Z-iw7-mod-windows-x64.zip` | [iw7-mod](https://github.com/auroramod/iw7-mod) | `false` |

Each one holds `iw-helper-gui.exe`, `iw-helper.exe`, `iw-helper.toml`, this README, the license and the
uninstaller ([Uninstall](#uninstall)). Windows SmartScreen may warn about an
unsigned program the first time: "More info" > "Run anyway".

**From source:** with the game closed, from the project folder:

```powershell
.\build.ps1
```

The script builds the interface pages, then both programs into `target\release\`:

| Program | Role |
| --- | --- |
| `iw-helper-gui.exe` | The graphical interface. This is the one Steam starts. |
| `iw-helper.exe` | The same thing from the command line, without a window. |

The pages are built into `iw-helper-gui.exe`: there is nothing else to copy, and nothing is downloaded
while the tool runs. Keep `iw-helper.toml` next to the program, or in one of its two parent folders (from
`target\release\`, the one at the root of the project is used).

### 2. Launch it with the game

In Steam: right-click the game > Properties > General > Launch options, and paste, with the path to
`iw-helper-gui.exe` (the unzipped folder, or `target\release\` when built from source):

```
"C:\path\to\iw-helper\iw-helper-gui.exe" -- %command%
```

Then start the game from Steam as usual. The tool's window opens with the game and closes with it.

Why through Steam: the engine measures the CPU at startup and again during play. If the affinity changes
in between, the two measures differ and the game asks "Apply optimal settings?" on every launch. Started
by the tool, the game inherits the affinity from its very first moment. Opening `iw-helper-gui.exe` by hand
next to an already running game works too, but the affinity is then applied afterwards.

### 3. Pick your language

The interface starts in English. Settings > Language switches it, and the log, to French; the choice is
saved in `iw-helper.toml`.

## Using it

The window follows the look of Windows 11 Settings in dark mode, with your Windows accent color.

| Page | Contents |
| --- | --- |
| Home | The board of options with their keys (a lit key means the option is on; a click toggles it), then what is set before the match: processor, FPS limit, Windows settings, character. |
| Lobby | What is set before starting the match: character, contracts. |
| In match | The Zombies options in three groups (Player, Weapons, Match), each with its key and switch; then values (game speed, movement speed, jump height, money multiplier) and actions (ammo, money, zombies, round, cards, tickets, position). |
| Account | Read and change the account progress. |
| Settings | Language, processor, FPS limit, Windows graphics settings, always on top, list of shortcuts. |
| Log | Everything the tool did, the address check, the full survey, player watching. |

- The top of the side pane shows the game state (not running, in the menus, in a match) and how many
  options are on.
- **Turn everything off** sits at the top right of every page. Use it before leaving a match.
- A bar says what to do next, then the result of your last action; it turns red when an action is refused.
- A yellow triangle marks options that change the game's code.
- Outside a match you can turn off an option left on, but not turn new ones on.

**Keep on top** (Settings) keeps the window above the game when it runs in borderless window. In exclusive
fullscreen, use Alt+Tab or the keyboard shortcuts.

### Special characters (Lobby page)

| Character | Map |
| --- | --- |
| David Hasselhoff | Zombies in Spaceland |
| Willard Wyler | Zombies in Spaceland |
| Kevin Smith | Rave in the Redwoods |
| Pam Grier | Shaolin Shuffle |
| Elvira | Attack of the Radioactive Thing |

Choose in the lobby, before starting the match. The choice lasts for one match, on that character's map
only; the game then goes back to the normal draw.

## Keyboard shortcuts

Active while the game window is in front. A high beep means "on", a low beep "off" or "refused". None of
them is on the numeric keypad, and everything is also available from the interface.

| Key | Action | How |
| --- | --- | --- |
| F1 | God mode | The engine's own `god` flag |
| F2 | Infinite ammo | Value freeze: magazine and reserve rewritten continuously; the magazine shows 511 |
| F3 | One-hit kill | Value freeze: the health of all 24 zombie slots set to 1 |
| F4 | Infinite money | Value freeze |
| F5 | Noclip | The engine's own `noclip` flag |
| F6 | No target | The player's `ignoreme` field, the one the zombie scripts check; set again if cleared |
| F7 | Game speed | Data |
| F8 | No recoil | Code patch |
| F9 | Rapid fire | Code patch |
| F10 | Low gravity | Code patch |
| F11 | Jump pack | Lifts the double jump, wall run and ledge climbing bans the Zombies scripts set (three bits of the player state) |
| Page Up | Save position | Read |
| Page Down | Back to saved position | Data |
| End | Turn everything off | |

The engine flags are the ones the engine's own cheat commands toggle: nothing to rewrite, no code changed.
A value freeze rewrites data every 100 ms, without touching code. Only three options (no recoil, rapid fire,
low gravity) change instructions, which the game's protection watches: they are the most likely to crash
it, hence **Turn everything off** before leaving a match.

These have no key by default and can get one in `[hotkeys]`:

| Value or action | How |
| --- | --- |
| Game speed, 0.1 to 3 | Data; back to 1 at the end of the match |
| Movement speed, 50 to 600 | Engine setting, found by the hash of its name; back to 190 at the end of the match and by Turn everything off |
| Jump height, 10 to 400 | Zombies suit setting; back to 39 at the end of the match and by Turn everything off |
| Money multiplier, 1 to 10 (trial) | The `level.cash_scalar` script field, the one Double Points uses; set again when Double Points ends |
| Pause zombies (`pause_zombies`) | `level.max_static_spawned_enemies` set to 0: the game's own spawn loop removes the extra zombies, then waits; the game's value comes back when turned off |
| Refill ammo (`refill_ammo`) | One write to magazines and reserves, no freeze |
| Add $10,000 (`add_money`) | One write, from the current amount; the in-game counter catches up on your next gain or purchase |
| Weaken current zombies (`weaken_zombies`) | One write: health of the zombies already spawned set to 1 |
| End the round (`end_round`) | `level.current_enemy_deaths` raised to the round's target |
| Recharge Fate & Fortune cards (`charge_cards`) | The player's `consumable_meter` set to its maximum; the game sees it full on your next money gain |
| Add 1,000 tickets (`add_tickets`) | The player's `num_tickets`, Zombies in Spaceland; the counter catches up on your next ticket won |

The [iw7-mod extras](#iw7-mod-extras) can get keys the same way.

Script fields are found by the public number of their name, like the round. The player's script object is
the one that points to entity 0 and holds `maxcurrency`.

When the tool opens, everything is off. If it was closed while options were still on in a game that stayed
open, it restores them once when it reattaches, and the log says which. It only restores what it left in
that very game: it keeps track in `logs\etat-en-cours.txt`.

## Configuration

Everything is in [iw-helper.toml](iw-helper.toml), commented line by line. The interface saves the settings
it offers by itself; the rest is changed by hand, then takes effect when the game restarts.

| Section | Contents |
| --- | --- |
| `[interface]` | `language`: `en` or `fr`. |
| `[cpu]` | CPU fix: one thread per physical core or not (`smt`), number of cores, priority. |
| `[memory]` | `allow_steam_client`: allows memory features on the Steam client. |
| `[fps]` | FPS limit. The engine counts a frame in whole milliseconds: 240 requested gives 250 in practice, 144 gives 166. |
| `[trainer]` | Zombies options: target game speed, infinite money amount, beeps. |
| `[hotkeys]` | One key per action; an empty string removes the shortcut. |

An unknown key makes the tool refuse the file with a message: the interface then runs with default settings, the command line stops.

## Command line

```
iw-helper -- <game command>      starts the game, like the interface, without a window
iw-helper run [--launch]         waits for a running game (--launch asks Steam to start it)
iw-helper info                   what would be applied, without touching anything
iw-helper verify                 read-only address check
iw-helper survey                 full read-only survey into logs\
iw-helper profile <action>       reads or changes the Zombies profile
```

`profile` actions: `show`, `max-level`, `weapon-xp`, `directors-cut`, `contracts`, `reroll-contracts`,
`character-0` to `character-5`, `reset` (requires `--confirm`). Every command accepts `--config <file>`.

## Logs and full survey

Each session writes a file into `logs\`, next to the configuration file: settings applied, actions and their
result. The Log page shows it and opens the folder.

The **Full survey** button on the Log page (or `iw-helper survey`) writes a `releve-YYYYMMDD-HHMMSS.txt` file
into `logs\` with everything the tool can read, in the menus or in a match: profile, engine settings, loaded
suits, weapons and attachments, script fields, player and zombies. The game is opened read-only for it:
nothing is changed. From the interface, a second survey taken in the same state starts with what changed
since the previous one. Survey files are written in French and contain account data; keep them to yourself.

## iw7-mod extras

**iw7-mod only.** [iw7-mod](https://github.com/auroramod/iw7-mod) loads game scripts (GSC) from its
`custom_scripts` folder; the Steam game does not. [iw_helper.gsc](iw7-mod/custom_scripts/cp/iw_helper.gsc)
uses this to call the game's own Zombies functions, which the tool cannot reach from memory.

**Install:** copy the [iw7-mod](iw7-mod/) folder of this repository (also in the `iw7-mod` release zip) into
the game folder, so that the file ends up in `<game folder>\iw7-mod\custom_scripts\cp\iw_helper.gsc`. It is
compiled when a Zombies match loads. To remove it, delete that file.

**Use**, as the host, in either of two ways:

- **From the window**: page **Match**, section **iw7-mod extras**, shown only when the game is
  `iw7-mod.exe`. The tool writes the command's number into a script field, `level.command_given`, which the
  script reads ten times a second; nothing is written there on the Steam game. Each one can get a key in
  `[hotkeys]` (names in the table).
- **From the iw7-mod console**: `set iwh <command>`, or bind a key once, for example `bind F5 "set iwh pap"`.
  This works without the tool.

| Window (`[hotkeys]` name) | Console | Effect |
| --- | --- | --- |
| Pack-a-Punch (`upgrade_weapons`) | `pap` | Every player's weapons go up one Pack-a-Punch level; again for the next level. |
| Kill every zombie (`kill_all`) | `killall` | Every zombie alive, at once (bosses may resist). |
| Open doors and turn on power (`open_doors`), trial | `doors` | What the game's direct boss fight mode runs: the map's own way to turn on the power and open its doors. |
| All perks (`all_perks`), trial | `perks` | Every perk the map has, given the way a perk machine does. |
| Mystery weapon (`box_weapon`), trial | `weapon` | A random weapon from the map's list, in place of the one in hand if you carry the maximum. |
| Max ammo, nuke, instakill, double money, fire sale, infinite ammo bonuses (`drop_max_ammo`, `drop_nuke`, `drop_instakill`, `drop_double_money`, `drop_fire_sale`, `drop_infinite_ammo`) | `drop ammo`, `drop kill`, `drop instakill`, `drop cash`, `drop fire`, `drop infinite` | The bonus appears in front of you: walk into it. Not every map has every bonus. |
| | `bonuses` | Lists the bonuses of the current map. |

Seen working in game, from the console: Pack-a-Punch, kill every zombie and bonuses. Not tried yet: doors,
perks, mystery weapon, and anything sent from the window. On iw7-mod, the **Account** page is turned off: its
actions are for the Steam game's account.

## Known limits

- **One game version**: Steam build 2715288. On any other, nothing is written.
- **Seen working in game**: game speed, movement speed, jump height, ammo, money, zombies, character, round
  change (several maps), no target, pause zombies, end of round, card recharge, tickets, jump pack.
- **Trial**: the money multiplier is written from the mode's scripts but has not been seen working yet.
- **No target**: zombies no longer pick you as a target, but one you bump into still hits.
- **Jump pack**: double jump, wall running and ledge climbing work; the jump energy bar does not show, as it
  belongs to the multiplayer HUD that Zombies does not have.
- **Out of reach**: a map's easter egg (its steps wait for game events that no written value triggers),
  quest parts (objects to pick up), Pack-a-Punch on the held weapon (the held weapon is decided on the client
  side, at a place not found), and characters from another map (each map only loads its own). On iw7-mod,
  Pack-a-Punch and bonuses are possible through a script: see [iw7-mod extras](#iw7-mod-extras).
- **Restoring leftovers on startup**: not yet tried in game.
- **CPU settings**: removing sibling threads is the most often reported stutter fix, but its effect depends
  on the machine. Compare with and without on yours.

## Troubleshooting

| Symptom | What to do |
| --- | --- |
| "Game version not recognised" | Only build 2715288 is supported. Make sure the game is up to date and not modified by another tool. |
| Options stay greyed out | They unlock once a Zombies round has started, and you must be the host. |
| "Apply optimal settings?" at every launch | Start the game through the Steam launch option above, not by opening the tool next to it. |
| The trainer does nothing on Steam | Check `allow_steam_client` and `trainer.enabled` in `iw-helper.toml`. |
| The game crashed after leaving a match | Use Turn everything off before leaving: code patches are the usual cause. |
| Shortcuts do nothing | They only work while the game window is in front. |

## Uninstall

Close the game and the tool, then double-click [uninstall.cmd](uninstall.cmd) in the tool's folder. It lists what
it will delete and asks before doing anything:

- the tool's files in its folder (programs, `iw-helper.toml`, README, license, `logs\`), then the folder if
  nothing else is left in it;
- the interface's data in `%LOCALAPPDATA%\fr.zen.iw-helper`;
- if you say yes to a second question, the two Windows graphics settings the tool can apply to the game
  (Settings page). Other options Windows keeps for the game are left as they are.

Then, in Steam, clear the game's launch options, otherwise the game will not start. Changes made to your
profile or account from the Lobby and Account pages stay: they are part of the game save.

From a source checkout, the script only deletes `logs\` and the interface's data; the code is kept.

## Development

| Folder | Contents |
| --- | --- |
| [src/](src/) | The core, in Rust: process, affinity, game tracking engine, Zombies options, profile, log, languages. And the command line (`main.rs`). |
| [gui/src-tauri/](gui/src-tauri/) | The [Tauri](https://tauri.app) window: it links the pages to the core and prepares what they show (`view.rs`). |
| [gui/src/](gui/src/) | The pages, in TypeScript with React, shown by WebView2. |

```
cargo test --workspace                    # unit tests, without the game
cargo clippy --workspace --all-targets
```

To work on the interface without the game:

- `target\release\iw-helper-gui.exe --demo` opens the window with a fake state, saving nothing;
  `--tab salon` (or `partie`, `compte`, `reglages`, `journal`) opens it on a given page.
- From [gui/](gui/), `pnpm dev` serves the pages in a browser with fake data (`?lang=fr` for French).
- From [gui/](gui/), `pnpm apercu <folder> [absent|menu|match]` writes one static HTML page per tab.

User-facing text is bilingual: `lang::tr("English", "Français")` in Rust and `t("English", "Français")` in
the pages, English being the default. Full survey files are written in French.

## Credits and license

- Engine structures and addresses: [iw7-mod](https://github.com/auroramod/iw7-mod) (auroramod, GPL-3.0).
  The integer dvar hash (`hash_integer`) is a port of its code.
- FPS limit location: [IW FPS Unlocker](https://gitlab.com/TacoCake/iw-fps-unlocker) (TacoCake,
  AGPL-3.0). Only the location and layout of the setting were used, no code.
- Script field names: the public name list of [gsc-tool](https://github.com/xensik/gsc-tool) (xensik,
  GPL-3.0).
- Interface icons: Fluent UI System Icons (Microsoft, MIT).

This project is licensed under the [GPL-3.0](LICENSE). No game file is included in this repository: only the
addresses and the few instruction bytes needed to recognise the game version and act on it.

*iw-helper is an unofficial fan project. It is not affiliated with or endorsed by Activision, Infinity
Ward or Valve. Use it at your own risk.*
