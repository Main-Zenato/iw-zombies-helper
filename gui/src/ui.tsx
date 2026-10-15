// Parts shared by the pages, sized like Windows 11 controls: group of setting rows, switch,
// number box, selector, slider, information bar.
import { CheckmarkCircle16Filled, ChevronDown12Regular, ChevronUp12Regular, DismissCircle16Filled, Info16Filled, Warning12Filled, Warning16Filled } from "@fluentui/react-icons";
import { type CSSProperties, type ReactNode, useState } from "react";
import type { Tone } from "./api";
import { lang, t } from "./i18n";
import { usePending } from "./toggles";

/** A keyboard key, as printed on it. Nothing if the action has no shortcut. */
export function Key({ name }: { name: string }) {
  return name ? <span className="key">{name}</span> : null;
}

/** "Trial" tag: the feature has never been seen working in game. */
export function Trial() {
  return (
    <span className="tag" title={t("Never seen working in game yet", "Jamais vu fonctionner en jeu")}>
      {t("Trial", "Essai")}
    </span>
  );
}

/** Yellow triangle: the feature changes the game's code. It is the only meaning of yellow in the interface. */
export function CodeMark() {
  return (
    <Warning12Filled
      className="mark"
      aria-label={t("Changes the game's code", "Modifie le code du jeu")}
      title={t("Changes the game's code: turn it off before leaving the match", "Modifie le code du jeu : à couper avant de quitter la partie")}
    />
  );
}

interface LegendProps {
  /** Also reminds that a lit key means "option on" (Home board). */
  lit?: boolean;
  code?: boolean;
  trial?: boolean;
}

/** The legend of the marks, under the options: only those on screen are explained. */
export function Legend({ lit, code, trial }: LegendProps) {
  if (!lit && !code && !trial) {
    return null;
  }
  return (
    <p className="legend">
      {lit && (
        <span>
          <i className="cap" data-on="true" />
          {t("On", "Activée")}
        </span>
      )}
      {code && (
        <span>
          <Warning12Filled className="mark" />
          {t("Changes the game's code: turn it off before leaving the match", "Modifie le code du jeu : à couper avant de quitter la partie")}
        </span>
      )}
      {trial && (
        <span>
          <span className="tag">{t("Trial", "Essai")}</span>
          {t("Never seen working in game yet", "Jamais vu fonctionner en jeu")}
        </span>
      )}
    </p>
  );
}

interface RowTextProps {
  label: string;
  hint?: ReactNode;
  code?: boolean;
  trial?: boolean;
}

export function RowText({ label, hint, code, trial }: RowTextProps) {
  return (
    <span className="row-text">
      <b>
        {label}
        {code && <CodeMark />}
        {trial && <Trial />}
      </b>
      {hint && <small>{hint}</small>}
    </span>
  );
}

interface SectionProps {
  title: string;
  /** Detail under the group. */
  note?: ReactNode;
  children: ReactNode;
}

/** A title and its group of rows, separated by a hairline like in Windows Settings. */
export function Section({ title, note, children }: SectionProps) {
  return (
    <section>
      <h2>{title}</h2>
      <div className="group">{children}</div>
      {note && <p className="caption">{note}</p>}
    </section>
  );
}

interface RowProps extends RowTextProps {
  /** The row does not respond for now: its text stays readable, dimmed. */
  off?: boolean;
  children?: ReactNode;
}

/** A setting row: text on the left, control on the right. */
export function Row({ off, children, ...text }: RowProps) {
  return (
    <div className="row" data-off={off}>
      <RowText {...text} />
      {children && <span className="row-side">{children}</span>}
    </div>
  );
}

interface ToggleRowProps extends RowTextProps {
  on: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
}

/** A whole clickable row, with its switch on the right. */
export function ToggleRow({ on, disabled, onChange, ...text }: ToggleRowProps) {
  return (
    <button type="button" className="row" role="switch" aria-checked={on} disabled={disabled} onClick={() => onChange(!on)}>
      <RowText {...text} />
      <span className="row-side">
        <span className="toggle-state">{on ? t("On", "Activé") : t("Off", "Désactivé")}</span>
        <span className="toggle" />
      </span>
    </button>
  );
}

interface OptionRowProps extends ToggleRowProps {
  /** Keyboard key that does the same thing in game; it lights up when the option is on. */
  hotkey: string;
}

/** A game option, as a table row: key, name, description, switch. */
export function OptionRow({ label, hint, code, trial, hotkey, on, disabled, onChange }: OptionRowProps) {
  return (
    <button type="button" className="option" role="switch" aria-checked={on} disabled={disabled} onClick={() => onChange(!on)}>
      <span className="cap">{hotkey}</span>
      <span className="option-name">
        <span>{label}</span>
        {code && <CodeMark />}
        {trial && <Trial />}
      </span>
      <span className="option-hint">{hint}</span>
      <span className="option-side">
        <span className="toggle-state">{on ? t("On", "Activé") : t("Off", "Désactivé")}</span>
        <span className="toggle" />
      </span>
    </button>
  );
}

interface InfoBarProps {
  tone: Tone | "notice";
  children: ReactNode;
}

/** Windows' information bar: what to do, what was just done, what was refused. */
export function InfoBar({ tone, children }: InfoBarProps) {
  const icons = { guide: Info16Filled, notice: Warning16Filled, done: CheckmarkCircle16Filled, refused: DismissCircle16Filled, error: DismissCircle16Filled };
  const Icon = icons[tone];
  return (
    <div className="infobar" data-tone={tone} role="status">
      <Icon />
      <p>{children}</p>
    </div>
  );
}

interface SegmentedProps<T extends string> {
  label: string;
  value: T;
  choices: { value: T; label: string }[];
  disabled?: boolean;
  onChange: (value: T) => void;
}

export function Segmented<T extends string>({ label, value, choices, disabled, onChange }: SegmentedProps<T>) {
  return (
    <div className="segmented" role="group" aria-label={label}>
      {choices.map((choice) => (
        <button key={choice.value} type="button" aria-pressed={choice.value === value} disabled={disabled} onClick={() => choice.value !== value && onChange(choice.value)}>
          {choice.label}
        </button>
      ))}
    </div>
  );
}

/** Number of decimals of a step: 0 for 10, 2 for 0.05. */
function decimalsOf(step: number): number {
  const text = String(step);
  const exponent = /e-(\d+)$/.exec(text);
  return exponent ? Number(exponent[1]) : (text.split(".")[1]?.length ?? 0);
}

/** A number with its step's decimals (0.50 for a step of 0.05), with a decimal comma in French. */
export function formatNumber(value: number, step = 1): string {
  const text = value.toFixed(decimalsOf(step)).replace("-", "−");
  return lang() === "fr" ? text.replace(".", ",") : text;
}

/** Brings a value within its bounds, at its step's precision (without snapping it to the step). */
function bound(value: number, min: number, max: number, step: number): number {
  return Number(Math.min(max, Math.max(min, value)).toFixed(decimalsOf(step)));
}

interface NumberBoxProps {
  label: string;
  value: number;
  min: number;
  max: number;
  /** Step of the arrows; its decimals set the precision accepted when typing. */
  step?: number;
  /** Unit shown after the value. */
  unit?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}

/** A number with its arrows; the value always stays within its bounds. */
export function NumberBox({ label, value, min, max, step = 1, unit, disabled, onChange }: NumberBoxProps) {
  // What is being typed: the value is only bounded and sent once typing is finished.
  const [draft, setDraft] = useState<string | null>(null);
  const set = (next: number) => {
    const bounded = bound(next, min, max, step);
    if (bounded !== value) {
      onChange(bounded);
    }
  };
  const commit = () => {
    const typed = Number((draft ?? "").trim().replace(",", ".").replace("−", "-"));
    if (draft !== null && draft.trim() !== "" && Number.isFinite(typed)) {
      set(typed);
    }
    setDraft(null);
  };
  const nudge = (direction: 1 | -1) => {
    setDraft(null);
    set(value + direction * step);
  };
  return (
    <span className="numberbox" data-off={disabled}>
      <input
        type="text"
        inputMode="decimal"
        role="spinbutton"
        aria-label={label}
        aria-valuenow={value}
        aria-valuemin={min}
        aria-valuemax={max}
        value={draft ?? formatNumber(value, step)}
        disabled={disabled}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            commit();
          } else if (event.key === "Escape") {
            setDraft(null);
          } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
            event.preventDefault();
            nudge(event.key === "ArrowUp" ? 1 : -1);
          }
        }}
      />
      {unit && <span className="unit">{unit}</span>}
      <button type="button" tabIndex={-1} aria-label={t(`${label}: up`, `${label} : plus`)} disabled={disabled || value >= max} onClick={() => nudge(1)}>
        <ChevronUp12Regular />
      </button>
      <button type="button" tabIndex={-1} aria-label={t(`${label}: down`, `${label} : moins`)} disabled={disabled || value <= min} onClick={() => nudge(-1)}>
        <ChevronDown12Regular />
      </button>
    </span>
  );
}

interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  disabled?: boolean;
  /** What shows to the right of the slider; nothing if the value is already written next to it. */
  format?: (value: number) => string;
  /** Value hovered while dragging, `null` once the slider is released. */
  onDraft?: (value: number | null) => void;
  /** Called on release, not while dragging. */
  onCommit: (value: number) => void;
}

export function Slider({ label, value, min, max, step = 1, disabled, format, onDraft, onCommit }: SliderProps) {
  const [draft, setDraft] = useState<number | null>(null);
  // Once released, the slider stays where it was put until the engine answers.
  const [settled, hold] = usePending(value);
  const shown = draft ?? settled;
  const move = (next: number | null) => {
    setDraft(next);
    onDraft?.(next);
  };
  const commit = () => {
    if (draft === null) {
      return;
    }
    if (draft !== settled) {
      hold(draft);
      onCommit(draft);
    }
    move(null);
  };
  const fill = { "--fill": `${max > min ? ((shown - min) / (max - min)) * 100 : 100}%` } as CSSProperties;
  return (
    <span className="slider">
      <input
        type="range"
        aria-label={label}
        aria-valuetext={format?.(shown)}
        style={fill}
        min={min}
        max={max}
        step={step}
        value={shown}
        disabled={disabled}
        onChange={(event) => move(bound(Number(event.target.value), min, max, step))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
      {format && <output>{format(shown)}</output>}
    </span>
  );
}
