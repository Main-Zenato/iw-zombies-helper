import { ChevronRight16Regular, Circle12Regular, Desktop20Regular, DeveloperBoard20Regular, PersonStar20Regular, TopSpeed20Regular } from "@fluentui/react-icons";
import type { ComponentType } from "react";
import type { Settings, TabId, View } from "../api";
import { t } from "../i18n";
import { useToggles } from "../toggles";
import { CodeMark, Legend, Trial } from "../ui";

interface Props {
  view: View;
  go: (tab: TabId) => void;
  act: (command: Promise<void>) => void;
}

interface TileProps {
  icon: ComponentType;
  label: string;
  value: string;
  detail: string;
  /** Something is still to do on the page the tile opens. */
  todo?: boolean;
  onOpen: () => void;
}

/** A summarised setting: its value in large type, and the page where it is changed. */
function Tile({ icon: Icon, label, value, detail, todo, onOpen }: TileProps) {
  return (
    <button type="button" className="tile" onClick={onOpen}>
      <Icon />
      <span className="tile-label">{label}</span>
      <ChevronRight16Regular className="chevron" />
      <b>{value}</b>
      <small>
        {todo && <Circle12Regular aria-label={t("To do", "À faire")} />}
        {detail}
      </small>
    </button>
  );
}

function cpuTile({ cpu }: Settings) {
  if (!cpu.enabled) {
    return {
      value: t("Not set", "Non réglé"),
      detail: t(`The game will start on all ${cpu.totalThreads} threads, normal priority`, `Le jeu démarrera sur ses ${cpu.totalThreads} threads, en priorité normale`),
    };
  }
  const priority = cpu.priorities.find((choice) => choice.key === cpu.priority)?.label ?? cpu.priority;
  return {
    value: t(`${cpu.threads} of ${cpu.totalThreads} threads`, `${cpu.threads} threads sur ${cpu.totalThreads}`),
    detail: t(`${priority} priority`, `Priorité ${priority.toLowerCase()}`),
  };
}

function fpsTile({ fps }: Settings) {
  if (!fps.enabled) {
    return { value: t("Game limit", "Limite du jeu"), detail: t("91 frames per second in Zombies", "91 images par seconde en Zombies") };
  }
  return {
    value: t(`${fps.limit} requested`, `${fps.limit} demandées`),
    detail:
      fps.effective === fps.limit
        ? t("The engine keeps this rate as is", "Cadence tenue telle quelle par le moteur")
        : t(`About ${fps.effective} in practice: the engine rounds`, `Environ ${fps.effective} réelles : le moteur arrondit`),
  };
}

function windowsTile({ windows }: Settings) {
  if (!windows) {
    return { value: t("Game not found", "Jeu introuvable"), detail: t("Start it once for these settings to be offered", "Lancez-le une fois pour que ces réglages soient proposés") };
  }
  const missing = [
    !windows.dedicatedGpu && t("dedicated graphics card", "carte graphique dédiée"),
    !windows.windowedOptimizationsOff && t("windowed optimizations to turn off", "optimisations en fenêtre à couper"),
  ].filter((name) => name !== false);
  if (missing.length === 0) {
    return { value: t("In place", "En place"), detail: t("Dedicated graphics card, windowed optimizations off", "Carte graphique dédiée, optimisations en fenêtre coupées") };
  }
  return {
    value: missing.length === 1 ? t("1 of 2 settings", "1 réglage sur 2") : t("To apply", "À appliquer"),
    detail: t(`Left: ${missing.join(", ")}`, `Reste : ${missing.join(", ")}`),
    todo: true,
  };
}

function characterTile(view: View) {
  const character = view.characters.find((candidate) => candidate.code === view.character);
  return character ? { value: character.name, detail: character.map } : { value: t("Not read yet", "Pas encore lu"), detail: t("Readable once the game is running", "Lisible une fois le jeu lancé") };
}

/** Home: the board of options with their keys, then what is set before the match. */
export function Home({ view, go, act }: Props) {
  const { isOn, canToggle, toggle } = useToggles(view, act);
  const options = view.groups.flatMap((group) => group.options);
  return (
    <>
      <section>
        <h2>{t("Options", "Options")}</h2>
        <div className="board">
          <div className="board-groups">
            {view.groups.map((group) => (
              <div key={group.title} role="group" aria-label={group.title}>
                <h3>{group.title}</h3>
                {group.options.map((option) => (
                  <button
                    key={option.key}
                    type="button"
                    className="opt"
                    role="switch"
                    aria-checked={isOn(option)}
                    disabled={!canToggle(option)}
                    title={option.hint}
                    onClick={() => toggle(option, !isOn(option))}
                  >
                    <span className="cap">{option.hotkey}</span>
                    <span className="opt-label">{option.label}</span>
                    {option.code && <CodeMark />}
                    {option.trial && <Trial />}
                  </button>
                ))}
              </div>
            ))}
          </div>
          <button type="button" className="board-more" onClick={() => go("partie")}>
            {t("Descriptions, values and actions", "Descriptions, valeurs et actions")}
            <ChevronRight16Regular className="chevron" />
          </button>
        </div>
        <Legend lit code={options.some((option) => option.code)} trial={options.some((option) => option.trial)} />
      </section>

      <section>
        <h2>{t("Before the match", "Avant la partie")}</h2>
        <div className="tiles">
          <Tile icon={DeveloperBoard20Regular} label={t("Processor", "Processeur")} {...cpuTile(view.settings)} onOpen={() => go("reglages")} />
          <Tile icon={TopSpeed20Regular} label={t("Frames per second", "Images par seconde")} {...fpsTile(view.settings)} onOpen={() => go("reglages")} />
          <Tile icon={Desktop20Regular} label={t("Windows, for this game", "Windows, pour ce jeu")} {...windowsTile(view.settings)} onOpen={() => go("reglages")} />
          <Tile icon={PersonStar20Regular} label={t("Character for the next match", "Personnage de la prochaine partie")} {...characterTile(view)} onOpen={() => go("salon")} />
        </div>
        <p className="caption">{t("Processor and Windows: a change applies from the next game launch.", "Processeur et Windows : un changement vaut pour le prochain lancement du jeu.")}</p>
      </section>
    </>
  );
}
