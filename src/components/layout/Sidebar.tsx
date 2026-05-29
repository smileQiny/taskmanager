import { NavLink } from 'react-router-dom';

const links = [
  { to: '/', label: '今日', icon: '◎' },
  { to: '/calendar', label: '日历', icon: '□' },
  { to: '/tasks', label: '任务', icon: '✓' },
  { to: '/pomodoro', label: '专注', icon: '◷' },
  { to: '/settings', label: '设置', icon: '⋯' },
];

export function Sidebar() {
  return (
    <aside className="flex h-screen w-20 shrink-0 flex-col items-center gap-3 border-r border-slate-200 bg-slate-950 py-4 text-slate-400">
      <div className="mb-4 flex h-10 w-10 items-center justify-center rounded-md bg-teal-400 font-semibold text-slate-950">
        TM
      </div>
      {links.map(({ to, label, icon }) => (
        <NavLink
          key={to}
          to={to}
          end={to === '/'}
          className={({ isActive }) =>
            `flex w-14 flex-col items-center gap-1 rounded-md px-2 py-2 text-xs transition
             ${isActive ? 'bg-white text-slate-950' : 'hover:bg-slate-800 hover:text-white'}`
          }
        >
          <span className="text-base">{icon}</span>
          <span>{label}</span>
        </NavLink>
      ))}
    </aside>
  );
}
