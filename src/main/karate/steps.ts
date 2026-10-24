import type { Step } from "../../shared/events";

/** Steps of every scenario in a depth-0 FEATURE_EXIT, by Karate refId. Steps of called features are not expanded. */
export function extractSteps(
  featureExitData: Record<string, any>,
): { refId: string; steps: Step[] }[] {
  return (featureExitData.scenarioResults ?? []).map((sc: Record<string, any>) => ({
    refId: sc.refId,
    steps: (sc.stepResults ?? []).map(
      (r: Record<string, any>): Step => ({
        keyword: r.step.prefix,
        text: `${r.step.keyword} ${r.step.text}`,
        line: r.step.line,
        status: r.result.status,
        millis: r.result.millis,
        background: Boolean(r.step.background),
        log: r.stepLog ?? null,
      }),
    ),
  }));
}
