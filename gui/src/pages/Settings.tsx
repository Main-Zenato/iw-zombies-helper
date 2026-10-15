import { CheckmarkCircle16Filled, Circle16Regular } from "@fluentui/react-icons";
import { api, type View } from "../api";
import { type Lang, t } from "../i18n";
import { Key, NumberBox, Row, Section, Segmented, Slider, ToggleRow } from "../ui";

interface Props {
  view: View;
  act: (command: Promise<void>) => void;
}

/** The languages offered, each written in its own language. */
const LANGUAGES: { value: Lang; label: string }[] = [
  { value: "en", label: "English" },
  { value: "fr", label: "Français" },
];

/** State of a Windows setting: in place, or still to do. */
function State({ ok, done, todo }: { ok: boolean; done: string; todo: string }) {
  return (
    <span className="state" data-ok={ok}>
      {ok ? <CheckmarkCircle16Filled /> : <Circle16Regular />}
      {ok ? done : todo}
    </span>
  );
}

export function Settings({ view, act }: Props) {
  const { cpu, fps, windows, onTop } = view.settings;
  return (
    <>
      <Section title={t("Language", "Langue")}>
        <Row label={t("Interface and log language", "Langue de l'interface et du journal")} hint={t("Saved in iw-helper.toml.", "Enregistrée dans iw-helper.toml.")}>
          <Segmented label={t("Language", "Langue")} value={view.language} choices={LANGUAGES} onChange={(language) => act(api.setLanguage(language))} />
        </Row>
      </Section>

      <Section title={t("Processor", "Processeur")} note={t(`${cpu.outcome} Takes effect at the next game launch.`, `${cpu.outcome} Effet au prochain lancement du jeu.`)}>
        <ToggleRow
          label={t("Set the game's affinity and priority", "Régler l'affinité et la priorité du jeu")}
          hint={t("Off: the game starts on every thread, normal priority.", "Désactivé : le jeu démarre sur tous les threads, en priorité normale.")}
          on={cpu.enabled}
          onChange={(on) => act(api.setCpuEnabled(on))}
        />
        <ToggleRow
          label={t("Use sibling threads (SMT)", "Utiliser les threads jumeaux (SMT)")}
          hint={t("Off: the game uses one thread per core.", "Désactivé : le jeu n'utilise qu'un thread par cœur.")}
          on={cpu.smt}
          disabled={!cpu.enabled}
          onChange={(on) => act(api.setSmt(on))}
        />
        <Row label={t("Cores used", "Cœurs utilisés")} off={!cpu.enabled}>
          <Slider
            label={t("Cores used", "Cœurs utilisés")}
            value={cpu.cores}
            min={1}
            max={cpu.coreCount}
            disabled={!cpu.enabled}
            format={(cores) => t(`${cores} of ${cpu.coreCount}`, `${cores} sur ${cpu.coreCount}`)}
            onCommit={(cores) => act(api.setCores(cores))}
          />
        </Row>
        <Row label={t("Game priority", "Priorité du jeu")} off={!cpu.enabled}>
          <Segmented
            label={t("Game priority", "Priorité du jeu")}
            value={cpu.priority}
            choices={cpu.priorities.map((priority) => ({ value: priority.key, label: priority.label }))}
            disabled={!cpu.enabled}
            onChange={(priority) => act(api.setPriority(priority))}
          />
        </Row>
      </Section>

      <Section title={t("Frames per second", "Images par seconde")}>
        <ToggleRow
          label={t("Replace the game's limit", "Remplacer la limite du jeu")}
          hint={t("The engine caps Zombies at 91 frames per second.", "Le moteur plafonne à 91 images par seconde en Zombies.")}
          on={fps.enabled}
          onChange={(on) => act(api.setFps(on, fps.limit))}
        />
        <Row
          label={t("Requested limit", "Limite demandée")}
          hint={
            fps.effective === fps.limit
              ? t("The engine keeps it as is.", "Tenue telle quelle par le moteur.")
              : t(`About ${fps.effective} in practice: the engine rounds.`, `Soit environ ${fps.effective} réelles : le moteur arrondit.`)
          }
          off={!fps.enabled}
        >
          <NumberBox label={t("Requested limit", "Limite demandée")} value={fps.limit} min={30} max={1000} step={10} disabled={!fps.enabled} onChange={(limit) => act(api.setFps(true, limit))} />
        </Row>
      </Section>

      <Section title={t("Windows, for this game", "Windows, pour ce jeu")} note={windows ? t("Takes effect at the next game launch.", "Effet au prochain lancement du jeu.") : undefined}>
        {windows ? (
          <>
            <Row label={t("Dedicated graphics card", "Carte graphique dédiée")} hint={t("Windows starts the game on the most powerful card.", "Windows lance le jeu sur la carte la plus performante.")}>
              <State ok={windows.dedicatedGpu} done={t("Forced", "Imposée")} todo={t("To force", "À imposer")} />
            </Row>
            <Row
              label={t("Optimizations for windowed games", "Optimisations pour les jeux en fenêtre")}
              hint={t("A known cause of this game's crashes on Windows 11.", "Cause connue de plantages de ce jeu sous Windows 11.")}
            >
              <State ok={windows.windowedOptimizationsOff} done={t("Off", "Coupées")} todo={t("To turn off", "À couper")} />
            </Row>
            {!(windows.dedicatedGpu && windows.windowedOptimizationsOff) && (
              <Row label={t("Apply what is missing", "Appliquer ce qui manque")} hint={t("Writes both settings in Windows, for this game only.", "Écrit les deux réglages dans Windows, pour ce jeu seulement.")}>
                <button type="button" className="btn" data-accent="true" onClick={() => act(api.applyWindowsSettings())}>
                  {t("Apply", "Appliquer")}
                </button>
              </Row>
            )}
          </>
        ) : (
          <Row
            label={t("Game not found at its usual Steam location", "Jeu introuvable à son emplacement Steam habituel")}
            hint={t("Start it once: these settings will be offered.", "Lancez-le une fois : ces réglages seront proposés.")}
            off
          />
        )}
      </Section>

      <Section title={t("This window", "Cette fenêtre")}>
        <ToggleRow
          label={t("Keep on top", "Garder au premier plan")}
          hint={t("Stays above the game when it runs in borderless window.", "Reste au-dessus du jeu s'il tourne en fenêtré sans bordure.")}
          on={onTop}
          onChange={(on) => act(api.setOnTop(on))}
        />
      </Section>

      <section>
        <h2>{t("Keyboard shortcuts", "Raccourcis clavier")}</h2>
        <div className="keys">
          {view.hotkeys.map((hotkey) => (
            <div key={hotkey.label}>
              <span>{hotkey.label}</span>
              {hotkey.key ? <Key name={hotkey.key} /> : <i>{t("No key", "Aucune touche")}</i>}
            </div>
          ))}
        </div>
        <p className="caption">
          {t(
            "Active while the game window is in front. Change them in the [hotkeys] section of iw-helper.toml; they apply from the next launch.",
            "Actifs quand la fenêtre du jeu est au premier plan. Ils se changent dans la section [hotkeys] du fichier iw-helper.toml, puis au prochain lancement.",
          )}
        </p>
      </section>
    </>
  );
}
