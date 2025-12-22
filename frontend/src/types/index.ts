// Message Types for WebSocket Communication

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected';

export interface Version {
  seq: number;
  timestamp: string;
  author: string | null;
  content: string;
}

export interface ActivityEvent {
  action: string;
  details: string | null;
  user: string | null;
  timestamp: string;
}

// WebSocket Message Types

export type ServerMessage =
  | { type: 'RoomCreated'; room_id: string; site_id: number }
  | { type: 'JoinedRoom'; room_id: string; site_id: number; document_content: string; filename: string }
  | { type: 'SyncResponse'; document_content: string }
  | { type: 'Operation'; from_site: number }
  | { type: 'UserJoined'; site_id: number }
  | { type: 'UserLeft'; site_id: number }
  | { type: 'VersionSaved'; version: Version }
  | { type: 'VersionList'; versions: Version[] }
  | { type: 'VersionRestored'; version: Version }
  | { type: 'ActivityLog'; events: ActivityEvent[] }
  | { type: 'Error'; message: string }
  | { type: 'Pong' };

export type ClientMessage =
  | { type: 'CreateRoom'; room_name: string; password: string; filename: string; initial_content: string }
  | { type: 'JoinRoom'; room_id: string; password: string }
  | { type: 'LeaveRoom' }
  | { type: 'RequestSync' }
  | { type: 'Insert'; position: number; text: string }
  | { type: 'Delete'; position: number; length: number }
  | { type: 'SaveVersion'; author: string | null }
  | { type: 'ListVersions' }
  | { type: 'RestoreVersion'; seq: number }
  | { type: 'GetActivityLog'; limit: number }
  | { type: 'Ping' };

// Application State

export interface RoomState {
  roomId: string | null;
  siteId: number | null;
  documentContent: string;
}

export interface LogEntry {
  timestamp: Date;
  message: string;
  type: 'info' | 'success' | 'error';
}

export interface AppState {
  connectionStatus: ConnectionStatus;
  room: RoomState;
  versions: Version[];
  activityLog: LogEntry[];
}
