import { useWebSocket } from './hooks/useWebSocket';
import { useEditorChanges } from './hooks/useEditorChanges';
import { StatusBar } from './components/StatusBar';
import { ConnectionPanel } from './components/ConnectionPanel';
import { RoomPanel } from './components/RoomPanel';
import { ActivityLog } from './components/ActivityLog';
import { Editor } from './components/Editor';
import { VersionList } from './components/VersionList';
import './styles/app.css';

export function App() {
  const {
    connectionStatus,
    roomId,
    siteId,
    documentContent,
    versions,
    activityLog,
    connect,
    createRoom,
    joinRoom,
    leaveRoom,
    syncDocument,
    sendEdit,
    sendDelete,
    saveVersion,
    listVersions,
    restoreVersion,
    getActivity,
  } = useWebSocket();

  const { handleChange } = useEditorChanges({
    documentContent,
    onInsert: sendEdit,
    onDelete: sendDelete,
  });

  const isConnected = connectionStatus === 'connected';
  const inRoom = roomId !== null;

  return (
    <div className="container">
      <header>
        <h1>BearShare</h1>
        <p>Real-time Collaborative Editor</p>
      </header>

      <StatusBar status={connectionStatus} siteId={siteId} />

      <div className="main-content">
        <div className="sidebar">
          <ConnectionPanel onConnect={connect} isConnected={isConnected} />

          <RoomPanel
            isConnected={isConnected}
            roomId={roomId}
            siteId={siteId}
            onCreateRoom={createRoom}
            onJoinRoom={joinRoom}
            onLeaveRoom={leaveRoom}
            onSaveVersion={saveVersion}
            onListVersions={listVersions}
          />

          {versions.length > 0 && (
            <VersionList versions={versions} onRestore={restoreVersion} />
          )}

          <ActivityLog entries={activityLog} />
        </div>

        <Editor
          content={documentContent}
          onChange={handleChange}
          disabled={!inRoom}
          roomId={roomId}
          onSync={syncDocument}
          onGetActivity={getActivity}
        />
      </div>
    </div>
  );
}
