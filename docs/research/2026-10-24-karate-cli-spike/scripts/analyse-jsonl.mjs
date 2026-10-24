// Usage: node analyse-jsonl.mjs <karate-events.jsonl>
// Prints every event type, count, envelope keys and data keys (recursive key paths, depth-limited).
import fs from 'node:fs';

const lines = fs.readFileSync(process.argv[2], 'utf8').split('\n').filter(l => l.trim());
const types = new Map();
function keys(o, prefix, depth, out) {
  if (o === null || typeof o !== 'object' || depth > 3) return;
  if (Array.isArray(o)) { if (o.length) keys(o[0], prefix + '[]', depth + 1, out); return; }
  for (const [k, v] of Object.entries(o)) {
    const p = prefix ? `${prefix}.${k}` : k;
    out.add(`${p}:${Array.isArray(v) ? 'array' : v === null ? 'null' : typeof v}`);
    keys(v, p, depth + 1, out);
  }
}
for (const l of lines) {
  const e = JSON.parse(l);
  const t = types.get(e.type) ?? { count: 0, env: new Set(), data: new Set() };
  t.count++;
  Object.keys(e).forEach(k => t.env.add(k));
  keys(e.data, '', 0, t.data);
  types.set(e.type, t);
}
for (const [type, t] of types) {
  console.log(`\n## ${type} x${t.count}\nenvelope: ${[...t.env].join(', ')}`);
  console.log('data:\n  ' + [...t.data].sort().join('\n  '));
}
