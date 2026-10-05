import { onMounted, onUnmounted } from 'vue';

/**
 * Every app shortcut, in one place: pages register handlers by id with
 * {@link useShortcuts}, and the Ctrl+/ help lists this catalog, so the help can
 * never drift from what is bound. "Ctrl" also matches ⌘ on macOS.
 */
const IS_MAC = typeof navigator !== 'undefined' && /mac/i.test(navigator.platform);

export const SHORTCUTS = {
  toggleSidebar: { keys: 'Ctrl+\\', description: 'Show or hide the sidebar', scope: 'Anywhere' },
  newNote: { keys: 'Ctrl+N', description: 'New note', scope: 'Anywhere' },
  quickOpen: { keys: 'Ctrl+P', description: 'Find a note', scope: 'Anywhere' },
  settings: { keys: 'Ctrl+,', description: 'Settings', scope: 'Anywhere' },
  dashboard: { keys: 'Ctrl+Shift+D', description: 'Dashboard', scope: 'Anywhere' },
  syncNow: { keys: 'Ctrl+Shift+S', description: 'Sync now', scope: 'Anywhere' },
  help: { keys: 'Ctrl+/', description: 'Show keyboard shortcuts', scope: 'Anywhere' },
  focusSearch: { keys: 'Ctrl+Shift+F', description: 'Search notes', scope: 'Notes list' },
  save: { keys: 'Ctrl+S', description: 'Save now', scope: 'Editor' },
  rename: { keys: 'F2', description: 'Rename note', scope: 'Editor' },
  tags: { keys: 'Ctrl+T', description: 'Manage tags', scope: 'Editor' },
  toggleEncryption: {
    keys: 'Ctrl+Shift+E',
    description: 'Turn note encryption on or off',
    scope: 'Editor',
  },
  moveToTrash: {
    keys: 'Ctrl+Shift+Delete',
    description: 'Move note to trash (press twice)',
    scope: 'Editor',
  },
  // On macOS Option+← moves the caret one word left, so use ⌘[ there.
  back: { keys: IS_MAC ? 'Ctrl+[' : 'Alt+←', description: 'Back', scope: 'Editor' },
} as const;

export type ShortcutId = keyof typeof SHORTCUTS;

/** Milkdown's own formatting keys, listed in the help only. */
export const EDITOR_FORMATTING = [
  { keys: 'Ctrl+B', description: 'Bold' },
  { keys: 'Ctrl+I', description: 'Italic' },
  { keys: 'Ctrl+E', description: 'Inline code' },
  { keys: 'Ctrl+Alt+X', description: 'Strikethrough' },
  { keys: 'Ctrl+Alt+1…6', description: 'Heading 1–6' },
  { keys: 'Ctrl+Alt+0', description: 'Normal text' },
  { keys: 'Ctrl+Alt+8', description: 'Bullet list' },
  { keys: 'Ctrl+Alt+7', description: 'Numbered list' },
  { keys: 'Ctrl+Shift+B', description: 'Quote' },
  { keys: 'Ctrl+Alt+C', description: 'Code block' },
  { keys: 'Ctrl+Z', description: 'Undo' },
  { keys: 'Ctrl+Shift+Z', description: 'Redo' },
] as const;

const KEY_ALIASES: Record<string, string> = { '←': 'arrowleft' };

/** Whether `event` is exactly the combination written in `keys` (e.g. "Ctrl+Shift+S"). */
export function matchesShortcut(event: KeyboardEvent, keys: string): boolean {
  const parts = keys.split('+');
  // "Ctrl++" style is not used, so the last part is always the key itself.
  const key = parts.pop()!.toLowerCase();
  const wanted = new Set(parts.map((part) => part.toLowerCase()));

  return (
    (event.ctrlKey || event.metaKey) === wanted.has('ctrl') &&
    event.shiftKey === wanted.has('shift') &&
    event.altKey === wanted.has('alt') &&
    event.key.toLowerCase() === (KEY_ALIASES[key] ?? key)
  );
}

/**
 * Binds shortcut handlers while the calling component is mounted. Keys the
 * focused element already handled (`defaultPrevented`, e.g. the editor's own
 * formatting keys) are left alone.
 */
export function useShortcuts(
  handlers: Partial<Record<ShortcutId, (event: KeyboardEvent) => void>>
) {
  function onKeyDown(event: KeyboardEvent) {
    if (event.defaultPrevented) return;

    for (const [id, handler] of Object.entries(handlers)) {
      if (handler && matchesShortcut(event, SHORTCUTS[id as ShortcutId].keys)) {
        event.preventDefault();
        handler(event);
        return;
      }
    }
  }

  onMounted(() => window.addEventListener('keydown', onKeyDown));
  onUnmounted(() => window.removeEventListener('keydown', onKeyDown));
}
