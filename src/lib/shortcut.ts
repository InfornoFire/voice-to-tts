type KeyPress = Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">;

export type ShortcutReading =
  | { kind: "incomplete" }
  | { kind: "rejected" }
  | { kind: "cancel" }
  | { kind: "clear" }
  | { kind: "accepted"; shortcut: string };

const MODIFIER_CODE = /^(Control|Shift|Alt|Meta)(Left|Right)$/;
const FUNCTION_KEY_CODE = /^F([1-9]|1\d|2[0-4])$/;

/**
 * Interprets a key press while a shortcut is being recorded. Accepted shortcuts are written the
 * way the backend parses them, such as `Ctrl+Alt+V`.
 */
export function readShortcut(press: KeyPress): ShortcutReading {
  const { code, ctrlKey, altKey, shiftKey, metaKey } = press;
  if (MODIFIER_CODE.test(code)) {
    return { kind: "incomplete" };
  }

  const hasModifier = ctrlKey || altKey || shiftKey || metaKey;
  if (!hasModifier && code === "Escape") {
    return { kind: "cancel" };
  }
  if (!hasModifier && (code === "Backspace" || code === "Delete")) {
    return { kind: "clear" };
  }

  // Shift alone would hijack normal typing, so a shortcut needs another modifier or a function key.
  const isGlobal = ctrlKey || altKey || metaKey || FUNCTION_KEY_CODE.test(code);
  if (!code || code === "Unidentified" || !isGlobal) {
    return { kind: "rejected" };
  }

  const modifiers = [
    ctrlKey && "Ctrl",
    altKey && "Alt",
    shiftKey && "Shift",
    metaKey && "Super",
  ].filter(Boolean);
  const key = code.replace(/^Key([A-Z])$/, "$1").replace(/^Digit(\d)$/, "$1");
  return { kind: "accepted", shortcut: [...modifiers, key].join("+") };
}

export function formatShortcut(shortcut: string): string {
  return shortcut === "" ? "Not set" : shortcut.split("+").join(" + ");
}
