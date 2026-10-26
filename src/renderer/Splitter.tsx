import { createSignal, onSettled } from "solid-js";
import { clampPanel, keyHeight, maxPanel, PANEL_MIN, savePanelHeight } from "./panelHeight";

/** The strip at the top of the bottom panel that resizes it, with the pointer or the keyboard. */
export function Splitter(props: {
  panel: () => HTMLElement | undefined;
  main: () => HTMLElement | undefined;
  setHeight: (h: number) => void;
  setDragging: (on: boolean) => void;
}) {
  // Read from the layout, not from the height signal: the panel shows the saved height clamped.
  const sizes = () => {
    const p = props.panel()?.getBoundingClientRect().height ?? 0;
    const m = props.main()?.getBoundingClientRect().height ?? 0;
    return { shown: Math.round(p), max: maxPanel(p + m) };
  };
  const [aria, setAria] = createSignal({ shown: 0, max: PANEL_MIN });
  onSettled(() => {
    const ro = new ResizeObserver(() => setAria(sizes()));
    for (const e of [props.panel(), props.main()]) if (e) ro.observe(e);
    return () => ro.disconnect();
  });

  let drag: { y: number; start: number; max: number; h: number } | null = null;
  const start = (e: PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault(); // no text selection
    const { shown, max } = sizes();
    drag = { y: e.clientY, start: shown, max, h: shown };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    props.setDragging(true);
  };
  const move = (e: PointerEvent) => {
    if (!drag) return;
    drag.h = clampPanel(drag.start + drag.y - e.clientY, drag.max);
    props.setHeight(drag.h);
  };
  // pointerup, pointercancel and lostpointercapture all end it; only the first one counts.
  const end = () => {
    if (!drag) return;
    if (drag.h !== drag.start) savePanelHeight(drag.h);
    drag = null;
    props.setDragging(false);
  };
  const key = (e: KeyboardEvent) => {
    const { shown, max } = sizes();
    const h = keyHeight(e.key, shown, max);
    if (h === null) return;
    e.preventDefault();
    if (h === shown) return; // at a limit: keep the saved height
    props.setHeight(h);
    savePanelHeight(h);
  };

  return (
    <div
      data-testid="splitter"
      role="separator"
      tabindex="0"
      aria-orientation="horizontal"
      aria-label="Resize bottom panel"
      aria-valuemin={PANEL_MIN}
      aria-valuemax={aria().max}
      aria-valuenow={aria().shown}
      class="h-1.5 shrink-0 cursor-row-resize bg-zinc-50 outline-none focus-visible:ring-2 focus-visible:ring-sky-600 focus-visible:ring-inset dark:bg-zinc-800"
      onPointerDown={start}
      onPointerMove={move}
      onPointerUp={end}
      onPointerCancel={end}
      onLostPointerCapture={end}
      onKeyDown={key}
    />
  );
}
