import { useState } from 'preact/hooks';

interface JoinRoomPageProps {
  onJoin: (roomId: string, password: string) => void;
  onBack: () => void;
  onRoomJoined: () => void;
  roomId: string | null;
}

export function JoinRoomPage({ onJoin, onBack, onRoomJoined, roomId }: JoinRoomPageProps) {
  const [roomIdInput, setRoomIdInput] = useState('');
  const [password, setPassword] = useState('');
  const [isJoining, setIsJoining] = useState(false);

  const handleJoin = () => {
    if (!roomIdInput.trim()) {
      return;
    }
    setIsJoining(true);
    onJoin(roomIdInput.trim(), password || 'secret');
  };

  // Watch for room join success
  if (roomId && isJoining) {
    onRoomJoined();
    return null;
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && roomIdInput.trim() && !isJoining) {
      handleJoin();
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
          <div className="form-icon join-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/>
              <polyline points="10 17 15 12 10 7"/>
              <line x1="15" y1="12" x2="3" y2="12"/>
            </svg>
          </div>
          <h2>Join Room</h2>
          <p>Enter an existing collaborative session</p>
        </div>

        <div className="form-body">
          <div className="form-group">
            <label>Room ID</label>
            <input
              type="text"
              value={roomIdInput}
              onChange={(e) => setRoomIdInput((e.target as HTMLInputElement).value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter the room ID"
              autoFocus
              disabled={isJoining}
            />
            <span className="form-hint">Get this from the room creator</span>
          </div>

          <div className="form-group">
            <label>Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword((e.target as HTMLInputElement).value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter room password"
              disabled={isJoining}
            />
          </div>

          <button
            className="btn btn-primary btn-full"
            onClick={handleJoin}
            disabled={!roomIdInput.trim() || isJoining}
          >
            {isJoining ? (
              <>
                <span className="spinner"></span>
                Joining Room...
              </>
            ) : (
              'Join Room'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
