// Language of the pages: English by default, French on request (Settings page). The engine sends its
// language with every view; the pages' own text is picked here from it.
export type Lang = "en" | "fr";

let current: Lang = "en";

/** To call before every render, with the language of the view received from the engine. */
export function setLang(lang: Lang) {
  current = lang;
}

export function lang(): Lang {
  return current;
}

/** The text for the current language. */
export function t(en: string, fr: string): string {
  return current === "fr" ? fr : en;
}
