interface MenuPageProps {
  onCreateRoom: () => void;
  onJoinRoom: () => void;
  onDisconnect: () => void;
}

export function MenuPage({ onCreateRoom, onJoinRoom, onDisconnect }: MenuPageProps) {
  return (
    <div className="page menu-page">
      <div className="menu-card">
        <div className="menu-header">
          <h1>BearShare</h1>
          <div className="connection-badge">
            <span className="status-dot connected"></span>
            Connected
          </div>
        </div>

        <div className="menu-options">
          <button className="menu-btn create-btn" onClick={onCreateRoom}>
            <div className="btn-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 5v14M5 12h14"/>
              </svg>
            </div>
            <div className="btn-content">
              <span className="btn-title">Create Room</span>
              <span className="btn-desc">Start a new collaborative session</span>
            </div>
            <span className="btn-arrow">→</span>
          </button>

          <button className="menu-btn join-btn" onClick={onJoinRoom}>
            <div className="btn-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/>
                <polyline points="10 17 15 12 10 7"/>
                <line x1="15" y1="12" x2="3" y2="12"/>
              </svg>
            </div>
            <div className="btn-content">
              <span className="btn-title">Join Room</span>
              <span className="btn-desc">Enter an existing room</span>
            </div>
            <span className="btn-arrow">→</span>
          </button>
        </div>

        <button className="disconnect-link" onClick={onDisconnect}>
          ← Disconnect from server
        </button>
      </div>
    </div>
  );
}
