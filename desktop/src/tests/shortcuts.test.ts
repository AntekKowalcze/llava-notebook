import { describe, expect, it } from 'vitest';
import { matchesShortcut, SHORTCUTS } from '../lib/shortcuts';

function key(init: Partial<KeyboardEvent> & { key: string }): KeyboardEvent {
  return {
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ...init,
  } as KeyboardEvent;
}

describe('matchesShortcut', () => {
  it('needs the exact modifiers', () => {
    expect(matchesShortcut(key({ key: 'S', ctrlKey: true, shiftKey: true }), 'Ctrl+Shift+S')).toBe(
      true
    );
    expect(matchesShortcut(key({ key: 's', ctrlKey: true }), 'Ctrl+Shift+S')).toBe(false);
    expect(
      matchesShortcut(
        key({ key: 's', ctrlKey: true, shiftKey: true, altKey: true }),
        'Ctrl+Shift+S'
      )
    ).toBe(false);
  });

  it('treats Cmd as Ctrl', () => {
    expect(matchesShortcut(key({ key: 'n', metaKey: true }), 'Ctrl+N')).toBe(true);
  });

  it('matches punctuation, function and named keys', () => {
    expect(matchesShortcut(key({ key: '\\', ctrlKey: true }), SHORTCUTS.toggleSidebar.keys)).toBe(
      true
    );
    expect(matchesShortcut(key({ key: ',', ctrlKey: true }), SHORTCUTS.settings.keys)).toBe(true);
    expect(matchesShortcut(key({ key: 'F2' }), SHORTCUTS.rename.keys)).toBe(true);
    expect(matchesShortcut(key({ key: 'F2', ctrlKey: true }), SHORTCUTS.rename.keys)).toBe(false);
    expect(matchesShortcut(key({ key: 'ArrowLeft', altKey: true }), SHORTCUTS.back.keys)).toBe(
      true
    );
    expect(
      matchesShortcut(
        key({ key: 'Delete', ctrlKey: true, shiftKey: true }),
        SHORTCUTS.moveToTrash.keys
      )
    ).toBe(true);
  });

  it('no longer binds Ctrl+B, which the editor uses for bold', () => {
    const ctrlB = key({ key: 'b', ctrlKey: true });
    expect(Object.values(SHORTCUTS).some((s) => matchesShortcut(ctrlB, s.keys))).toBe(false);
  });

  it('has no two shortcuts on the same keys', () => {
    const keys = Object.values(SHORTCUTS).map((s) => s.keys);
    expect(new Set(keys).size).toBe(keys.length);
  });
});
