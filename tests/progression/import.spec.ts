import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as any;
    w.__clipboard = "";
    w.__clipboardDenied = false;
    w.__calls = [];
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
      invoke: async (command: string, args: any) => {
        w.__calls.push({ command, args });
        if (command === "plugin:clipboard-manager|read_text") {
          if (w.__clipboardDenied) throw new Error("Clipboard permission denied");
          return w.__clipboard;
        }
        if (command === "list_builds" || command === "list_build_folders") return [];
        if (command === "ggg_auth_available") return false;
        if (command === "build_xml_game") return "poe1";
        if (command === "fetch_build_code") return { site: "pobb.in", code: "resolved-fixture" };
        if (command === "engine_call") {
          if (args.method === "load_build_code" && args.params.code === "invalid") throw new Error("Invalid PoB code fixture");
          return { elapsed_ms: 1, result: args.method === "code_game" ? { game: "poe1" } : {} };
        }
        return null;
      },
    };
  });
  await page.goto("/tests/progression/?view=import");
});

test("Paste and Import explains an empty or denied clipboard and recovers", async ({ page }) => {
  const paste = page.getByRole("button", { name: "Paste and import", exact: true });
  await paste.click();
  await expect(page.getByRole("alert")).toContainText("clipboard has no text");
  await page.evaluate(() => (window as any).__clipboardDenied = true);
  await paste.click();
  await expect(page.getByRole("alert")).toContainText("Clipboard permission denied");
  await expect(paste).toBeEnabled();
  await page.evaluate(() => { (window as any).__clipboardDenied = false; (window as any).__clipboard = "valid-fixture"; });
  await paste.click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__calls.some((c: any) => c.args?.method === "load_build_code" && c.args.params.code === "valid-fixture"))).toBe(true);
});

test("manual Import reports an invalid code next to the controls", async ({ page }) => {
  await page.locator("textarea").fill("invalid");
  await page.getByRole("button", { name: "Import", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Invalid PoB code fixture");
  await expect(page.getByRole("button", { name: "Import", exact: true })).toBeEnabled();
});

test("Paste and Import dispatches URLs and XML without account authorization", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", e => errors.push(e.message));
  for (const text of ["https://pobb.in/test-fixture", "<PathOfBuilding><Build/></PathOfBuilding>"]) {
    await page.evaluate(text => (window as any).__clipboard = text, text);
    await page.getByRole("button", { name: "Paste and import", exact: true }).click();
    await expect(page.getByRole("button", { name: "Paste and import", exact: true })).toBeEnabled();
  }
  const methods = await page.evaluate(() => (window as any).__calls.map((c: any) => c.args?.method ?? c.command));
  expect(methods).toContain("fetch_build_code");
  expect(methods).toContain("load_build_code");
  expect(methods).toContain("load_build_xml");
  expect(methods).not.toContain("ggg_account");
  expect(methods).not.toContain("ggg_sign_in");
  expect(errors).toEqual([]);
});
