import { useRef, useEffect } from 'preact/hooks';
import type { LogEntry, Version } from '../types';

interface EditorPageProps {
  content: string;
  filename: string | null;
  roomId: string | null;
  siteId: number | null;
  activityLog: LogEntry[];
  versions: Version[];
  onChange: (content: string) => void;
  onSync: () => void;
  onLeaveRoom: () => void;
  onSaveVersion: (author: string | null) => void;
  onListVersions: () => void;
  onRestoreVersion: (seq: number) => void;
  onGetActivity: () => void;
}

export function EditorPage({
  content,
  filename,
  roomId,
  siteId,
  activityLog,
  versions,
  onChange,
  onSync,
  onLeaveRoom,
  onSaveVersion,
  onListVersions,
  onRestoreVersion,
  onGetActivity,
}: EditorPageProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const lastCursorPosRef = useRef<number>(0);

  useEffect(() => {
    if (textareaRef.current) {
      lastCursorPosRef.current = textareaRef.current.selectionStart;
    }
  }, [content]);

  useEffect(() => {
    if (textareaRef.current) {
      const cursorPos = Math.min(lastCursorPosRef.current, content.length);
      textareaRef.current.selectionStart = cursorPos;
      textareaRef.current.selectionEnd = cursorPos;
    }
  }, [content]);

  const handleChange = (e: Event) => {
    const target = e.target as HTMLTextAreaElement;
    onChange(target.value);
  };

  const handleSaveVersion = () => {
    const author = prompt('Enter author name (optional):');
    onSaveVersion(author || null);
  };

  const handleRestoreVersion = (seq: number) => {
    if (confirm(`Restore to version ${seq}? This will replace the current document.`)) {
      onRestoreVersion(seq);
    }
  };

  const copyRoomId = () => {
    if (roomId) {
      navigator.clipboard.writeText(roomId).then(
        () => alert('Room ID copied to clipboard!'),
        () => alert('Failed to copy room ID')
      );
    }
  };

  return (
    <div className="page editor-page">
      <div className="editor-container">
        {/* Header */}
        <div className="editor-header">
          <div className="file-info">
            <svg className="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
              <line x1="16" y1="13" x2="8" y2="13"/>
              <line x1="16" y1="17" x2="8" y2="17"/>
              <polyline points="10 9 9 9 8 9"/>
            </svg>
            <span className="filename">{filename || 'Untitled'}</span>
          </div>
          
          <div className="header-actions">
            <div className="room-badge" onClick={copyRoomId} title="Click to copy Room ID">
              <span className="badge-label">Room</span>
              <span className="badge-value">{roomId?.substring(0, 8)}...</span>
              <svg className="copy-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
            </div>
            <div className="site-badge">
              <span className="badge-label">Site</span>
              <span className="badge-value">{siteId}</span>
            </div>
            <button className="btn btn-leave" onClick={onLeaveRoom}>
              Leave Room
            </button>
          </div>
        </div>

        {/* Toolbar */}
        <div className="editor-toolbar">
          <div className="toolbar-group">
            <button className="toolbar-btn" onClick={onSync} title="Sync Document">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="23 4 23 10 17 10"/>
                <polyline points="1 20 1 14 7 14"/>
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
              </svg>
              Sync
            </button>
            <button className="toolbar-btn" onClick={onGetActivity} title="Get Activity">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
              </svg>
              Activity
            </button>
          </div>
          <div className="toolbar-group">
            <button className="toolbar-btn" onClick={handleSaveVersion} title="Save Version">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
                <polyline points="17 21 17 13 7 13 7 21"/>
                <polyline points="7 3 7 8 15 8"/>
              </svg>
              Save
            </button>
            <button className="toolbar-btn" onClick={onListVersions} title="List Versions">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10"/>
                <polyline points="12 6 12 12 16 14"/>
              </svg>
              Versions
            </button>
          </div>
        </div>

        {/* Editor Area */}
        <div className="editor-main">
          <textarea
            ref={textareaRef}
            className="editor-textarea"
            value={content}
            onInput={handleChange}
            placeholder="Start typing..."
            spellcheck={false}
          />
          
          {/* Version Panel (conditional) */}
          {versions.length > 0 && (
            <div className="version-panel">
              <div className="version-header">
                <h3>Saved Versions</h3>
                <button className="close-btn" onClick={() => onListVersions()}>×</button>
              </div>
              <div className="version-list">
                {versions.map((version) => (
                  <div
                    key={version.seq}
                    className="version-item"
                    onClick={() => handleRestoreVersion(version.seq)}
                  >
                    <div className="version-seq">v{version.seq}</div>
                    <div className="version-meta">
                      <span className="version-author">{version.author || 'Anonymous'}</span>
                      <span className="version-time">{new Date(version.timestamp).toLocaleString()}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Activity Log */}
        <div className="activity-panel">
          <div className="activity-header">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
            </svg>
            <span>Activity Log</span>
          </div>
          <div className="activity-entries">
            {activityLog.length === 0 ? (
              <div className="activity-empty">No activity yet</div>
            ) : (
              activityLog.map((entry, index) => (
                <div key={index} className={`activity-entry ${entry.type}`}>
                  <span className="entry-time">{entry.timestamp.toLocaleTimeString()}</span>
                  <span className="entry-message">{entry.message}</span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
