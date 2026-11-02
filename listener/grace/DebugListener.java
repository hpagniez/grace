package grace;

import io.karatelabs.common.Json;
import io.karatelabs.common.Xml;
import io.karatelabs.core.RunEvent;
import io.karatelabs.core.RunEventType;
import io.karatelabs.core.RunListener;
import io.karatelabs.core.ScenarioRuntime;
import io.karatelabs.core.StepRunEvent;
import io.karatelabs.gherkin.Step;
import io.karatelabs.js.JsFunction;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.Writer;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.LinkedBlockingQueue;
import org.w3c.dom.Node;

/**
 * Grace's step debugger (docs/specs/2026-11-02-step-debugger.md, R1). Pauses before a step on a breakpoint, a
 * pause asked for, or a step command, sends the step and the scenario's variables to the app, and waits for its
 * command. Any error stops the pausing for the rest of the run: the run itself goes on.
 */
public class DebugListener implements RunListener {
    static final int VALUE_LIMIT = 65_536;

    private final BufferedReader in;
    private final Writer out;
    private final LinkedBlockingQueue<String> commands = new LinkedBlockingQueue<>();
    private volatile Set<String> breakpoints;
    private volatile boolean pauseAsked;
    private volatile boolean dead;
    /** The last step command: run (to a breakpoint), over or into. Read and written on the run's one thread. */
    private String mode = "run";
    private int overDepth;

    public DebugListener() throws IOException {
        Socket socket = new Socket("127.0.0.1", Integer.parseInt(System.getProperty("grace.debug.port")));
        in = new BufferedReader(new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8));
        out = new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8);
        send(System.getProperty("grace.debug.token"));
        Map<String, Object> first = read(in.readLine());
        breakpoints = toSet(first.get("breakpoints"));
        pauseAsked = Boolean.TRUE.equals(first.get("pause"));
        Thread reader = new Thread(this::readCommands, "grace-debug");
        reader.setDaemon(true);
        reader.start();
    }

    @Override
    public boolean onEvent(RunEvent event) {
        try {
            if (!dead && event instanceof StepRunEvent e && e.type() == RunEventType.STEP_ENTER) {
                pauseIfAsked(e.step(), e.scenarioRuntime());
            }
        } catch (Throwable t) {
            dead = true;
        }
        return true;
    }

    private void pauseIfAsked(Step step, ScenarioRuntime sr) throws IOException, InterruptedException {
        String file = step.getFeature().getResource().getRelativePath();
        int depth = sr.getFeatureRuntime().getCallDepth();
        boolean pause = pauseAsked
                || breakpoints.contains(file + ":" + step.getLine())
                || mode.equals("into")
                || (mode.equals("over") && depth <= overDepth);
        if (!pause) {
            return;
        }
        pauseAsked = false;
        ScenarioRuntime test = sr;
        while (test.getFeatureRuntime().getCallDepth() > 0 && test.getFeatureRuntime().getCallerScenario() != null) {
            test = test.getFeatureRuntime().getCallerScenario();
        }
        Map<String, Object> testId = new LinkedHashMap<>();
        testId.put("file", test.getScenario().getFeature().getResource().getRelativePath());
        testId.put("refId", test.getScenario().getRefId());
        List<Object> vars = new ArrayList<>();
        sr.getAllVariables().forEach((k, v) -> vars.add(variable(k, v)));
        Map<String, Object> msg = new LinkedHashMap<>();
        msg.put("file", file);
        msg.put("line", step.getLine());
        msg.put("depth", depth);
        msg.put("prefix", step.getPrefix());
        msg.put("keyword", step.getKeyword());
        msg.put("text", step.getText());
        msg.put("test", testId);
        msg.put("vars", vars);
        send(Json.stringifyStrict(msg));
        mode = commands.take();
        overDepth = depth;
    }

    private static Map<String, Object> variable(String name, Object v) {
        String kind;
        String value;
        if (v == null) {
            kind = "null";
            value = "null";
        } else if (v instanceof JsFunction) {
            kind = "function";
            value = "function";
        } else if (v instanceof Map || v instanceof List) {
            kind = "json";
            value = Json.stringifyStrict(v);
        } else if (v instanceof Node n) {
            kind = "xml";
            value = Xml.toString(n);
        } else if (v instanceof String s) {
            kind = "string";
            value = s;
        } else if (v instanceof Number) {
            kind = "number";
            value = v.toString();
        } else if (v instanceof Boolean) {
            kind = "boolean";
            value = v.toString();
        } else {
            kind = "other";
            value = String.valueOf(v);
        }
        Map<String, Object> m = new LinkedHashMap<>();
        m.put("name", name);
        m.put("kind", kind);
        if (kind.equals("other")) {
            m.put("type", v.getClass().getSimpleName());
        }
        if (value.length() > VALUE_LIMIT) {
            m.put("cut", value.length() - VALUE_LIMIT);
            value = value.substring(0, VALUE_LIMIT);
        }
        m.put("value", value);
        return m;
    }

    private void readCommands() {
        try {
            for (String line; (line = in.readLine()) != null; ) {
                Map<String, Object> msg = read(line);
                if (msg.containsKey("breakpoints")) {
                    breakpoints = toSet(msg.get("breakpoints"));
                } else if ("pause".equals(msg.get("command"))) {
                    pauseAsked = true;
                } else if (msg.get("command") instanceof String command) {
                    commands.offer(command);
                }
            }
        } catch (Throwable t) {
            // the app is gone or sent something unreadable: same as the end of the stream
        }
        dead = true;
        commands.offer("run"); // releases a paused step
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> read(String line) {
        return (Map<String, Object>) Json.parseStrict(line);
    }

    @SuppressWarnings("unchecked")
    private static Set<String> toSet(Object list) {
        Set<String> set = new HashSet<>();
        for (Object o : (List<Object>) list) {
            Map<String, Object> b = (Map<String, Object>) o;
            set.add(b.get("file") + ":" + ((Number) b.get("line")).intValue());
        }
        return set;
    }

    private synchronized void send(String line) throws IOException {
        out.write(line + "\n");
        out.flush();
    }
}
