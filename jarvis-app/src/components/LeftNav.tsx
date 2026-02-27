type ActiveNav = 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings';

interface LeftNavProps {
  activeNav: ActiveNav;
  onNavChange: (nav: ActiveNav) => void;
  youtubeNotification: boolean;
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export default function LeftNav({
  activeNav,
  onNavChange,
  youtubeNotification,
  collapsed,
  onToggleCollapse
}: LeftNavProps) {
  const navItems: Array<{ id: ActiveNav; label: string; icon: string }> = [
    { id: 'record', label: 'Record', icon: '🎙️' },
    { id: 'recordings', label: 'Recordings', icon: '📼' },
    { id: 'gems', label: 'Gems', icon: '💎' },
    { id: 'youtube', label: 'YouTube', icon: '📺' },
    { id: 'browser', label: 'Browser', icon: '🌐' }
  ];

  return (
    <nav className={`left-nav ${collapsed ? 'collapsed' : 'expanded'}`}>
      <div className="nav-items">
        {navItems.map(item => (
          <button
            key={item.id}
            className={`nav-item ${activeNav === item.id ? 'active' : ''}`}
            onClick={() => onNavChange(item.id)}
            title={item.label}
          >
            <span className="nav-item-icon">{item.icon}</span>
            {!collapsed && <span className="nav-item-label">{item.label}</span>}
            {item.id === 'youtube' && youtubeNotification && (
              <span className="nav-item-badge"></span>
            )}
          </button>
        ))}
      </div>

      <div className="nav-bottom">
        <button
          className={`nav-item ${activeNav === 'settings' ? 'active' : ''}`}
          onClick={() => onNavChange('settings')}
          title="Settings"
        >
          <span className="nav-item-icon">⚙️</span>
          {!collapsed && <span className="nav-item-label">Settings</span>}
        </button>

        <button
          className="nav-toggle"
          onClick={onToggleCollapse}
          title={collapsed ? 'Expand' : 'Collapse'}
        >
          <span className="nav-toggle-icon">{collapsed ? '→' : '←'}</span>
        </button>
      </div>
    </nav>
  );
}
