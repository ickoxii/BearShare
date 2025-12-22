import type { LogEntry } from '../types';

interface ActivityLogProps {
  entries: LogEntry[];
}

export function ActivityLog({ entries }: ActivityLogProps) {
  return (
    <div className="activity-log-container">
      <h3>📋 Activity Log</h3>
      <div className="activity-log">
        {entries.map((entry, index) => (
          <div key={index} className={`log-entry ${entry.type}`}>
            [{entry.timestamp.toLocaleTimeString()}] {entry.message}
          </div>
        ))}
      </div>
    </div>
  );
}
