export interface SidebarSection {
  key: string;
  label: string;
  icon: string;
  badge?: number;
}

export function Sidebar(props: {
  sections: SidebarSection[];
  active: string;
  onSelect: (key: string) => void;
}) {
  return (
    <nav className="w-44 shrink-0 bg-gray-50 border-r border-gray-200 flex flex-col py-3">
      <div className="px-4 pb-3 text-base font-bold text-gray-800">QuickNote</div>
      {props.sections.map((s) => (
        <button
          key={s.key}
          data-testid={`sidebar-${s.key}`}
          onClick={() => props.onSelect(s.key)}
          className={`flex items-center gap-2 px-4 py-2 text-sm text-left ${
            props.active === s.key
              ? 'bg-blue-50 text-blue-700 font-medium'
              : 'text-gray-600 hover:bg-gray-100'
          }`}
        >
          <span>{s.icon}</span>
          <span className="truncate">{s.label}</span>
          {s.badge ? (
            <span className="ml-auto bg-blue-500 text-white text-xs rounded-full px-1.5">
              {s.badge}
            </span>
          ) : null}
        </button>
      ))}
    </nav>
  );
}
