import { describe, expect, it } from "vitest";
import { batched } from "./batch";

const tick = () => new Promise((resolve) => setImmediate(resolve));

describe("batched", () => {
  it("sends the items of one tick as one message, in order", async () => {
    const sent: number[][] = [];
    const emit = batched<number>((items) => sent.push(items));
    for (let i = 1; i <= 500; i++) emit(i);
    expect(sent).toEqual([]);
    await tick();
    expect(sent).toEqual([Array.from({ length: 500 }, (_, i) => i + 1)]);
  });

  it("starts a new message on the next tick", async () => {
    const sent: string[][] = [];
    const emit = batched<string>((items) => sent.push(items));
    emit("a");
    emit("b");
    await tick();
    emit("c");
    await tick();
    await tick();
    expect(sent).toEqual([["a", "b"], ["c"]]);
  });
});
