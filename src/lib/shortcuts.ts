export const shortcutActionIds = [
  'openVideo',
  'openFolder',
  'openSettings',
  'profileCompatible',
  'profileLossless',
  'profileMetadata',
] as const;

export type ShortcutActionId = (typeof shortcutActionIds)[number];
export type ShortcutSettings = Record<ShortcutActionId, string>;

export const defaultShortcuts: ShortcutSettings = {
  openVideo: 'Ctrl+KeyO',
  openFolder: 'Ctrl+Shift+KeyO',
  openSettings: 'Ctrl+Comma',
  profileCompatible: 'Alt+Digit1',
  profileLossless: 'Alt+Digit2',
  profileMetadata: 'Alt+Digit3',
};

export const reservedShortcutChords = new Set([
  'Escape',
  'F11',
  'Space',
  'PageUp',
  'PageDown',
  'Ctrl+KeyS',
  'Ctrl+Shift+KeyS',
]);

const modifierOrder = ['Ctrl', 'Alt', 'Shift'] as const;
const shortcutCodePattern = /^(?:Key[A-Z]|Digit[0-9]|Numpad[0-9]|F(?:[1-9]|1[0-2])|Comma|Period|Slash|Semicolon|Quote|BracketLeft|BracketRight|Backslash|Minus|Equal|Backquote|Space|Enter|Arrow(?:Up|Down|Left|Right)|Home|End|PageUp|PageDown|Insert|Delete)$/;

const displayCodes: Record<string, string> = {
  Comma: ',',
  Period: '.',
  Slash: '/',
  Semicolon: ';',
  Quote: "'",
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  Minus: '-',
  Equal: '=',
  Backquote: '`',
  Space: 'Space',
  Enter: 'Enter',
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
  PageUp: 'Page Up',
  PageDown: 'Page Down',
};

export function isShortcutChord(value: unknown): value is string {
  if (typeof value !== 'string') return false;
  const parts = value.split('+');
  const code = parts.pop();
  if (!code || !shortcutCodePattern.test(code)) return false;
  const modifiers = parts.filter(
    (part): part is (typeof modifierOrder)[number] => modifierOrder.includes(part as never),
  );
  if (modifiers.length !== parts.length || new Set(modifiers).size !== modifiers.length) return false;
  return [...modifierOrder.filter((modifier) => modifiers.includes(modifier)), code].join('+') === value;
}

export function shortcutChordFromEvent(
  event: Pick<KeyboardEvent, 'altKey' | 'code' | 'ctrlKey' | 'metaKey' | 'shiftKey'>,
): string | null {
  if (event.metaKey || !shortcutCodePattern.test(event.code)) return null;
  const modifiers = [
    event.ctrlKey ? 'Ctrl' : null,
    event.altKey ? 'Alt' : null,
    event.shiftKey ? 'Shift' : null,
  ].filter((modifier): modifier is string => modifier !== null);
  return [...modifiers, event.code].join('+');
}

export function shortcutMatchesEvent(
  chord: string,
  event: Pick<KeyboardEvent, 'altKey' | 'code' | 'ctrlKey' | 'metaKey' | 'shiftKey'>,
): boolean {
  return shortcutChordFromEvent(event) === chord;
}

export function formatShortcutChord(chord: string): string {
  if (!isShortcutChord(chord)) return chord;
  const parts = chord.split('+');
  const code = parts.pop()!;
  let display = displayCodes[code] ?? code;
  if (code.startsWith('Key')) display = code.slice(3);
  if (code.startsWith('Digit')) display = code.slice(5);
  if (code.startsWith('Numpad')) display = `Num ${code.slice(6)}`;
  return [...parts, display].join('+');
}

export function findShortcutConflict(
  action: ShortcutActionId,
  chord: string,
  shortcuts: ShortcutSettings,
): ShortcutActionId | null {
  return shortcutActionIds.find((candidate) => candidate !== action && shortcuts[candidate] === chord) ?? null;
}
