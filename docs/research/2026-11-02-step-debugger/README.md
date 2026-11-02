# Step debugger spike: a Karate `RunListener` that pauses steps

Checked on 2026-11-02 on the reference desktop with the bundled Temurin 21.0.12.1 and Karate 2.1.4, for [#65](https://github.com/hpagniez/grace/issues/65). Source read from Maven Central's `karate-core-2.1.4-sources.jar` and `karate-js-2.1.4-sources.jar` (MIT).

**Bottom line:** open-source Karate 2.1.4 gives a `--listener` class everything a step debugger needs. It sees every step before the step runs, holds it for as long as `onEvent` does not return, and reads the scenario's variables as Java values. The bundled JRE cannot compile Java, but the Eclipse compiler (ECJ, one 3.4 MB jar) runs on it and compiles the listener in under a second.

## Karate's listener interface

- `io.karatelabs.core.RunListener`: one method, `boolean onEvent(RunEvent event)`. Returning `false` from a `*_ENTER` event skips it; on `STEP_ENTER` the step is recorded as skipped.
- `--listener <class>` (`RunCommand.java`) goes to `Runner.Builder.listenerFactory(String)`, which builds the class with its no-argument constructor. A `RunListener` becomes one global instance shared by all threads; a `RunListenerFactory` (`--listener-factory`) gets one listener per thread. Debugging runs one thread (`-T 1`), so one global listener is enough: every event arrives on `main`.
- `Suite.fireEvent` calls the global listeners first, then the JSONL writer, then per-thread listeners. It calls every listener even when one throws, then rethrows the first exception. An exception in our listener would fail the step, so the listener must catch its own errors (a lost socket means: stop pausing, return `true`).
- `StepExecutor.execute` fires `StepRunEvent.enter(step, runtime)` before it runs the step. The record holds `step()` (`io.karatelabs.gherkin.Step`: `getLine()`, `getPrefix()`, `getKeyword()`, `getText()`, `getFeature().getResource().getRelativePath()`) and `scenarioRuntime()`.
- `ScenarioRuntime.getAllVariables()` copies the JS engine's visible bindings, unwrapped: strings and numbers as Java values, JSON as `Map` and `List` (Karate's `JsObject` and `JsArray` implement them), XML as a DOM `Node`, functions as `JsFunctionWrapper`. Hidden bindings (`responseBytes` and the like) are left out. `io.karatelabs.common.Json.stringifyStrict` and `Xml.toString` turn them into text.
- `scenarioRuntime().getFeatureRuntime().getCallDepth()` is 0 for the scenario's own steps and 1 or more inside a called feature: enough for step over (next step at the same or a lower depth) and step into (the next step at any depth).
- Karate also has its own debug hooks (`RunInterceptor` and `DebugPointFactory` in karate-js, and `karate.debug.port`, which hands the run to `io.karatelabs.debug.Main` from the paid karate-ide jar). We do not use them: the listener path is all open source and documented as a public interface.

## The spike

`src/grace/DebugListener.java` connects to `127.0.0.1:<grace.debug.port>` in its constructor. On every `STEP_ENTER` it sends one JSON line (feature, line, call depth, step text, variables) and waits for a line back. `driver.mjs` opens the socket, starts Karate as the app does plus `-T 1 --listener grace.DebugListener` with the class folder on the classpath, and answers each pause after a delay. `project/` has one feature with a variable of each kind and a called feature.

```bash
java -jar ecj.jar -21 -nowarn -cp vendor/karate.jar -d <classes> docs/research/2026-11-02-step-debugger/src/grace/DebugListener.java
node docs/research/2026-11-02-step-debugger/driver.mjs <repo> <classes> <copy of project> 1500
```

Results:

| Check | Result |
|---|---|
| Compile with ECJ 3.46.100 on the bundled JRE | 0.9 s, one 4.9 KB class |
| Step events | 13 `STEP_ENTER`, one per step, in order, on thread `main`; the background step again before the second scenario |
| Blocking | Each pause held 1.5 s: the 2 scenarios took 19.7 s and passed |
| Variables before line 12 | `name` `"grace"`, `count` `42` (Integer), `user` `{"id":7,"tags":["a","b"]}`, `list` `[1,2,3]`, `doc` `<root><item>x</item></root>`, `base` from the background |
| Function variable | `JsFunctionWrapper`, stringified as `{}`: show it as a function, not as JSON |
| Called feature | Its step arrives with `feature: called.feature`, depth 1, and the caller's variables plus `arg` |
| Feature path | `getRelativePath()` is `debug.feature`, the same as the JSONL `FEATURE_ENTER.path`, so breakpoints can use the app's node keys |
| JSONL while paused | The JSONL stream has no step events; `SCENARIO_ENTER` is written before the first step pauses, so the tree shows the scenario as running |
| Java exit | The socket closes with `ECONNRESET` on the Node side: handle `error`, not only `close` |

## Toolchain

The bundled Temurin JRE has no `jdk.compiler` module (`java --list-modules`), so neither `javac` nor `java Foo.java` works, and the machine has no JDK. Options:

| Option | Build download | Notes |
|---|---|---|
| ECJ (`org.eclipse.jdt:ecj:3.46.100`, EPL-2.0) on the bundled JRE | 3.4 MB jar, SHA-1 matches Maven Central | Compiles against `karate.jar` in 0.9 s. Build tool only, never shipped |
| Temurin 21 JDK | ~190 MB zip | Same compile, 55 times the download, on every CI cache miss |
| Commit the compiled `.class` | none | A binary nobody can review, rebuilt by hand on every Karate bump |

Shipped cost of any option: the class files, a few KB, in `resources/`. A normal run does not load them: only a debug run adds the folder to the classpath and passes `--listener`.
