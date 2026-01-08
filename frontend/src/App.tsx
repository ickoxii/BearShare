import { useWebSocket } from './hooks/useWebSocket';
import { useEditorChanges } from './hooks/useEditorChanges';
import { useRouter } from './hooks/useRouter';
import { ConnectPage, MenuPage, CreateRoomPage, JoinRoomPage, EditorPage } from './pages';
import './styles/app.css';

export function App() {
  const { currentPage, navigate } = useRouter('connect');

  const {
    connectionStatus,
    roomId,
    siteId,
    filename,
    documentContent,
    versions,
    activityLog,
    connect,
    disconnect,
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

  const handleDisconnect = () => {
    disconnect();
    navigate('connect');
  };

  const handleLeaveRoom = () => {
    leaveRoom();
    navigate('menu');
  };

  const renderPage = () => {
    switch (currentPage) {
      case 'connect':
        return (
          <ConnectPage
            onConnect={connect}
            onSuccess={() => navigate('menu')}
          />
        );

      case 'menu':
        return (
          <MenuPage
            onCreateRoom={() => navigate('create-room')}
            onJoinRoom={() => navigate('join-room')}
            onDisconnect={handleDisconnect}
          />
        );

      case 'create-room':
        return (
          <CreateRoomPage
            onCreate={createRoom}
            onBack={() => navigate('menu')}
            onRoomCreated={() => navigate('editor')}
            roomId={roomId}
          />
        );

      case 'join-room':
        return (
          <JoinRoomPage
            onJoin={joinRoom}
            onBack={() => navigate('menu')}
            onRoomJoined={() => navigate('editor')}
            roomId={roomId}
          />
        );

      case 'editor':
        return (
          <EditorPage
            content={documentContent}
            filename={filename}
            roomId={roomId}
            siteId={siteId}
            activityLog={activityLog}
            versions={versions}
            onChange={handleChange}
            onSync={syncDocument}
            onLeaveRoom={handleLeaveRoom}
            onSaveVersion={saveVersion}
            onListVersions={listVersions}
            onRestoreVersion={restoreVersion}
            onGetActivity={getActivity}
          />
        );

      default:
        return null;
    }
  };

  return (
    <div className="app">
      {renderPage()}
    </div>
  );
}
