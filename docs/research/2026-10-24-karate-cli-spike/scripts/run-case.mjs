// Usage: node run-case.mjs <case> [--env K=V] [--kill taskkill|childkill --after-event SCENARIO_ENTER:slow loop|--after-ms N] -- <karate args...>
// Runs java -jar karate.jar from SPIKE/project, captures stdout/stderr/exit code,
// tails <out>/karate-json/karate-events.jsonl and timestamps each line arrival.
// The token {OUT} in karate args is replaced with the absolute output dir.
import { spawn, execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const SPIKE = path.resolve(import.meta.dirname, '..');
const JAVA = path.join(SPIKE, 'dl', 'jdk-21.0.12.1+1-jre', 'bin', 'java.exe');
const JAR = path.join(SPIKE, 'dl', 'karate-2.1.4.jar');
let PROJECT = path.join(SPIKE, 'project');

const argv = process.argv.slice(2);
const caseName = argv.shift();
const sep = argv.indexOf('--');
const opts = argv.slice(0, sep);
const kargs = argv.slice(sep + 1);
const envOverrides = {};
let kill = null, afterEvent = null, afterMs = null, shell = false; let cwdOverride = null;
for (let i = 0; i < opts.length; i++) {
  if (opts[i] === '--env') { const [k, ...v] = opts[++i].split('='); envOverrides[k] = v.join('='); }
  else if (opts[i] === '--kill') kill = opts[++i];
  else if (opts[i] === '--after-event') afterEvent = opts[++i];
  else if (opts[i] === '--after-ms') afterMs = Number(opts[++i]);
  else if (opts[i] === '--cwd') cwdOverride = opts[++i];
  else if (opts[i] === '--shell') shell = true; // spawn through cmd.exe, like a .cmd launcher would
}

if (cwdOverride) PROJECT = cwdOverride;
const CAP = path.join(SPIKE, 'captures', caseName);
fs.mkdirSync(CAP, { recursive: true });
const OUT = path.join(CAP, 'reports');
const args = ['-jar', JAR, ...kargs.map(a => a.replaceAll('{OUT}', OUT))];
const env = { ...process.env };
delete env.KARATE_OPTIONS;
Object.assign(env, envOverrides);

const t0 = Date.now();
const ts = () => String(Date.now() - t0).padStart(6, ' ');
fs.writeFileSync(path.join(CAP, 'command.txt'),
  `cwd: ${PROJECT}\nenv overrides: ${JSON.stringify(envOverrides)}\n"${JAVA}" ${args.map(a => JSON.stringify(a)).join(' ')}\n`);

const child = shell
  ? spawn([JAVA, ...args].map(a => `"${a}"`).join(' '), { cwd: PROJECT, env, windowsHide: true, shell: true })
  : spawn(JAVA, args, { cwd: PROJECT, env, windowsHide: true });
const stdout = fs.createWriteStream(path.join(CAP, 'stdout.txt'));
const stderr = fs.createWriteStream(path.join(CAP, 'stderr.txt'));
const timeline = fs.createWriteStream(path.join(CAP, 'timeline.txt'));
child.stdout.on('data', d => { stdout.write(d); for (const l of d.toString().split(/\r?\n/)) if (l) timeline.write(`${ts()} OUT ${l.slice(0, 160)}\n`); });
child.stderr.on('data', d => { stderr.write(d); for (const l of d.toString().split(/\r?\n/)) if (l) timeline.write(`${ts()} ERR ${l.slice(0, 160)}\n`); });

// Tail the JSONL by polling its size.
const jsonl = path.join(OUT, 'karate-json', 'karate-events.jsonl');
let pos = 0, partial = '', killed = false;
function doKill() {
  if (killed) return; killed = true;
  timeline.write(`${ts()} KILL via ${kill} pid=${child.pid}\n`);
  if (kill === 'taskkill') { try { timeline.write(execSync(`taskkill /PID ${child.pid} /T /F`).toString()); } catch (e) { timeline.write(`taskkill error ${e.message}\n`); } }
  else child.kill();
}
const tail = () => {
  let st; try { st = fs.statSync(jsonl); } catch { return; }
  if (st.size < pos) pos = 0;
  if (st.size === pos) return;
  const fd = fs.openSync(jsonl, 'r'); const buf = Buffer.alloc(st.size - pos);
  fs.readSync(fd, buf, 0, buf.length, pos); fs.closeSync(fd); pos = st.size;
  const parts = (partial + buf.toString()).split('\n'); partial = parts.pop();
  for (const l of parts) {
    if (!l.trim()) continue;
    let e; try { e = JSON.parse(l); } catch { timeline.write(`${ts()} JSONL <unparsable ${l.length}b>\n`); continue; }
    const d = e.data || {};
    timeline.write(`${ts()} JSONL ${e.type} depth=${d.callDepth ?? ''} ${d.name ?? d.path ?? ''}\n`);
    if (afterEvent) { const [t, n] = afterEvent.split(':'); if (e.type === t && (!n || d.name === n)) setTimeout(doKill, 1500); }
  }
};
const poll = setInterval(tail, 25);
if (afterMs) setTimeout(doKill, afterMs);

child.on('exit', (code, signal) => {
  setTimeout(() => {
    tail(); clearInterval(poll);
    if (partial) timeline.write(`${ts()} JSONL trailing partial line (${partial.length} bytes, no newline)\n`);
    const res = `exit code: ${code}\nsignal: ${signal}\nelapsed ms: ${Date.now() - t0}\n`;
    fs.writeFileSync(path.join(CAP, 'exit-code.txt'), res);
    timeline.end(); stdout.end(); stderr.end();
    console.log(`[${caseName}] ${res.replaceAll('\n', ' ')}`);
  }, 500);
});
