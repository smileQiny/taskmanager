import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface DayViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
}

const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function DayView({ selectedDate, tasks }: DayViewProps) {
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
    <div className="flex flex-col h-full overflow-auto">
      {allDayTasks.length > 0 && (
        <div className="border-b border-base-300 p-2 bg-base-200">
          <span className="text-xs text-base-content/60 mr-2">全天</span>
          {allDayTasks.map((t) => (
            <span key={t.id} className="badge badge-primary badge-sm mr-1">{t.title}</span>
          ))}
        </div>
      )}
      <div className="flex-1">
        {HOURS.map((hour) => {
          const hourTasks = getTasksForHour(hour);
          return (
            <div key={hour} className="flex border-b border-base-200 min-h-[48px]">
              <div className="w-16 text-right pr-3 pt-1 text-xs text-base-content/50 shrink-0">
                {hour.toString().padStart(2, '0')}:00
              </div>
              <div className="flex-1 p-1 border-l border-base-200">
                {hourTasks.map((t) => (
                  <div key={t.id} className="text-sm px-2 py-1 rounded bg-primary/10 text-primary mb-1">
                    {t.title}
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
