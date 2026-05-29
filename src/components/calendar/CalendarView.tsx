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
  onSlotClick: (date: Dayjs) => void;
  onTaskClick: (task: Task) => void;
}

export function CalendarView({ viewType, selectedDate, tasks, onDateClick, onSlotClick, onTaskClick }: CalendarViewProps) {
  switch (viewType) {
    case 'month':
      return <MonthView selectedDate={selectedDate} tasks={tasks} onDateClick={onDateClick} onTaskClick={onTaskClick} />;
    case 'week':
      return <WeekView selectedDate={selectedDate} tasks={tasks} onDateClick={onDateClick} onSlotClick={onSlotClick} onTaskClick={onTaskClick} />;
    case 'day':
      return <DayView selectedDate={selectedDate} tasks={tasks} onSlotClick={onSlotClick} onTaskClick={onTaskClick} />;
  }
}
