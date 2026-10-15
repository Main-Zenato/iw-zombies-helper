// Static previews of the interface: one HTML page per tab, with a fake state, without the game and
// without opening the window. Used to judge a layout (open them in a browser).
//   pnpm apercu <output folder> [absent|menu|match] [accent color, #rrggbb] [actives]
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { renderToStaticMarkup } from "react-dom/server";
import type { TabId, View } from "../src/api";

// The word `actives`, anywhere among the arguments, turns a few options on: the fake engine starts
// with everything off, like the real tool, and a preview may want to show the on state.
const args = process.argv.slice(2);
const lit = args.includes("actives");
const [out = "apercu", game = "match", accent] = args.filter((arg) => arg !== "actives");
// The interface modules expect to run in a window.
Object.assign(globalThis, { window: {}, location: { search: `?etat=${game}${lit ? "&actives=1" : ""}` } });

const { respond } = await import("../src/mock");
const { Frame } = await import("../src/App");

const view = { ...(respond("view", {}) as View), ...(accent ? { accent } : {}) };

const css = readFileSync(join(import.meta.dirname, "../src/styles.css"), "utf8");
const tabs: TabId[] = ["accueil", "salon", "partie", "compte", "reglages", "journal"];
mkdirSync(out, { recursive: true });
for (const tab of tabs) {
  const page = renderToStaticMarkup(<Frame view={view} tab={tab} go={() => undefined} act={() => undefined} journal={respond("journal", {}) as string[]} />);
  const html = `<!doctype html><html lang="fr" style="--accent:${view.accent}"><head><meta charset="utf-8"><title>${tab}</title><style>${css}</style></head><body><div id="root">${page}</div></body></html>`;
  writeFileSync(join(out, `${tab}.html`), html);
}
console.log(`${tabs.length} aperçus écrits dans ${out}`);
