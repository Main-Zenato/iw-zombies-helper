import { useState } from "react";
import { api, type ProfileAction, type View } from "../api";
import { t } from "../i18n";
import { InfoBar, Row, Section } from "../ui";
import { Log, useJournal } from "./Journal";

interface Props {
  view: View;
  act: (command: Promise<void>) => void;
  /** Log lines already known (static previews). */
  journal?: string[];
}

interface Entry {
  action: ProfileAction;
  label: string;
  hint: string;
  button: string;
}

/** The progress actions, in the current language. */
function progress(): Entry[] {
  return [
    {
      action: "show",
      label: t("Read the profile", "Lire le profil"),
      hint: t("XP, Director's Cut, weapon XP, contracts and character, written below. Changes nothing.", "XP, Director's Cut, XP d'armes, contrats et personnage, écrits ci-dessous. Ne modifie rien."),
      button: t("Read", "Lire"),
    },
    { action: "max-level", label: t("Level 999", "Niveau 999"), hint: t("Writes the XP for level 999.", "Écrit l'XP du niveau 999."), button: t("Apply", "Appliquer") },
    {
      action: "weapon-xp",
      label: t("Max weapon XP", "XP d'armes au maximum"),
      hint: t("Writes the maximum XP to all 47 weapon slots.", "Écrit l'XP maximale sur les 47 emplacements d'armes."),
      button: t("Apply", "Appliquer"),
    },
    { action: "directors-cut", label: "Director's Cut", hint: t("Marks the mode as unlocked in the profile.", "Marque le mode comme débloqué dans le profil."), button: t("Unlock", "Débloquer") },
  ];
}

/** The account's saved progress: permanent writes, hence the warning at the top. */
export function Account({ view, act, journal }: Props) {
  const [armed, setArmed] = useState(false);
  const lines = useJournal(journal);
  // On iw7-mod, the account's progress is not the one from the official servers: nothing to do here.
  const ready = view.profileReady && !view.modClient;
  return (
    <>
      {view.modClient && (
        <InfoBar tone="guide">
          {t(
            "Not available on iw7-mod: these actions are for the account of the Steam game.",
            "Indisponible sur iw7-mod : ces actions concernent le compte du jeu Steam.",
          )}
        </InfoBar>
      )}
      <InfoBar tone="notice">
        <b>{t("Permanent.", "Définitif.")}</b>{" "}
        {t(
          "These actions change the account's saved progress: the game saves the profile and sends it online.",
          "Ces actions modifient la progression enregistrée du compte : le jeu sauvegarde le profil et l'envoie en ligne.",
        )}
        {ready
          ? t(" Do them from the Zombies menu.", " À faire depuis le menu Zombies.")
          : t(" Start the game and open the Zombies menu: they will respond.", " Lancez le jeu et ouvrez le menu Zombies : elles répondront.")}
      </InfoBar>

      <Section title={t("Progress", "Progression")}>
        {progress().map((entry) => (
          <Row key={entry.action} label={entry.label} hint={entry.hint} off={!ready}>
            <button type="button" className="btn" disabled={!ready} onClick={() => act(api.profile(entry.action))}>
              {entry.button}
            </button>
          </Row>
        ))}
      </Section>

      <Section title={t("Reset", "Remise à zéro")}>
        <Row
          label={t("Reset the account", "Remettre le compte à zéro")}
          hint={t("Erases all of this account's progress. There is no going back.", "Efface toute la progression de ce compte. Sans retour possible.")}
          off={!ready}
        >
          <label className="check">
            <input type="checkbox" checked={armed} disabled={!ready} onChange={(event) => setArmed(event.target.checked)} />
            {t("I confirm", "Je confirme")}
          </label>
          <button
            type="button"
            className="btn"
            disabled={!ready || !armed}
            onClick={() => {
              setArmed(false);
              act(api.profile("reset"));
            }}
          >
            {t("Erase", "Effacer")}
          </button>
        </Row>
      </Section>

      <section>
        <h2>{t("Result", "Résultat")}</h2>
        <Log lines={lines.slice(-8)} empty={t("The result of an action is written here.", "Le résultat d'une action s'écrit ici.")} />
      </section>
    </>
  );
}
