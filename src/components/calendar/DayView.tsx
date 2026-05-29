import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface DayViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onSlotClick: (date: Dayjs) => void;
  onTaskClick: (task: Task) => void;
}

const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function DayView({ selectedDate, tasks, onSlotClick, onTaskClick }: DayViewProps) {
  const getTasksForHour = (hour: number) => {
    const slotStart = selectedDate.hour(hour).minute(0).second(0).unix();
    const slotEnd = selectedDate.hour(hour).minute(59).second(59).unix();
    return tasks.filter((t) => {
      if (!t.start_time) return false;
      return t.start_time >= slotStart && t.start_time <= slotEnd;
    });
  };

  const allDayTasks = tasks.filter((t) => t.all_day && t.start_time &&
    dayjs.unix(t.start_time).isSame(selectedDate, 'day'));

  return (
    <div className="flex h-full flex-col overflow-auto rounded-md border border-slate-200 bg-white">
      {allDayTasks.length > 0 && (
        <div className="border-b border-slate-200 bg-slate-50 p-2">
          <span className="mr-2 text-xs text-slate-500">全天</span>
          {allDayTasks.map((t) => (
            <button key={t.id} className="badge badge-primary badge-sm mr-1" onClick={() => onTaskClick(t)}>{t.title}</button>
          ))}
        </div>
      )}
      <div className="flex-1">
        {HOURS.map((hour) => {
          const hourTasks = getTasksForHour(hour);
          return (
            <div key={hour} className="flex min-h-[52px] border-b border-slate-100">
              <div className="w-16 shrink-0 pr-3 pt-1 text-right text-xs text-slate-400">
                {hour.toString().padStart(2, '0')}:00
              </div>
              <div className="flex-1 border-l border-slate-100 p-1 hover:bg-teal-50/40" onClick={() => onSlotClick(selectedDate.hour(hour).minute(0).second(0))}>
                {hourTasks.map((t) => (
                  <button
                    key={t.id}
                    className="mb-1 block w-full rounded bg-teal-50 px-2 py-1 text-left text-sm text-teal-700"
                    onClick={(event) => {
                      event.stopPropagation();
                      onTaskClick(t);
                    }}
                  >
                    {t.title}
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
