/**
 * Settings saved by builds from before the rename live under an older key
 * prefix. Copy each one to the current prefix once, unless the current key is
 * already set, then drop the old copy. Runs before any store reads its key.
 */
export const KEY_PREFIX = "pob-forge:";
const OLD_PREFIXES = ["pob-redux:"];

export function migrateStorage(storage: Storage | null = safeStorage()) {
  if (!storage) return;
  try {
    const keys = Array.from({ length: storage.length }, (_, i) => storage.key(i)).filter((k): k is string => k !== null);
    for (const key of keys) {
      const old = OLD_PREFIXES.find((p) => key.startsWith(p));
      if (!old) continue;
      const next = KEY_PREFIX + key.slice(old.length);
      const value = storage.getItem(key);
      if (value !== null && storage.getItem(next) === null) storage.setItem(next, value);
      storage.removeItem(key);
    }
  } catch {
    // A full or blocked store keeps the old keys; the next start tries again.
  }
}

function safeStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}
