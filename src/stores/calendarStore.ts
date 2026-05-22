import { create } from 'zustand';
import dayjs, { Dayjs } from 'dayjs';

type ViewType = 'month' | 'week' | 'day';

interface CalendarStore {
  viewType: ViewType;
  selectedDate: Dayjs;
  setViewType: (v: ViewType) => void;
  setSelectedDate: (d: Dayjs) => void;
  goToToday: () => void;
  goToPrev: () => void;
  goToNext: () => void;
}

export const useCalendarStore = create<CalendarStore>((set, get) => ({
  viewType: 'month',
  selectedDate: dayjs(),

  setViewType: (viewType) => set({ viewType }),
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
