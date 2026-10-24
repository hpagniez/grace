// Usage: node names.mjs <case-dir>: prints exit code and the depth-0 scenarios that ran, with status
import fs from 'node:fs';
const dir = process.argv[2];
const ex = fs.readFileSync(dir + '/exit-code.txt', 'utf8').split('\n')[0];
let ls = [];
try { ls = fs.readFileSync(dir + '/reports/karate-json/karate-events.jsonl', 'utf8').split('\n').filter(Boolean).map(JSON.parse); } catch { }
const names = ls.filter(e => e.type === 'SCENARIO_EXIT' && e.data.callDepth === 0).map(e => `${e.data.name}${e.data.passed ? '' : '(FAIL)'}`).sort();
console.log(`${dir.split('/').pop()}: ${ex} | ${names.length} scenarios: ${names.join('; ')}`);
