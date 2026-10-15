// Requests show immediately: a click is visible right away, without waiting for the engine's answer.
// Shared by the Home, In match and Lobby pages.
import { useEffect, useState } from "react";
import { api, type Option, type View } from "./api";

/** How long a request shows as taken into account while waiting for the engine's answer. */
export const PENDING_SHOWN = 900;

/**
 * A requested value shows instead of the engine's until the engine confirms it.
 * Refused, it goes back after a moment, and the top bar says why.
 */
export function usePending<T>(value: T): [T, (next: T) => void] {
  const [pending, setPending] = useState<{ value: T } | null>(null);
  useEffect(() => setPending(null), [value]);
  const hold = (next: T) => {
    const entry = { value: next };
    setPending(entry);
    setTimeout(() => setPending((current) => (current === entry ? null : current)), PENDING_SHOWN);
  };
  return [pending ? pending.value : value, hold];
}

export function useToggles(view: View, act: (command: Promise<void>) => void) {
  // Clicks not yet confirmed by the engine: the switch flips right away.
  const [pending, setPending] = useState<Record<string, boolean>>({});

  useEffect(() => {
    const states = new Map(view.groups.flatMap((group) => group.options).map((option) => [option.key, option.on]));
    setPending((current) => {
      const waiting = Object.entries(current).filter(([key, on]) => states.get(key) !== on);
      return waiting.length === Object.keys(current).length ? current : Object.fromEntries(waiting);
    });
  }, [view]);

  const playing = view.ready && view.game === "match";
  return {
    playing,
    isOn: (option: Option) => pending[option.key] ?? option.on,
    // Outside a match you can still turn off what stayed on, not turn anything else on.
    canToggle: (option: Option) => playing || (view.ready && option.on),
    toggle: (option: Option, on: boolean) => {
      setPending((current) => ({ ...current, [option.key]: on }));
      // Refused by the engine: the switch goes back, and the top bar says why.
      setTimeout(() => setPending(({ [option.key]: _, ...others }) => others), PENDING_SHOWN);
      act(api.toggle(option.key));
    },
  };
}
