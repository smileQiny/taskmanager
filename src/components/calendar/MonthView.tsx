import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface MonthViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
  onTaskClick: (task: Task) => void;
}

export function MonthView({ selectedDate, tasks, onDateClick, onTaskClick }: MonthViewProps) {
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
    <div className="grid h-full grid-cols-7 gap-px overflow-hidden rounded-md border border-slate-200 bg-slate-200">
      {['日', '一', '二', '三', '四', '五', '六'].map((d) => (
        <div key={d} className="bg-slate-50 py-2 text-center text-xs font-medium text-slate-500">{d}</div>
      ))}
      {weeks.flat().map((date, i) => {
        const dayTasks = getTasksForDay(date);
        return (
          <div
            key={i}
            onClick={() => onDateClick(date)}
            className={`min-h-[96px] cursor-pointer bg-white p-2 transition hover:bg-teal-50/60
              ${!isCurrentMonth(date) ? 'opacity-40' : ''}`}
          >
            <span className={`inline-block h-6 w-6 rounded-full text-center text-xs leading-6
              ${isToday(date) ? 'bg-slate-950 text-white' : 'text-slate-600'}`}>
              {date.date()}
            </span>
            <div className="mt-1 space-y-0.5">
              {dayTasks.slice(0, 3).map((t) => (
                <button
                  key={t.id}
                  className="block w-full truncate rounded bg-teal-50 px-1 py-0.5 text-left text-[10px] text-teal-700"
                  onClick={(event) => {
                    event.stopPropagation();
                    onTaskClick(t);
                  }}
                >
                  {t.title}
                </button>
              ))}
              {dayTasks.length > 3 && (
                <div className="text-[10px] text-slate-400">+{dayTasks.length - 3} 更多</div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
