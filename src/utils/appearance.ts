export type ThemeName = 'light' | 'dark';
export type CalendarViewName = 'month' | 'week' | 'day';

export interface ThemeRoot {
  setAttribute(name: string, value: string): void;
  style: { colorScheme: string };
}

export function daisyThemeName(theme: ThemeName): ThemeName {
  return theme === 'dark' ? 'dark' : 'light';
}

export function applyDocumentTheme(theme: ThemeName, root: ThemeRoot = document.documentElement): void {
  const name = daisyThemeName(theme);
  root.setAttribute('data-theme', name);
  root.style.colorScheme = name;
}

export function initialCalendarView(input: {
  defaultView: CalendarViewName;
  sessionOverride: CalendarViewName | null;
}): CalendarViewName {
  return input.sessionOverride ?? input.defaultView;
}
