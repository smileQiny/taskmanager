interface TopBarProps {
  title: string;
  subtitle?: string;
  children?: React.ReactNode;
}

export function TopBar({ title, subtitle, children }: TopBarProps) {
  return (
    <header className="flex min-h-16 items-center justify-between border-b border-slate-200 bg-white px-5">
      <div>
        <h1 className="text-xl font-semibold tracking-tight text-slate-950">{title}</h1>
        {subtitle && <p className="text-sm text-slate-500">{subtitle}</p>}
      </div>
      <div className="flex items-center gap-2">{children}</div>
    </header>
  );
}
