import { create } from 'zustand';
import dayjs, { Dayjs } from 'dayjs';
import { CalendarViewName, initialCalendarView } from '../utils/appearance';

type ViewType = CalendarViewName;

interface CalendarStore {
  viewType: ViewType;
  sessionOverride: ViewType | null;
  selectedDate: Dayjs;
  setViewType: (v: ViewType) => void;
  applyDefaultView: (defaultView: ViewType) => void;
  setSelectedDate: (d: Dayjs) => void;
  goToToday: () => void;
  goToPrev: () => void;
  goToNext: () => void;
}

export const useCalendarStore = create<CalendarStore>((set, get) => ({
  viewType: 'month',
  sessionOverride: null,
  selectedDate: dayjs(),

  setViewType: (viewType) => set({ viewType, sessionOverride: viewType }),
  applyDefaultView: (defaultView) => set((state) => ({
    viewType: initialCalendarView({
      defaultView,
      sessionOverride: state.sessionOverride,
    }),
  })),
  setSelectedDate: (selectedDate) => set({ selectedDate }),
  goToToday: () => set({ selectedDate: dayjs() }),

  goToPrev: () => {
    const { viewType, selectedDate } = get();
    set({ selectedDate: selectedDate.subtract(1, viewType) });
  },

  goToNext: () => {
    const { viewType, selectedDate } = get();
    set({ selectedDate: selectedDate.add(1, viewType) });
  },
}));
