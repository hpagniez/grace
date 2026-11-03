// jsdom has no layout, so no ResizeObserver: the bottom panel's splitter needs one to exist.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// jsdom has no CSS Custom Highlight API: the console paints its find matches with it (console search R4).
(globalThis as { CSS?: object }).CSS ??= {};
(CSS as { highlights?: Map<string, unknown> }).highlights ??= new Map();
globalThis.Highlight ??= class extends Set<AbstractRange> {
  constructor(...ranges: AbstractRange[]) {
    super(ranges);
  }
} as unknown as typeof Highlight;

// nor checkVisibility: the console's keys act only while it is on screen (console search R5.3).
Element.prototype.checkVisibility ??= () => true;
