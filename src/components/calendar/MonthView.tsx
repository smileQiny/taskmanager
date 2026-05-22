import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface MonthViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
}

export function MonthView({ selectedDate, tasks, onDateClick }: MonthViewProps) {
  const startOfMonth = selectedDate.startOf('month');
  const startDay = startOfMonth.startOf('week');
  const weeks: Dayjs[][] = [];

  let current = startDay;
  for (let w = 0; w < 6; w++) {
    const week: Dayjs[] = [];
    for (let d = 0; d < 7; d++) {
      week.push(current);
      current = current.add(1, 'day');
    }
    weeks.push(week);
  }

  const getTasksForDay = (date: Dayjs) => {
    const dayStart = date.startOf('day').unix();
    const dayEnd = date.endOf('day').unix();
    return tasks.filter((t) => {
      if (!t.start_time) return false;
      return t.start_time >= dayStart && t.start_time <= dayEnd;
    });
  };

  const isToday = (date: Dayjs) => date.isSame(dayjs(), 'day');
  const isCurrentMonth = (date: Dayjs) => date.month() === selectedDate.month();

  return (
    <div className="grid grid-cols-7 gap-px bg-base-300 rounded-lg overflow-hidden h-full">
      {['日', '一', '二', '三', '四', '五', '六'].map((d) => (
        <div key={d} className="bg-base-200 text-center text-xs font-medium py-2">{d}</div>
      ))}
      {weeks.flat().map((date, i) => {
        const dayTasks = getTasksForDay(date);
        return (
          <div
            key={i}
            onClick={() => onDateClick(date)}
            className={`bg-base-100 min-h-[80px] p-1 cursor-pointer hover:bg-base-200 transition
              ${!isCurrentMonth(date) ? 'opacity-40' : ''}`}
          >
            <span className={`text-xs inline-block w-6 h-6 text-center leading-6 rounded-full
              ${isToday(date) ? 'bg-primary text-primary-content' : ''}`}>
              {date.date()}
            </span>
            <div className="mt-1 space-y-0.5">
              {dayTasks.slice(0, 3).map((t) => (
                <div key={t.id} className="text-[10px] truncate px-1 rounded bg-primary/10 text-primary">
                  {t.title}
                </div>
              ))}
              {dayTasks.length > 3 && (
                <div className="text-[10px] text-base-content/50">+{dayTasks.length - 3} 更多</div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
