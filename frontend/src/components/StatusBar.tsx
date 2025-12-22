import type { ConnectionStatus } from '../types';

interface StatusBarProps {
  status: ConnectionStatus;
  siteId: number | null;
}

export function StatusBar({ status, siteId }: StatusBarProps) {
  const statusText = {
    disconnected: 'Disconnected',
    connecting: 'Connecting...',
    connected: 'Connected',
  }[status];

  const statusClass = status === 'connected' ? 'connected' : '';

  return (
    <div className="status-bar">
      <div className="status-indicator">
        <div className={`status-dot ${statusClass}`} />
        <span>{statusText}</span>
      </div>
      {siteId !== null && (
        <div className="user-info">Site ID: {siteId}</div>
      )}
    </div>
  );
}
