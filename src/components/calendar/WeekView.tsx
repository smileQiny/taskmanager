import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface WeekViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
  onSlotClick: (date: Dayjs) => void;
  onTaskClick: (task: Task) => void;
}

const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function WeekView({ selectedDate, tasks, onDateClick, onSlotClick, onTaskClick }: WeekViewProps) {
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
    <div className="flex h-full flex-col overflow-auto rounded-md border border-slate-200 bg-white">
      <div className="sticky top-0 z-10 grid grid-cols-[60px_repeat(7,1fr)] border-b border-slate-200 bg-white">
        <div className="p-2" />
        {days.map((d, i) => (
          <div key={i} className="cursor-pointer border-l border-slate-200 py-2 text-center hover:bg-teal-50/60"
            onClick={() => onDateClick(d)}>
            <div className="text-xs text-slate-500">{['日','一','二','三','四','五','六'][d.day()]}</div>
            <div className={`mx-auto flex h-7 w-7 items-center justify-center rounded-full text-sm font-medium
              ${d.isSame(dayjs(), 'day') ? 'bg-slate-950 text-white' : 'text-slate-700'}`}>
              {d.date()}
            </div>
          </div>
        ))}
      </div>
      <div className="grid grid-cols-[60px_repeat(7,1fr)] flex-1">
        {HOURS.map((hour) => (
          <div key={hour} className="contents">
            <div className="h-12 border-t border-slate-100 pr-2 pt-1 text-right text-[10px] text-slate-400">
              {hour.toString().padStart(2, '0')}:00
            </div>
            {days.map((d, di) => {
              const slotTasks = getTasksForDayHour(d, hour);
              return (
                <div key={di} className="relative h-12 border-l border-t border-slate-100 p-0.5 hover:bg-teal-50/40" onClick={() => onSlotClick(d.hour(hour).minute(0).second(0))}>
                  {slotTasks.map((t) => (
                    <button
                      key={t.id}
                      className="mb-0.5 block w-full truncate rounded bg-teal-50 px-1 text-left text-[10px] text-teal-700"
                      onClick={(event) => {
                        event.stopPropagation();
                        onTaskClick(t);
                      }}
                    >
                      {t.title}
                    </button>
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
