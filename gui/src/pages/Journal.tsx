import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { api, type View } from "../api";
import { t } from "../i18n";
import { Segmented, ToggleRow } from "../ui";

const REFRESH = 500;
// Words of a refusal, in French then in English: the log may mix both languages.
const REFUSED = [
  "indisponible", "impossible", "refus", "non reconnue", "inactif", "inaccessible", "non appliqu", "inattendue", "ILLISIBLE",
  "unavailable", "not recognised", "Trainer off", "not applied", "unexpected", "Could not",
];
/** Width of the time at the start of each line ("19:02:14  "). */
const STAMP = 10;

const isRefusal = (line: string) => REFUSED.some((word) => line.slice(STAMP).includes(word));

/** The session log lines, read again while the page showing them is open. */
export function useJournal(known: string[] = []): string[] {
  const [lines, setLines] = useState<string[]>(known);
  useEffect(() => {
    let alive = true;
    const read = async () => {
      const next = await api.journal().catch(() => null);
      if (alive && next) {
        // The log only keeps its last lines: the first and last are enough to see whether it moved.
        setLines((current) => (current.length === next.length && current.at(-1) === next.at(-1) && current[0] === next[0] ? current : next));
      }
    };
    void read();
    const timer = setInterval(read, REFRESH);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, []);
  return lines;
}

interface LogProps {
  lines: string[];
  /** Stays on the last line as long as the user has not scrolled up to read. */
  follow?: boolean;
  /** What shows when there are no lines. */
  empty: string;
}

export function Log({ lines, follow, empty }: LogProps) {
  const box = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  useLayoutEffect(() => {
    if (follow && pinned.current && box.current) {
      box.current.scrollTop = box.current.scrollHeight;
    }
  }, [lines, follow]);
  const onScroll = () => {
    const element = box.current;
    if (element) {
      pinned.current = element.scrollHeight - element.scrollTop - element.clientHeight < 24;
    }
  };
  return (
    <div className="log" ref={box} onScroll={onScroll} tabIndex={0} role="log" aria-label={t("Log", "Journal")}>
      {lines.length === 0 && <p className="log-empty">{empty}</p>}
      {lines.map((line, index) => {
        const text = line.slice(STAMP);
        // Surveys (indented lines, block markers) stay discreet so actions stand out.
        const tone = isRefusal(line) ? "refused" : text.startsWith(" ") || text.startsWith("---") ? "detail" : "action";
        return (
          <div key={index} className="log-line" data-tone={tone}>
            <time>{line.slice(0, STAMP)}</time>
            <span>{text}</span>
          </div>
        );
      })}
    </div>
  );
}

interface Props {
  view: View;
  act: (command: Promise<void>) => void;
  /** Lines already known (static previews). */
  journal?: string[];
}

type Filter = "all" | "refused";

export function Journal({ view, act, journal }: Props) {
  const lines = useJournal(journal);
  const [filter, setFilter] = useState<Filter>("all");
  const refusals = lines.filter(isRefusal);
  return (
    <div className="journal">
      <div className="toolbar">
        <Segmented
          label={t("Lines shown", "Lignes affichées")}
          value={filter}
          choices={[
            { value: "all", label: t(`All (${lines.length})`, `Tout (${lines.length})`) },
            { value: "refused", label: t(`Refused (${refusals.length})`, `Refus (${refusals.length})`) },
          ]}
          onChange={setFilter}
        />
        <span className="toolbar-end">
          <button type="button" className="btn" onClick={() => act(api.verify())}>
            {t("Check addresses", "Contrôler les adresses")}
          </button>
          <button
            type="button"
            className="btn"
            disabled={!view.profileReady}
            title={t(
              "Writes everything the tool can read from the game into a separate file, in the menu or in a match. Nothing is changed in the game.",
              "Écrit dans un fichier à part tout ce que l'outil sait lire du jeu, au menu comme en partie. Rien n'est modifié dans le jeu.",
            )}
            onClick={() => act(api.survey())}
          >
            {t("Full survey", "Relevé complet")}
          </button>
          {view.logPath && (
            <button type="button" className="btn" onClick={() => act(api.openLogs())}>
              {t("Open folder", "Ouvrir le dossier")}
            </button>
          )}
        </span>
      </div>
      <Log
        lines={filter === "refused" ? refusals : lines}
        follow
        empty={
          filter === "refused"
            ? t("Nothing refused in this session.", "Aucun refus dans cette session.")
            : t("No lines yet. Actions, surveys and refusals are written here.", "Aucune ligne pour l'instant. Les actions, les relevés et les refus s'écrivent ici.")
        }
      />
      <div className="group">
        <ToggleRow
          label={t("Watch what the game changes on the player", "Surveiller ce que le jeu change sur le joueur")}
          hint={t("For research: logs the values that move. Start it during a match.", "Pour la recherche : note dans le journal les valeurs qui bougent. À lancer pendant une partie.")}
          on={view.watching}
          onChange={(on) => act(api.setWatch(on))}
        />
      </div>
      <p className="path">
        {t(
          "Full survey: read-only, it writes a « releve-… » file (in French) into the logs folder. A second survey starts with what changed since the first.",
          "Relevé complet : en lecture seule, il écrit un fichier « releve-… » dans le dossier des journaux. Un second relevé commence par ce qui a changé depuis le premier.",
        )}
      </p>
      {view.logPath && <p className="path">{t(`This session's file: ${view.logPath}`, `Fichier de cette session : ${view.logPath}`)}</p>}
    </div>
  );
}
