import { useState, useCallback, useEffect } from 'preact/hooks';
import { WebSocketService } from '../services/websocket';
import type { ServerMessage, LogEntry, ConnectionStatus, Version, ActivityEvent } from '../types';

interface UseWebSocketResult {
  connectionStatus: ConnectionStatus;
  roomId: string | null;
  siteId: number | null;
  filename: string | null;
  documentContent: string;
  versions: Version[];
  activityLog: LogEntry[];
  connect: (url: string) => Promise<void>;
  disconnect: () => void;
  createRoom: (name: string, password: string, initialContent: string) => void;
  joinRoom: (roomId: string, password: string) => void;
  leaveRoom: () => void;
  syncDocument: () => void;
  sendEdit: (position: number, text: string) => void;
  sendDelete: (position: number, length: number) => void;
  saveVersion: (author: string | null) => void;
  listVersions: () => void;
  restoreVersion: (seq: number) => void;
  getActivity: () => void;
  setDocumentContent: (content: string) => void;
}

export function useWebSocket(): UseWebSocketResult {
  const [wsService] = useState(() => new WebSocketService());
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected');
  const [roomId, setRoomId] = useState<string | null>(null);
  const [siteId, setSiteId] = useState<number | null>(null);
  const [filename, setFilename] = useState<string | null>(null);
  const [documentContent, setDocumentContent] = useState<string>('');
  const [versions, setVersions] = useState<Version[]>([]);
  const [activityLog, setActivityLog] = useState<LogEntry[]>([]);

  const addLog = useCallback((entry: LogEntry) => {
    setActivityLog((prev) => [entry, ...prev].slice(0, 50));
  }, []);

  const handleMessage = useCallback((message: ServerMessage) => {
    switch (message.type) {
      case 'RoomCreated':
        setRoomId(message.room_id);
        setSiteId(message.site_id);
        addLog({ timestamp: new Date(), message: `Room created: ${message.room_id}`, type: 'success' });
        break;

      case 'JoinedRoom':
        setRoomId(message.room_id);
        setSiteId(message.site_id);
        setFilename(message.filename);
        setDocumentContent(message.document_content);
        addLog({ timestamp: new Date(), message: `Joined room: ${message.room_id}`, type: 'success' });
        wsService.send({ type: 'RequestSync' });
        addLog({ timestamp: new Date(), message: 'Requesting document sync...', type: 'info' });
        break;

      case 'SyncResponse':
        setDocumentContent(message.document_content);
        addLog({ timestamp: new Date(), message: 'Document synced', type: 'info' });
        break;

      case 'Operation':
        addLog({ timestamp: new Date(), message: `Remote edit from site ${message.from_site}`, type: 'info' });
        break;

      case 'UserJoined':
        addLog({ timestamp: new Date(), message: `User joined (site ${message.site_id})`, type: 'info' });
        break;

      case 'UserLeft':
        addLog({ timestamp: new Date(), message: `User left (site ${message.site_id})`, type: 'info' });
        break;

      case 'VersionSaved':
        addLog({ timestamp: new Date(), message: `Version ${message.version.seq} saved`, type: 'success' });
        break;

      case 'VersionList':
        setVersions(message.versions);
        break;

      case 'VersionRestored':
        setDocumentContent(message.version.content);
        addLog({ timestamp: new Date(), message: `Restored to version ${message.version.seq}`, type: 'success' });
        break;

      case 'ActivityLog':
        message.events.forEach((e: ActivityEvent) => {
          addLog({
            timestamp: new Date(),
            message: `${e.action}: ${e.details || ''} (${e.user || 'system'})`,
            type: 'info',
          });
        });
        break;

      case 'Error':
        addLog({ timestamp: new Date(), message: 'Error: ' + message.message, type: 'error' });
        break;

      case 'Pong':
        addLog({ timestamp: new Date(), message: 'Pong received', type: 'info' });
        break;
    }
  }, [addLog, wsService]);

  useEffect(() => {
    wsService.setMessageHandler(handleMessage);
    wsService.setLogHandler(addLog);
  }, [wsService, handleMessage, addLog]);

  const connect = useCallback(async (url: string) => {
    setConnectionStatus('connecting');
    try {
      await wsService.connect(url);
      setConnectionStatus('connected');
    } catch (error) {
      setConnectionStatus('disconnected');
      throw error;
    }
  }, [wsService]);

  const disconnect = useCallback(() => {
    wsService.disconnect();
    setConnectionStatus('disconnected');
    setRoomId(null);
    setSiteId(null);
    setFilename(null);
    setDocumentContent('');
  }, [wsService]);

  const createRoom = useCallback((name: string, password: string, initialContent: string) => {
    const fname = name;
    setFilename(fname);
    wsService.send({
      type: 'CreateRoom',
      room_name: name,
      password: password,
      filename: fname,
      initial_content: initialContent,
    });
  }, [wsService]);

  const joinRoom = useCallback((roomId: string, password: string) => {
    wsService.send({
      type: 'JoinRoom',
      room_id: roomId,
      password: password,
    });
  }, [wsService]);

  const leaveRoom = useCallback(() => {
    wsService.send({ type: 'LeaveRoom' });
    setRoomId(null);
    setSiteId(null);
    setFilename(null);
    setDocumentContent('');
    addLog({ timestamp: new Date(), message: 'Left room', type: 'info' });
  }, [wsService, addLog]);

  const syncDocument = useCallback(() => {
    wsService.send({ type: 'RequestSync' });
  }, [wsService]);

  const sendEdit = useCallback((position: number, text: string) => {
    wsService.send({
      type: 'Insert',
      position,
      text,
    });
  }, [wsService]);

  const sendDelete = useCallback((position: number, length: number) => {
    wsService.send({
      type: 'Delete',
      position,
      length,
    });
  }, [wsService]);

  const saveVersion = useCallback((author: string | null) => {
    wsService.send({
      type: 'SaveVersion',
      author,
    });
  }, [wsService]);

  const listVersions = useCallback(() => {
    wsService.send({ type: 'ListVersions' });
  }, [wsService]);

  const restoreVersion = useCallback((seq: number) => {
    wsService.send({
      type: 'RestoreVersion',
      seq,
    });
  }, [wsService]);

  const getActivity = useCallback(() => {
    wsService.send({
      type: 'GetActivityLog',
      limit: 20,
    });
  }, [wsService]);

  return {
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
    setDocumentContent,
  };
}
