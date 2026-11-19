import { expect, test } from "bun:test";
import { migrateStorage } from "../src/lib/storage-migration";

class MemoryStorage implements Storage {
  private map = new Map<string, string>();
  failWrites = false;
  get length() {
    return this.map.size;
  }
  key(i: number) {
    return [...this.map.keys()][i] ?? null;
  }
  getItem(k: string) {
    return this.map.get(k) ?? null;
  }
  setItem(k: string, v: string) {
    if (this.failWrites) throw new Error("QuotaExceededError");
    this.map.set(k, v);
  }
  removeItem(k: string) {
    this.map.delete(k);
  }
  clear() {
    this.map.clear();
  }
}

test("moves old-prefix keys to the current prefix", () => {
  const s = new MemoryStorage();
  s.setItem("pob-redux:ui", '{"scale":1.1}');
  s.setItem("pob-redux:progression:poe1:v1", "[]");
  s.setItem("unrelated", "x");
  migrateStorage(s);
  expect(s.getItem("pob-forge:ui")).toBe('{"scale":1.1}');
  expect(s.getItem("pob-forge:progression:poe1:v1")).toBe("[]");
  expect(s.getItem("pob-redux:ui")).toBeNull();
  expect(s.getItem("unrelated")).toBe("x");
});

test("never overwrites a value already saved under the current prefix", () => {
  const s = new MemoryStorage();
  s.setItem("pob-forge:ui", "new");
  s.setItem("pob-redux:ui", "old");
  migrateStorage(s);
  expect(s.getItem("pob-forge:ui")).toBe("new");
  expect(s.getItem("pob-redux:ui")).toBeNull();
});

test("keeps the old key when the copy cannot be written", () => {
  const s = new MemoryStorage();
  s.setItem("pob-redux:chat", "thread");
  s.failWrites = true;
  migrateStorage(s);
  expect(s.getItem("pob-redux:chat")).toBe("thread");
  expect(s.getItem("pob-forge:chat")).toBeNull();
});
