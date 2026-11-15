import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
      invoke: async (command: string, args: any) => {
        if (command !== "engine_call" || args.method !== "progression_capture") return null;
        if (args.params.code === "invalid") throw new Error("Invalid PoB fixture");
        const target = args.params.code === "target";
        return { elapsed_ms: 1, result: {
          xml: "<PathOfBuilding><Build/></PathOfBuilding>", label: args.params.label, game: "poe1", pobVersion: "UI fixture", treeVersion: "fixture",
          level: target ? 80 : 75, className: "Duelist", ascendancy: "Slayer", mainSkill: target ? "Dual Strike of Ambidexterity" : "Shield Crush",
          nodes: [{ id: target ? 2 : 1, name: target ? "Target passive" : "Current passive", mastery: null }],
          equipment: [{ slot: "Weapon 2", name: target ? "Target claw" : "Current shield", raw: target ? "Target text" : "Current text", variant: null, variant2: null }],
          gems: [], gemErrors: [], config: {}, stats: {}, loadout: { tree: "Fixture", items: "1", skills: "1" }, boss: { kind: args.params.boss, stats: {}, overrides: {} },
        } };
      },
    };
  });
});

test("capture failure is visible and leaves history and controls usable", async ({ page }) => {
  await page.goto("/tests/progression/");
  await page.getByRole("textbox", { name: "PoB share code, XML or supported build URL" }).fill("invalid");
  await page.getByRole("button", { name: "Capture pasted build" }).click();
  await expect(page.getByRole("alert")).toContainText("Invalid PoB fixture");
  await expect(page.getByRole("button", { name: "Capture pasted build" })).toBeEnabled();
  await expect(page.getByText("No snapshots yet.", { exact: false })).toBeVisible();
});

test("current and target persist, render differences and clear a removed selection", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", e => errors.push(e.message));
  await page.goto("/tests/progression/");
  for (const role of ["current", "target"] as const) {
    await page.getByLabel("Use as", { exact: true }).selectOption(role);
    await page.getByRole("textbox", { name: "Snapshot name" }).fill(`${role} fixture`);
    await page.getByRole("textbox", { name: "PoB share code, XML or supported build URL" }).fill(role);
    await page.getByRole("button", { name: "Capture pasted build" }).click();
    await expect(page.getByRole("textbox", { name: "PoB share code, XML or supported build URL" })).toHaveValue("");
  }
  await expect(page.getByRole("heading", { name: "Bossing profile", exact: true })).toBeVisible();
  await expect(page.getByText("Prepare Weapon 2: Target claw", { exact: true })).toBeVisible();
  await expect(page.getByText("Current passive", { exact: false }).first()).toBeVisible();
  await expect(page.getByText("Target passive", { exact: false }).first()).toBeVisible();
  await expect(page.locator("table tbody tr").first().locator("td").first()).toHaveText("—");
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
  await page.reload();
  await expect(page.getByRole("button", { name: "Remove", exact: true })).toHaveCount(2);
  await page.getByRole("button", { name: "Remove", exact: true }).first().click();
  await expect(page.getByRole("heading", { name: "Bossing profile", exact: true })).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole("button", { name: "Remove", exact: true })).toHaveCount(1);
  expect(errors).toEqual([]);
});

test("corrupt history is retained and cannot be silently overwritten", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("pob-redux:progression:poe1:v1", "broken-history"));
  await page.goto("/tests/progression/");
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page.getByRole("button", { name: "Capture open build" })).toBeDisabled();
  expect(await page.evaluate(() => localStorage.getItem("pob-redux:progression:poe1:v1"))).toBe("broken-history");
});
