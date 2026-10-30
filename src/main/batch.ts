/**
 * A function that queues each item and sends the items of one tick as one message, in order, so the
 * window applies a whole poll of run events in one task instead of one per event (#123).
 */
export function batched<T>(send: (items: T[]) => void): (item: T) => void {
  let queued: T[] = [];
  return (item) => {
    if (queued.push(item) > 1) return;
    setImmediate(() => {
      const items = queued;
      queued = [];
      send(items);
    });
  };
}
