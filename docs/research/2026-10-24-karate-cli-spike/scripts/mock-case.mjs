// Starts `karate mock -m mocks/api-mock.feature -p 0`, finds the port in stdout, calls it,
// tries GET /__admin/stop, then taskkills if still alive. Saves to captures/j-mock.
import { spawn, execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const SPIKE = path.resolve(import.meta.dirname, '..');
const JAVA = path.join(SPIKE, 'dl', 'jdk-21.0.12.1+1-jre', 'bin', 'java.exe');
const JAR = path.join(SPIKE, 'dl', 'karate-2.1.4.jar');
const MOCK = process.argv[2] ?? 'mocks/api-mock.feature';
const CAP = path.join(SPIKE, 'captures', process.argv[3] ?? 'j-mock');
fs.mkdirSync(CAP, { recursive: true });
const t0 = Date.now();
const log = [];
const note = s => { log.push(`${String(Date.now() - t0).padStart(6)} ${s}`); };
const args = ['-jar', JAR, 'mock', '-m', MOCK, '-p', '0'];
note(`cmd: java ${args.join(' ')} (cwd project)`);
const env = { ...process.env }; delete env.KARATE_OPTIONS;
const child = spawn(JAVA, args, { cwd: path.join(SPIKE, 'project'), env, windowsHide: true });
let port = null, out = '', err = '', exited = null;
child.stdout.on('data', d => { out += d; for (const l of d.toString().split(/\r?\n/)) if (l) note(`OUT ${JSON.stringify(l)}`); const m = /localhost:(\d+)/.exec(out); if (m && !port) port = m[1]; });
child.stderr.on('data', d => { err += d; for (const l of d.toString().split(/\r?\n/)) if (l) note(`ERR ${JSON.stringify(l)}`); const m = /localhost:(\d+)/.exec(err); if (m && !port) port = m[1]; });
child.on('exit', (c, s) => { exited = { c, s }; note(`EXIT code=${c} signal=${s}`); });

const sleep = ms => new Promise(r => setTimeout(r, ms));
for (let i = 0; i < 100 && !port; i++) await sleep(100);
note(`port detected: ${port}`);
if (port) {
  const r = await fetch(`http://localhost:${port}/hello`); note(`GET /hello -> ${r.status} ${await r.text()}`);
  const r2 = await fetch(`http://localhost:${port}/nope`); note(`GET /nope -> ${r2.status} ${await r2.text()}`);
  try { const r3 = await fetch(`http://localhost:${port}/__admin/stop`); note(`GET /__admin/stop -> ${r3.status} ${await r3.text()}`); } catch (e) { note(`GET /__admin/stop error ${e.message}`); }
}
await sleep(3000);
if (!exited) { note('still alive after 3 s, taskkill'); try { execSync(`taskkill /PID ${child.pid} /T /F`); } catch { } }
await sleep(1000);
fs.writeFileSync(path.join(CAP, 'stdout.txt'), out);
fs.writeFileSync(path.join(CAP, 'stderr.txt'), err);
fs.writeFileSync(path.join(CAP, 'timeline.txt'), log.join('\n') + '\n');
console.log(log.join('\n'));
