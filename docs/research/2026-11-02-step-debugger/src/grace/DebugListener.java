package grace;

import io.karatelabs.common.Json;
import io.karatelabs.common.Xml;
import io.karatelabs.core.RunEvent;
import io.karatelabs.core.RunEventType;
import io.karatelabs.core.RunListener;
import io.karatelabs.core.ScenarioRuntime;
import io.karatelabs.core.StepRunEvent;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.UncheckedIOException;
import java.io.Writer;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.w3c.dom.Node;

/** Spike: pause before every step, send the step and its scenario's variables, wait for a line back. */
public class DebugListener implements RunListener {
    private final BufferedReader in;
    private final Writer out;

    public DebugListener() throws IOException {
        Socket socket = new Socket("127.0.0.1", Integer.getInteger("grace.debug.port"));
        in = new BufferedReader(new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8));
        out = new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8);
    }

    @Override
    public synchronized boolean onEvent(RunEvent event) {
        if (!(event instanceof StepRunEvent e) || e.type() != RunEventType.STEP_ENTER) {
            return true;
        }
        ScenarioRuntime sr = e.scenarioRuntime();
        Map<String, Object> msg = new LinkedHashMap<>();
        msg.put("thread", Thread.currentThread().getName());
        msg.put("feature", e.step().getFeature().getResource().getRelativePath());
        msg.put("line", e.step().getLine());
        msg.put("depth", sr.getFeatureRuntime().getCallDepth());
        msg.put("text", e.step().getPrefix() + " " + e.step().getText());
        Map<String, Object> vars = new LinkedHashMap<>();
        sr.getAllVariables().forEach((k, v) -> vars.put(k, show(v)));
        msg.put("vars", vars);
        try {
            out.write(Json.stringifyStrict(msg) + "\n");
            out.flush();
            return !"skip".equals(in.readLine());
        } catch (IOException ex) {
            throw new UncheckedIOException(ex);
        }
    }

    private static Map<String, Object> show(Object v) {
        Map<String, Object> m = new LinkedHashMap<>();
        m.put("type", v == null ? "null" : v.getClass().getName());
        m.put("value", v instanceof Map || v instanceof List ? Json.stringifyStrict(v)
                : v instanceof Node n ? Xml.toString(n) : String.valueOf(v));
        return m;
    }
}
