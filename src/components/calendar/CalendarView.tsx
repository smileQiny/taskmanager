import { Dayjs } from 'dayjs';
import { Task } from '../../types/task';
import { MonthView } from './MonthView';
import { WeekView } from './WeekView';
import { DayView } from './DayView';

interface CalendarViewProps {
  viewType: 'month' | 'week' | 'day';
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
}

export function CalendarView({ viewType, selectedDate, tasks, onDateClick }: CalendarViewProps) {
  switch (viewType) {
    case 'month':
      return <MonthView selectedDate={selectedDate} tasks={tasks} onDateClick={onDateClick} />;
    case 'week':
      return <WeekView selectedDate={selectedDate} tasks={tasks} onDateClick={onDateClick} />;
    case 'day':
      return <DayView selectedDate={selectedDate} tasks={tasks} />;
  }
}
