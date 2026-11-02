// Spike for #179: writes a long suite whose steps are mostly Karate's own JavaScript, where C2-compiled code should
// beat C1's. It goes where measure.mjs looks for a project of size "js".
//   node js-project.mjs [features]   (10 scenarios each, default 100)
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

const features = Number(process.argv[2] ?? 100);
const root = path.join(os.tmpdir(), "grace-java-memory", "project-js");
rmSync(root, { recursive: true, force: true });
mkdirSync(path.join(root, "tests"), { recursive: true });
writeFileSync(
  path.join(root, "karate-config.js"),
  `function fn() {
  return {
    churn: function(n) {
      var s = 0;
      for (var i = 0; i < n; i++) { s = (s + i * 31 + (i % 7)) % 1000003; }
      return s;
    },
    make: function(n) {
      return karate.repeat(n, function(i) { return { id: i, name: 'item ' + i, price: i % 97, tags: ['a', 'b'] }; });
    }
  };
}
`,
);
for (let f = 1; f <= features; f++) {
  const lines = [`Feature: js ${f}`, ""];
  for (let s = 1; s <= 10; s++)
    lines.push(
      `Scenario: js ${f}.${s}`,
      `  * def total = churn(${300000 + s})`,
      "  * assert total > 0",
      "  * def items = make(2000)",
      "  * def cheap = karate.filter(items, function(x) { return x.price < 10 })",
      "  * def names = karate.map(cheap, function(x) { return x.name.toUpperCase() })",
      "  * def back = JSON.parse(JSON.stringify(items))",
      "  * match back[1999] == { id: 1999, name: 'item 1999', price: '#number', tags: ['a', 'b'] }",
      "  * match each cheap contains { price: '#? _ < 10' }",
      "  * match names[0] == 'ITEM 0'",
      "",
    );
  writeFileSync(path.join(root, "tests", `js-${f}.feature`), lines.join("\n"));
}
console.log(`${features * 10} scenarios in ${root}`);
