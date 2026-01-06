# BearShare - Collaborative Editor

A real-time collaborative text editor with end-to-end encryption, built with TypeScript, Vite, and Preact.

---

## Features

- **End-to-End Encryption**: X25519 key exchange + ChaCha20-Poly1305 AEAD
- **Real-time Collaboration**: Multiple users can edit the same document simultaneously
- **Room Management**: Create and join password-protected rooms
- **Version Control**: Save and restore document versions
- **Activity Logging**: Track all document activities
- **Secure WebSocket**: Custom handshake protocol with encrypted channels
- **Multi-Page UI**: Clean, focused pages for each task

---

## Tech Stack

- **TypeScript**: Full type safety throughout the application
- **Preact**: Lightweight React alternative for UI
- **Vite**: Fast build tool and dev server
- **Noble Crypto**: Modern cryptographic primitives
  - `@noble/ciphers`: ChaCha20-Poly1305 encryption
  - `@noble/curves`: X25519 key exchange
  - `@noble/hashes`: SHA-256, HKDF, HMAC

---

## Project Structure

```sh
bearshare/
├── src/
│   ├── pages/               # Page components
│   │   ├── ConnectPage.tsx  # Server connection page
│   │   ├── MenuPage.tsx     # Create/Join room selection
│   │   ├── CreateRoomPage.tsx  # Room creation form
│   │   ├── JoinRoomPage.tsx    # Room joining form
│   │   ├── EditorPage.tsx      # Main editor with activity log
│   │   └── index.ts            # Page exports
│   ├── hooks/               # Custom React hooks
│   │   ├── useRouter.ts         # Simple page router
│   │   ├── useEditorChanges.ts  # Editor change detection
│   │   └── useWebSocket.ts      # WebSocket connection management
│   ├── services/            # Core services
│   │   ├── crypto.ts        # Encryption and handshake protocol
│   │   └── websocket.ts     # WebSocket service wrapper
│   ├── styles/              # CSS styles
│   │   └── app.css          # Main stylesheet
│   ├── types/               # TypeScript type definitions
│   │   └── index.ts         # All type interfaces
│   ├── App.tsx              # Main app component with routing
│   └── main.tsx             # Application entry point
├── index.html               # HTML entry point
├── package.json             # Dependencies and scripts
├── tsconfig.json            # TypeScript configuration
└── vite.config.ts           # Vite configuration
```

---

## Installation

```bash
# Install dependencies
yarn install

# Start development server
yarn run dev

# Build for production
yarn run build

# Preview production build
yarn run preview

# Type check without building
yarn run type-check
```

---

## Page Flow

1. **Connect Page**: Enter server IP and port, click connect
2. **Menu Page**: Choose to create or join a room
3. **Create Room Page**: Enter room name and password
4. **Join Room Page**: Enter room ID and password
5. **Editor Page**: Edit document with real-time sync and activity log

---

## Development

The application will be available at `http://localhost:3000` when running the dev server.

### Key Components

- **App.tsx**: Main application component with page routing
- **useRouter**: Simple page navigation hook
- **useWebSocket**: WebSocket connection, encryption, and message handling
- **useEditorChanges**: Handles editor change detection and debouncing
- **SecureChannel**: Implements the encrypted communication channel
- **performHandshake**: Executes the X25519 + ChaCha20-Poly1305 handshake

### Type Safety

All components, hooks, and services are fully typed with TypeScript:

- `Page` type for router navigation
- `ServerMessage` and `ClientMessage` types for WebSocket messages
- `ConnectionStatus`, `RoomState`, `LogEntry` for application state
- Proper type inference throughout the application

---

## WebSocket Protocol

The application uses a custom secure protocol:

1. **Handshake Phase**: X25519 key exchange with verify data
2. **Encrypted Communication**: ChaCha20-Poly1305 AEAD with sequence numbers
3. **Message Types**: JSON messages for room management and document operations

---

## Message Types

### Client → Server

- `CreateRoom`, `JoinRoom`, `LeaveRoom`
- `Insert`, `Delete` (document operations)
- `RequestSync`, `SaveVersion`, `RestoreVersion`
- `ListVersions`, `GetActivityLog`

### Server → Client

- `RoomCreated`, `JoinedRoom`
- `SyncResponse`, `Operation`
- `UserJoined`, `UserLeft`
- `VersionSaved`, `VersionList`, `VersionRestored`
- `ActivityLog`, `Error`, `Pong`
