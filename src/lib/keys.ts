/** Ctrl, without the Windows key. */
export function modKey(e: KeyboardEvent | MouseEvent): boolean {
  return e.ctrlKey && !e.metaKey;
}

const NON_TEXT_INPUTS = new Set(["checkbox", "radio", "button", "submit", "reset", "range", "color", "file", "image"]);

export function isTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || target instanceof HTMLTextAreaElement) return true;
  return target instanceof HTMLInputElement && !NON_TEXT_INPUTS.has(target.type);
}

/** The keycaps for a shortcut written "Mod+Shift+K". */
export function keycaps(shortcut: string): string[] {
  return shortcut.split("+").map((k) => {
    return k === "Mod" ? "Ctrl" : k;
  });
}
