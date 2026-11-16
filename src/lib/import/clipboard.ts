export class EmptyClipboardError extends Error {}

/** Retry a transient clipboard lock only after the user's Paste action. */
export async function readImportClipboard(
  read: () => Promise<string | null>,
  pause: () => Promise<void> = () => new Promise(resolve => setTimeout(resolve, 80)),
): Promise<string> {
  for (let attempt = 0; ; attempt++) {
    let text: string | null;
    try {
      text = await read();
    } catch (error) {
      if (attempt === 2) throw error;
      await pause();
      continue;
    }
    if (!text?.trim()) throw new EmptyClipboardError();
    return text.trim();
  }
}
