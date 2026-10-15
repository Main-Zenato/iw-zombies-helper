import { DocumentText20Regular, Games20Regular, Home20Regular, People20Regular, Person20Regular, Settings20Regular } from "@fluentui/react-icons";
import { type ComponentType, useCallback, useEffect, useRef, useState } from "react";
import { api, type TabId, type View } from "./api";
import { Account } from "./pages/Account";
import { Home } from "./pages/Home";
import { Journal } from "./pages/Journal";
import { Lobby } from "./pages/Lobby";
import { Match } from "./pages/Match";
import { Settings } from "./pages/Settings";
import { setLang, t } from "./i18n";
import { InfoBar, Key } from "./ui";

const REFRESH = 200;

interface Tab {
  id: TabId;
  label: () => string;
  icon: ComponentType;
}

// The navigation pane: game pages at the top, tool pages at the bottom, like in Windows.
const PAGES: Tab[] = [
  { id: "accueil", label: () => t("Home", "Accueil"), icon: Home20Regular },
  { id: "salon", label: () => t("Lobby", "Salon"), icon: People20Regular },
  { id: "partie", label: () => t("In match", "En partie"), icon: Games20Regular },
  { id: "compte", label: () => t("Account", "Compte"), icon: Person20Regular },
];
const FOOTER: Tab[] = [
  { id: "reglages", label: () => t("Settings", "Réglages"), icon: Settings20Regular },
  { id: "journal", label: () => t("Log", "Journal"), icon: DocumentText20Regular },
];

export function App() {
  const [view, setView] = useState<View | null>(null);
  const [tab, setTab] = useState<TabId | null>(null);
  const alive = useRef(true);

  const refresh = useCallback(async () => {
    const next = await api.view().catch(() => null);
    if (next && alive.current) {
      setView(next);
      setTab((current) => current ?? next.startTab);
    }
  }, []);

  useEffect(() => {
    alive.current = true;
    void refresh();
    const timer = setInterval(refresh, REFRESH);
    return () => {
      alive.current = false;
      clearInterval(timer);
    };
  }, [refresh]);

  // The accent color is Windows': switches and selection follow it.
  const accent = view?.accent;
  useEffect(() => {
    if (accent) {
      document.documentElement.style.setProperty("--accent", accent);
    }
  }, [accent]);

  // The page language follows the interface's (screen readers, hyphenation).
  const language = view?.language;
  useEffect(() => {
    if (language) {
      document.documentElement.lang = language;
    }
  }, [language]);

  // After an action, the state is read again without waiting for the next refresh.
  const act = useCallback(
    (command: Promise<void>) => {
      void command.catch(() => undefined).then(() => setTimeout(refresh, 60));
    },
    [refresh],
  );

  if (!view || !tab) {
    return null;
  }
  return <Frame view={view} tab={tab} go={setTab} act={act} />;
}

interface FrameProps {
  view: View;
  tab: TabId;
  go: (tab: TabId) => void;
  act: (command: Promise<void>) => void;
  /** Log lines already known: a static preview has no engine to ask. */
  journal?: string[];
}

/** The whole window for a given state: the pane, the page header, the bar, the page. */
export function Frame({ view, tab, go, act, journal }: FrameProps) {
  // The whole window is drawn in this view's language.
  setLang(view.language);
  const active = view.groups.flatMap((group) => group.options).filter((option) => option.on).length;
  const title = [...PAGES, ...FOOTER].find((page) => page.id === tab)?.label();
  const summary =
    active === 0 ? t("No option on", "Aucune option active") : active === 1 ? t("1 option on", "1 option active") : t(`${active} options on`, `${active} options actives`);
  // A changed value (jump height) turns no option on: "Turn everything off" must still respond.
  const tweaked = view.tweaks.some((tweak) => tweak.value !== tweak.default);

  const items = (tabs: Tab[]) =>
    tabs.map(({ id, label, icon: Icon }) => (
      <button key={id} type="button" className="nav-item" aria-current={tab === id ? "page" : undefined} title={label()} onClick={() => go(id)}>
        <Icon />
        <span>{label()}</span>
      </button>
    ));

  return (
    <div className="app">
      <aside className="pane">
        <div className="status" title={view.state}>
          <i className="lamp" data-game={view.game} />
          <b>{view.state}</b>
          {view.game !== "absent" && <small>{summary}</small>}
        </div>
        <nav className="nav">{items(PAGES)}</nav>
        <nav className="nav" data-footer="true">
          {items(FOOTER)}
        </nav>
      </aside>

      <main className="page">
        <div className="page-inner">
          <header className="page-head">
            <h1>{title}</h1>
            <button type="button" className="btn" disabled={!view.ready || (active === 0 && !tweaked)} onClick={() => act(api.allOff())}>
              {t("Turn everything off", "Tout désactiver")}
              <Key name={view.keys.allOff} />
            </button>
          </header>
          {/* Guidance only shows where it helps; an action's result shows everywhere. */}
          {(view.message.tone !== "guide" || tab === "accueil" || tab === "partie") && <InfoBar tone={view.message.tone}>{view.message.text}</InfoBar>}
          {tab === "accueil" && <Home view={view} go={go} act={act} />}
          {tab === "salon" && <Lobby view={view} act={act} />}
          {tab === "partie" && <Match view={view} act={act} />}
          {tab === "compte" && <Account view={view} act={act} journal={journal} />}
          {tab === "reglages" && <Settings view={view} act={act} />}
          {tab === "journal" && <Journal view={view} act={act} journal={journal} />}
        </div>
      </main>
    </div>
  );
}
