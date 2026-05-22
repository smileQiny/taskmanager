import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface WeekViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
}

const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function WeekView({ selectedDate, tasks, onDateClick }: WeekViewProps) {
  const startOfWeek = selectedDate.startOf('week');
  const days = Array.from({ length: 7 }, (_, i) => startOfWeek.add(i, 'day'));

  const getTasksForDayHour = (date: Dayjs, hour: number) => {
    const slotStart = date.hour(hour).minute(0).second(0).unix();
    const slotEnd = date.hour(hour).minute(59).second(59).unix();
    return tasks.filter((t) => {
      if (!t.start_time) return false;
      return t.start_time >= slotStart && t.start_time <= slotEnd;
    });
  };

  return (
    <div className="flex flex-col h-full overflow-auto">
      <div className="grid grid-cols-[60px_repeat(7,1fr)] sticky top-0 bg-base-100 z-10 border-b">
        <div className="p-2" />
        {days.map((d, i) => (
          <div key={i} className="text-center py-2 border-l border-base-300 cursor-pointer hover:bg-base-200"
            onClick={() => onDateClick(d)}>
            <div className="text-xs text-base-content/60">{['日','一','二','三','四','五','六'][d.day()]}</div>
            <div className={`text-sm font-medium w-7 h-7 mx-auto flex items-center justify-center rounded-full
              ${d.isSame(dayjs(), 'day') ? 'bg-primary text-primary-content' : ''}`}>
              {d.date()}
            </div>
          </div>
        ))}
      </div>
      <div className="grid grid-cols-[60px_repeat(7,1fr)] flex-1">
        {HOURS.map((hour) => (
          <div key={hour} className="contents">
            <div className="text-[10px] text-base-content/50 text-right pr-2 pt-1 h-12 border-t border-base-200">
              {hour.toString().padStart(2, '0')}:00
            </div>
            {days.map((d, di) => {
              const slotTasks = getTasksForDayHour(d, hour);
              return (
                <div key={di} className="border-l border-t border-base-200 h-12 p-0.5 relative">
                  {slotTasks.map((t) => (
                    <div key={t.id} className="text-[10px] truncate px-1 rounded bg-primary/20 text-primary mb-0.5">
                      {t.title}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
