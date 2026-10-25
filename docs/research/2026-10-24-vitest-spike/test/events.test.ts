import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';

test('counts depth-0 SCENARIO_EXIT in a real Karate event stream', () => {
  const events = readFileSync(new URL('../../2026-10-24-karate-cli-spike/samples/1-normal-run-failure.jsonl', import.meta.url), 'utf8')
    .split('\n').filter(Boolean).map((l) => JSON.parse(l));
  const done = events.filter((e) => e.type === 'SCENARIO_EXIT' && e.data.callDepth === 0);
  expect(done.length).toBeGreaterThan(0);
});
