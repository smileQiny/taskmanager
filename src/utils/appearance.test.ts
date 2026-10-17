import { describe, expect, it } from 'vitest';
import { applyDocumentTheme, daisyThemeName, initialCalendarView } from './appearance';

describe('appearance theme', () => {
  it('maps stored light and dark settings onto DaisyUI theme names', () => {
    expect(daisyThemeName('light')).toBe('light');
    expect(daisyThemeName('dark')).toBe('dark');
  });

  it('applies the DaisyUI theme on the document root instead of a hardcoded theme', () => {
    const attributes = new Map<string, string>();
    const root = {
      setAttribute(name: string, value: string) {
        attributes.set(name, value);
      },
      style: { colorScheme: '' },
    };

    applyDocumentTheme('dark', root);

    expect(attributes.get('data-theme')).toBe('dark');
    expect(root.style.colorScheme).toBe('dark');
  });
});

describe('default calendar view', () => {
  it('uses the saved default until the user picks a view in this session', () => {
    expect(initialCalendarView({ defaultView: 'week', sessionOverride: null })).toBe('week');
    expect(initialCalendarView({ defaultView: 'week', sessionOverride: 'day' })).toBe('day');
  });
});
