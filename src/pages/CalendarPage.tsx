import { useEffect } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { CalendarView } from '../components/calendar/CalendarView';
import { useCalendarStore } from '../stores/calendarStore';
import { useTaskStore } from '../stores/taskStore';

export function CalendarPage() {
  const { viewType, selectedDate, setViewType, goToToday, goToPrev, goToNext, setSelectedDate } = useCalendarStore();
  const { tasks, fetchTasks } = useTaskStore();

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const title = (() => {
    switch (viewType) {
      case 'month': return selectedDate.format('YYYY年M月');
      case 'week': return `${selectedDate.startOf('week').format('M/D')} - ${selectedDate.endOf('week').format('M/D')}`;
      case 'day': return selectedDate.format('YYYY年M月D日 ddd');
    }
  })();

  return (
    <div className="flex flex-col flex-1">
      <TopBar title={title}>
        <button className="btn btn-ghost btn-xs" onClick={goToPrev}>◀</button>
        <button className="btn btn-ghost btn-xs" onClick={goToToday}>今天</button>
        <button className="btn btn-ghost btn-xs" onClick={goToNext}>▶</button>
        <div className="join ml-2">
          {(['month', 'week', 'day'] as const).map((v) => (
            <button key={v}
              className={`btn btn-xs join-item ${viewType === v ? 'btn-active' : ''}`}
              onClick={() => setViewType(v)}
            >
              {{ month: '月', week: '周', day: '日' }[v]}
            </button>
          ))}
        </div>
      </TopBar>
      <div className="flex-1 overflow-hidden p-4">
        <CalendarView
          viewType={viewType}
          selectedDate={selectedDate}
          tasks={tasks}
          onDateClick={(date) => {
            setSelectedDate(date);
            setViewType('day');
          }}
        />
      </div>
    </div>
  );
}
