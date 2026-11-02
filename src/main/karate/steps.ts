import type { FileStep, Step } from "../../shared/events";

/** Steps of every scenario in a depth-0 FEATURE_EXIT, by Karate refId. Steps of called features are not expanded. */
export function extractSteps(
  featureExitData: Record<string, any>,
): { refId: string; steps: Step[] }[] {
  return (featureExitData.scenarioResults ?? []).map((sc: Record<string, any>) => ({
    refId: sc.refId,
    steps: (sc.stepResults ?? []).map((r: Record<string, any>): Step => ({
      keyword: r.step.prefix,
      text: `${r.step.keyword} ${r.step.text}`,
      line: r.step.line,
      status: r.result.status,
      millis: r.result.millis,
      background: Boolean(r.step.background),
      log: r.stepLog ?? null,
    })),
  }));
}

const STEP = /^(\*|Given|When|Then|And|But)\s/;
const HEADER = /^(@|(Feature|Rule|Scenario|Scenario Outline|Examples|Scenarios)\s*:)/;

/**
 * The background's steps, then those of the section whose header is on `sectionLine` (a scenario or an outline),
 * as written. A step is a line whose first word is a Gherkin keyword, outside a doc string (step debugger R4.2).
 */
export function fileSteps(featureText: string, sectionLine: number): FileStep[] {
  const steps: FileStep[] = [];
  let section: "background" | "test" | null = null;
  let fence: string | null = null;
  featureText.split(/\r?\n/).forEach((raw, i) => {
    const t = raw.trim();
    if (fence) {
      if (t.startsWith(fence)) fence = null;
    } else if (t.startsWith('"""') || t.startsWith("```")) fence = t.slice(0, 3);
    else if (/^Background\s*:/.test(t)) section = "background";
    else if (i + 1 === sectionLine) section = "test";
    else if (HEADER.test(t)) section = null;
    else if (section && STEP.test(t)) {
      const keyword = t.split(/\s/, 1)[0];
      const background = section === "background";
      steps.push({ keyword, text: t.slice(keyword.length).trim(), line: i + 1, background });
    }
  });
  return steps;
}
