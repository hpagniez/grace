import { expect, launch, test } from "./helpers";

test("localStorage survives a restart (spike)", async () => {
  const first = await launch();
  await first.win.evaluate(() => localStorage.setItem("karate-kid.spike", "1"));
  await first.app.close();

  const { win } = await launch(first.userData);
  expect(await win.evaluate(() => localStorage.getItem("karate-kid.spike"))).toBe("1");
});
