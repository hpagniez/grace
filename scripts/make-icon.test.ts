import { describe, expect, it } from "vitest";
import { ico } from "./make-icon.mjs";

// Stand-ins for PNGs: ico() copies the bytes as they are.
const png = (n: number, fill: number) => Buffer.alloc(n, fill);

describe("ico", () => {
  it("writes the header and one entry per image", () => {
    const out = ico([
      { size: 16, png: png(10, 1) },
      { size: 32, png: png(20, 2) },
    ]);
    expect([out.readUInt16LE(0), out.readUInt16LE(2), out.readUInt16LE(4)]).toEqual([0, 1, 2]);
    const entry = (i: number) => out.subarray(6 + 16 * i, 6 + 16 * (i + 1));
    expect([...entry(0).subarray(0, 4)]).toEqual([16, 16, 0, 0]);
    expect([entry(0).readUInt16LE(4), entry(0).readUInt16LE(6)]).toEqual([1, 32]);
    expect(entry(0).readUInt32LE(8)).toBe(10);
    expect([...entry(1).subarray(0, 2)]).toEqual([32, 32]);
    expect(entry(1).readUInt32LE(8)).toBe(20);
    expect(out.length).toBe(6 + 32 + 30);
  });

  it("writes 256 as 0", () => {
    const out = ico([{ size: 256, png: png(4, 9) }]);
    expect([out[6], out[7]]).toEqual([0, 0]);
  });

  it("points each entry at its png", () => {
    const images = [
      { size: 16, png: png(10, 1) },
      { size: 48, png: png(7, 2) },
      { size: 256, png: png(5, 3) },
    ];
    const out = ico(images);
    images.forEach((image, i) => {
      const offset = out.readUInt32LE(6 + 16 * i + 12);
      expect(out.subarray(offset, offset + image.png.length)).toEqual(image.png);
    });
  });
});
