import { NavLink } from 'react-router-dom';
import appIcon from '../../../src-tauri/icons/icon.png';

const links = [
  { to: '/', label: '今日', icon: '◎' },
  { to: '/calendar', label: '日历', icon: '□' },
  { to: '/tasks', label: '任务', icon: '✓' },
  { to: '/pomodoro', label: '专注', icon: '◷' },
  { to: '/settings', label: '设置', icon: '⋯' },
];

export function Sidebar() {
  return (
    <aside className="flex h-screen w-20 shrink-0 flex-col items-center gap-3 border-r border-[#dbe5f1] bg-white/78 py-4 text-slate-500 shadow-[12px_0_36px_rgba(71,85,105,0.08)] backdrop-blur">
      <div className="mb-4 grid h-10 w-10 place-items-center rounded-xl bg-white/70 shadow-sm ring-1 ring-[#dbe5f1]">
        <img src={appIcon} alt="Task Manager" className="h-9 w-9 rounded-[10px]" />
      </div>
      {links.map(({ to, label, icon }) => (
        <NavLink
          key={to}
          to={to}
          end={to === '/'}
          className={({ isActive }) =>
            `flex w-14 flex-col items-center gap-1 rounded-xl px-2 py-2 text-xs font-medium transition
             ${isActive ? 'bg-blue-50 text-blue-700 shadow-sm' : 'hover:bg-slate-100/80 hover:text-slate-800'}`
          }
        >
          <span className="text-base">{icon}</span>
          <span>{label}</span>
        </NavLink>
      ))}
    </aside>
  );
}
