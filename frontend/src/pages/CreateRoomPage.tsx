import { useState } from 'preact/hooks';

interface CreateRoomPageProps {
  onCreate: (name: string, password: string, initialContent: string) => void;
  onBack: () => void;
  onRoomCreated: () => void;
  roomId: string | null;
}

export function CreateRoomPage({ onCreate, onBack, onRoomCreated, roomId }: CreateRoomPageProps) {
  const [roomName, setRoomName] = useState('');
  const [password, setPassword] = useState('');
  const [isCreating, setIsCreating] = useState(false);

  const handleCreate = () => {
    if (!roomName.trim()) {
      return;
    }
    setIsCreating(true);
    onCreate(roomName.trim(), password || 'secret', '');
  };

  // Watch for room creation success
  if (roomId && isCreating) {
    onRoomCreated();
    return null;
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && roomName.trim() && !isCreating) {
      handleCreate();
    }
  };

  return (
    <div className="page form-page">
      <div className="form-card">
        <button className="back-btn" onClick={onBack}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M19 12H5M12 19l-7-7 7-7"/>
          </svg>
          Back
        </button>

        <div className="form-header">
          <div className="form-icon create-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M12 5v14M5 12h14"/>
            </svg>
          </div>
          <h2>Create Room</h2>
          <p>Start a new collaborative editing session</p>
        </div>

        <div className="form-body">
          <div className="form-group">
            <label>Room Name</label>
            <input
              type="text"
              value={roomName}
              onChange={(e) => setRoomName((e.target as HTMLInputElement).value)}
              onKeyDown={handleKeyDown}
              placeholder="my-document"
              autoFocus
              disabled={isCreating}
            />
            <span className="form-hint">This will also be your filename</span>
          </div>

          <div className="form-group">
            <label>Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword((e.target as HTMLInputElement).value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter a secure password"
              disabled={isCreating}
            />
            <span className="form-hint">Share this with collaborators</span>
          </div>

          <button
            className="btn btn-primary btn-full"
            onClick={handleCreate}
            disabled={!roomName.trim() || isCreating}
          >
            {isCreating ? (
              <>
                <span className="spinner"></span>
                Creating Room...
              </>
            ) : (
              'Create Room'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
