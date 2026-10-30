import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const AREAS = 8;
const MODULES = 5;
const TAGS = ["@smoke", "@regression", "@slow", "@smoke @api", "@regression @api", "@wip"];
const WORDS =
  "checks that the order service returns the item with every field filled and the price rounded to cents".split(
    " ",
  );

/**
 * Writes a throwaway Karate project into `dir`: 8 areas x 5 modules x `featuresPerModule` features (5 gives
 * 2,000 tests and 600 HTTP calls). Each feature has 6 scenarios and one outline of 4 rows, and tags. Half the
 * scenarios call a Karate mock that the suite starts once; 1 test in 50 fails. Every 7th name is long enough
 * to be cut in the tree. Ported from docs/research/2026-10-29-big-project-perf/generate.mjs.
 */
export function writeBigProject(
  dir: string,
  featuresPerModule: number,
): { tests: number; calls: number } {
  const write = (rel: string, text: string) => {
    mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true });
    writeFileSync(path.join(dir, rel), text);
  };

  write(
    "karate-config.js",
    `function fn() {
  var mock = karate.callSingle('classpath:start-mock.feature');
  return { baseUrl: 'http://localhost:' + mock.port };
}
`,
  );
  write(
    "start-mock.feature",
    `@ignore
Feature: start the mock once per suite

Scenario:
  * def server = karate.start('mocks/api-mock.feature')
  * def port = server.port
`,
  );
  write(
    "mocks/api-mock.feature",
    `@ignore
Feature: items mock

Scenario: pathMatches('/items/{id}') && methodIs('get')
  * def response = { id: '#(pathParams.id)', name: 'item', tags: ['a', 'b'], price: 12.5 }

Scenario: pathMatches('/items') && methodIs('post')
  * def response = request
  * def responseStatus = 201

Scenario: pathMatches('/boom')
  * def responseStatus = 500
  * def response = { error: 'boom' }

Scenario:
  * def responseStatus = 404
  * def response = { error: 'not found' }
`,
  );

  let tests = 0;
  let calls = 0;
  for (let a = 1; a <= AREAS; a++)
    for (let m = 1; m <= MODULES; m++)
      for (let f = 1; f <= featuresPerModule; f++) {
        const lines = [
          `@area-${a} @module-${a}-${m}`,
          `Feature: area ${a} module ${m} feature ${f}`,
          "",
          "Background:",
          "  * url baseUrl",
          "",
        ];
        for (let s = 1; s <= 6; s++) {
          tests++;
          const name =
            tests % 7 === 0
              ? `scenario ${tests} ${WORDS.join(" ")}`
              : `scenario ${tests} ${WORDS.slice(0, 2 + (tests % 5)).join(" ")}`;
          lines.push(TAGS[s - 1], `Scenario: ${name}`);
          if (s % 2 === 1) {
            calls++;
            const kind = tests % 10;
            if (kind === 3)
              lines.push("  Given path 'missing'", "  When method get", "  Then status 404");
            else if (kind === 7)
              lines.push("  Given path 'boom'", "  When method get", "  Then status 500");
            else if (kind === 5)
              lines.push(
                "  Given path 'items'",
                "  And request { name: 'new', price: 1 }",
                "  When method post",
                "  Then status 201",
              );
            else
              lines.push(
                `  Given path 'items', ${tests}`,
                "  When method get",
                "  Then status 200",
              );
          } else lines.push(`  * def n = ${tests}`, `  * match n == ${tests}`);
          if (tests % 50 === 1) lines.push("  * match 1 == 2");
          lines.push("");
        }
        lines.push(
          "@outline",
          `Scenario Outline: outline ${a}.${m}.${f} row <n>`,
          "  * match <n> == <n>",
          "",
        );
        lines.push("Examples:", "  | n |");
        for (let r = 1; r <= 4; r++) {
          tests++;
          lines.push(`  | ${tests} |`);
        }
        lines.push("");
        write(`tests/area-${a}/module-${m}/feature-${f}.feature`, lines.join("\n"));
      }
  return { tests, calls };
}
