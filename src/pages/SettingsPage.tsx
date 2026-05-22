import { TopBar } from '../components/layout/TopBar';

export function SettingsPage() {
  return (
    <div className="flex flex-col flex-1">
      <TopBar title="设置" />
      <div className="flex-1 p-4">
        <p className="text-base-content/60">设置页面</p>
      </div>
    </div>
  );
}
