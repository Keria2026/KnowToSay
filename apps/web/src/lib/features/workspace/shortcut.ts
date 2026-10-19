export function shortcutFromKey(
  event: Pick<
    KeyboardEvent,
    'key' | 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'
  >,
): string | null {
  if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return null;
  const key =
    event.code === 'Space'
      ? 'Space'
      : /^Key[A-Z]$/.test(event.code)
        ? event.code.slice(3)
        : /^Digit[0-9]$/.test(event.code)
          ? event.code.slice(5)
          : /^(F[1-9]|F1[0-9]|F2[0-4])$/.test(event.key)
            ? event.key
            : [
                  'Enter',
                  'Tab',
                  'Home',
                  'End',
                  'PageUp',
                  'PageDown',
                  'ArrowUp',
                  'ArrowDown',
                  'ArrowLeft',
                  'ArrowRight',
                ].includes(event.key)
              ? event.key
              : null;
  if (!key || !(event.ctrlKey || event.altKey || event.metaKey)) return null;
  return [
    event.ctrlKey ? 'Ctrl' : '',
    event.altKey ? 'Alt' : '',
    event.shiftKey ? 'Shift' : '',
    event.metaKey ? 'Super' : '',
    key,
  ]
    .filter(Boolean)
    .join('+');
}
