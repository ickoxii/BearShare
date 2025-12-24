import { useState } from 'preact/hooks';

interface RoomPanelProps {
  isConnected: boolean;
  roomId: string | null;
  siteId: number | null;
  onCreateRoom: (name: string, password: string, initialContent: string) => void;
  onJoinRoom: (roomId: string, password: string) => void;
  onLeaveRoom: () => void;
  onSaveVersion: (author: string | null) => void;
  onListVersions: () => void;
}

export function RoomPanel({
  isConnected,
  roomId,
  siteId,
  onCreateRoom,
  onJoinRoom,
  onLeaveRoom,
  onSaveVersion,
  onListVersions,
}: RoomPanelProps) {
  const [newRoomName, setNewRoomName] = useState('');
  const [newRoomPassword, setNewRoomPassword] = useState('');
  const [initialContent, setInitialContent] = useState('');
  const [joinRoomId, setJoinRoomId] = useState('');
  const [joinRoomPassword, setJoinRoomPassword] = useState('');

  if (!isConnected) {
    return null;
  }

  const handleCreateRoom = () => {
    onCreateRoom(
      newRoomName || 'untitled',
      newRoomPassword || 'secret',
      initialContent
    );
  };

  const handleJoinRoom = () => {
    if (!joinRoomId) {
      alert('Please enter a room ID');
      return;
    }
    onJoinRoom(joinRoomId, joinRoomPassword || 'secret');
  };

  const handleSaveVersion = () => {
    const author = prompt('Enter author name (optional):');
    onSaveVersion(author || null);
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
    <div className="panel">
      <h2>Room</h2>

      {!roomId ? (
        <>
          <h3 className="section-title">Create Room</h3>
          <div className="form-group">
            <label>Room Name</label>
            <input
              type="text"
              value={newRoomName}
              onChange={(e) => setNewRoomName((e.target as HTMLInputElement).value)}
              placeholder="my-document"
            />
          </div>
          <div className="form-group">
            <label>Password</label>
            <input
              type="password"
              value={newRoomPassword}
              onChange={(e) => setNewRoomPassword((e.target as HTMLInputElement).value)}
              placeholder="secret"
            />
          </div>
          <div className="form-group">
            <label>Initial Content</label>
            <input
              type="text"
              value={initialContent}
              onChange={(e) => setInitialContent((e.target as HTMLInputElement).value)}
              placeholder="Hello World"
            />
          </div>
          <button className="btn btn-primary" onClick={handleCreateRoom}>
            Create Room
          </button>

          <h3 className="section-title">Join Room</h3>
          <div className="form-group">
            <label>Room ID</label>
            <input
              type="text"
              value={joinRoomId}
              onChange={(e) => setJoinRoomId((e.target as HTMLInputElement).value)}
              placeholder="room-uuid"
            />
          </div>
          <div className="form-group">
            <label>Password</label>
            <input
              type="password"
              value={joinRoomPassword}
              onChange={(e) => setJoinRoomPassword((e.target as HTMLInputElement).value)}
              placeholder="secret"
            />
          </div>
          <button className="btn btn-secondary" onClick={handleJoinRoom}>
            Join Room
          </button>
        </>
      ) : (
        <>
          <div className="room-info-box">
            <div className="room-info-item">
              <strong>Room ID:</strong>
              <div className="room-id-container">
                <input
                  type="text"
                  value={roomId}
                  readOnly
                  className="room-id-input"
                />
                <button
                  className="btn btn-secondary btn-icon"
                  onClick={copyRoomId}
                  title="Copy Room ID"
                >
                </button>
              </div>
            </div>
            <div className="room-info-item">
              <strong>Site ID:</strong> <span>{siteId}</span>
            </div>
          </div>
          <button className="btn btn-danger" onClick={onLeaveRoom}>
            Leave Room
          </button>

          <div className="version-section">
            <h3 className="section-title">Versions</h3>
            <button className="btn btn-secondary" onClick={handleSaveVersion}>
              Save Version
            </button>
            <button className="btn btn-secondary" onClick={onListVersions}>
              List Versions
            </button>
          </div>
        </>
      )}
    </div>
  );
}
