// Builds assets/icon.ico (ADR 0013) from the two SVG sources. Run by hand after editing them: `npm run icon`.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "@playwright/test";

const SIZES = [16, 24, 32, 48, 64, 128, 256];
const asset = (name) => fileURLToPath(new URL(`../assets/${name}`, import.meta.url));

/** A Windows .ico holding each PNG as is (PNG entries need Windows Vista or later). */
export function ico(images) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(1, 2); // type 1: icon
  header.writeUInt16LE(images.length, 4);
  let offset = 6 + 16 * images.length;
  const entries = images.map(({ size, png }) => {
    const e = Buffer.alloc(16);
    e[0] = e[1] = size % 256; // 256 is written as 0
    e.writeUInt16LE(1, 4); // planes
    e.writeUInt16LE(32, 6); // bits per pixel
    e.writeUInt32LE(png.length, 8);
    e.writeUInt32LE(offset, 12);
    offset += png.length;
    return e;
  });
  return Buffer.concat([header, ...entries, ...images.map((i) => i.png)]);
}

async function main() {
  // Edge comes with Windows 11, so no Playwright browser download is needed.
  const browser = await chromium.launch({ channel: "msedge" });
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  const images = [];
  for (const size of SIZES) {
    // the small source is drawn for the taskbar sizes
    const svg = readFileSync(asset(size <= 32 ? "icon-small.svg" : "icon.svg"), "utf8");
    await page.setViewportSize({ width: size, height: size });
    await page.setContent(
      `<style>body{margin:0}svg{display:block}</style>` +
        svg.replace("<svg ", `<svg width="${size}" height="${size}" `),
    );
    images.push({ size, png: await page.screenshot({ omitBackground: true }) });
  }
  await browser.close();
  writeFileSync(asset("icon.ico"), ico(images));
  console.log(`assets/icon.ico: ${SIZES.join(", ")} px`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
