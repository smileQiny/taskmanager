import { NavLink } from 'react-router-dom';

const links = [
  { to: '/', label: '今日', icon: '☀' },
  { to: '/calendar', label: '日历', icon: '📅' },
  { to: '/tasks', label: '任务', icon: '✓' },
  { to: '/settings', label: '设置', icon: '⚙' },
];

export function Sidebar() {
  return (
    <aside className="w-16 bg-base-200 flex flex-col items-center py-4 gap-4 h-screen">
      {links.map(({ to, label, icon }) => (
        <NavLink
          key={to}
          to={to}
          end={to === '/'}
          className={({ isActive }) =>
            `flex flex-col items-center gap-1 p-2 rounded-lg w-12 text-xs
             ${isActive ? 'bg-primary text-primary-content' : 'hover:bg-base-300'}`
          }
        >
          <span className="text-lg">{icon}</span>
          <span>{label}</span>
        </NavLink>
      ))}
    </aside>
  );
}
