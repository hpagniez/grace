// jsdom has no layout, so no ResizeObserver: the bottom panel's splitter needs one to exist.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
