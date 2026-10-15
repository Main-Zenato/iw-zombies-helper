import { api, type View } from "../api";
import { t } from "../i18n";
import { usePending } from "../toggles";
import { InfoBar, Row, Section } from "../ui";

interface Props {
  view: View;
  act: (command: Promise<void>) => void;
}

/** The lobby: what is set before starting the match, from the Zombies menu. */
export function Lobby({ view, act }: Props) {
  const ready = view.profileReady;
  const [selected, hold] = usePending(view.character);
  const choose = (code: number) => {
    hold(code);
    act(api.profile(`character-${code}`));
  };
  const heading = t("Character for the next match", "Personnage de la prochaine partie");

  return (
    <>
      {!ready && <InfoBar tone="guide">{t("Profile not readable yet. Start the game and open the Zombies menu.", "Profil illisible pour l'instant. Lancez le jeu et ouvrez le menu Zombies.")}</InfoBar>}

      <section>
        <h2>{heading}</h2>
        <div className="group" role="radiogroup" aria-label={heading}>
          {view.characters.map((character) => (
            <button
              key={character.code}
              type="button"
              className="choice"
              role="radio"
              aria-checked={selected === character.code}
              disabled={!ready || view.character === null}
              onClick={() => selected !== character.code && choose(character.code)}
            >
              <span className="radio" />
              <span className="choice-name">{character.name}</span>
              <span className="choice-detail">{character.code === 0 ? t("One of the map's four characters, at random", "Un des quatre personnages de la carte, au hasard") : character.map}</span>
            </button>
          ))}
        </div>
        <p className="caption">
          {t(
            "The choice lasts for one match, on the map shown. The game then goes back to Normal draw. Set it in the lobby, before starting the match.",
            "Le choix vaut pour une seule partie, sur la carte indiquée. Le jeu revient ensuite à « Tirage normal ». À régler dans le salon, avant de lancer la partie.",
          )}
        </p>
      </section>

      <Section title={t("Contracts", "Contrats")}>
        <Row
          label={t("Prepare both contracts", "Préparer les deux contrats")}
          hint={t("Then start a match and buy something: both contracts are completed.", "Lancez ensuite une partie et achetez quelque chose : les deux contrats sont validés.")}
          off={!ready}
        >
          <button type="button" className="btn" disabled={!ready} onClick={() => act(api.profile("contracts"))}>
            {t("Prepare", "Préparer")}
          </button>
        </Row>
        <Row label={t("Change contracts", "Changer de contrats")} hint={t("The game draws two new contracts.", "Le jeu tire deux nouveaux contrats.")} off={!ready}>
          <button type="button" className="btn" disabled={!ready} onClick={() => act(api.profile("reroll-contracts"))}>
            {t("Change", "Changer")}
          </button>
        </Row>
      </Section>
    </>
  );
}
