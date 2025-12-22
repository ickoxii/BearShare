# 🐻 BearShare - Collaborative Editor

A real-time collaborative text editor with end-to-end encryption, built with TypeScript, Vite, and Preact.

## Features

- **End-to-End Encryption**: X25519 key exchange + ChaCha20-Poly1305 AEAD
- **Real-time Collaboration**: Multiple users can edit the same document simultaneously
- **Room Management**: Create and join password-protected rooms
- **Version Control**: Save and restore document versions
- **Activity Logging**: Track all document activities
- **Secure WebSocket**: Custom handshake protocol with encrypted channels

## Tech Stack

- **TypeScript**: Full type safety throughout the application
- **Preact**: Lightweight React alternative for UI
- **Vite**: Fast build tool and dev server
- **Noble Crypto**: Modern cryptographic primitives
  - `@noble/ciphers`: ChaCha20-Poly1305 encryption
  - `@noble/curves`: X25519 key exchange
  - `@noble/hashes`: SHA-256, HKDF, HMAC

## Project Structure

```
bearshare/
├── src/
│   ├── components/          # React components
│   │   ├── ActivityLog.tsx  # Activity log display
│   │   ├── ConnectionPanel.tsx  # WebSocket connection UI
│   │   ├── Editor.tsx       # Main text editor
│   │   ├── RoomPanel.tsx    # Room management UI
│   │   ├── StatusBar.tsx    # Connection status indicator
│   │   └── VersionList.tsx  # Version history display
│   ├── hooks/               # Custom React hooks
│   │   ├── useEditorChanges.ts  # Editor change detection with debouncing
│   │   └── useWebSocket.ts      # WebSocket connection management
│   ├── services/            # Core services
│   │   ├── crypto.ts        # Encryption and handshake protocol
│   │   └── websocket.ts     # WebSocket service wrapper
│   ├── styles/              # CSS styles
│   │   └── app.css          # Main stylesheet
│   ├── types/               # TypeScript type definitions
│   │   └── index.ts         # All type interfaces
│   ├── App.tsx              # Main app component
│   └── main.tsx             # Application entry point
├── index.html               # HTML entry point
├── package.json             # Dependencies and scripts
├── tsconfig.json            # TypeScript configuration
└── vite.config.ts           # Vite configuration
```

## Installation

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview

# Type check without building
npm run type-check
```

## Development

The application will be available at `http://localhost:3000` when running the dev server.

### Key Components

- **App.tsx**: Main application component that orchestrates all features
- **useWebSocket**: Custom hook managing WebSocket connection, encryption, and message handling
- **useEditorChanges**: Handles editor change detection and debouncing
- **SecureChannel**: Implements the encrypted communication channel
- **performHandshake**: Executes the X25519 + ChaCha20-Poly1305 handshake

### Type Safety

All components, hooks, and services are fully typed with TypeScript:
- `ServerMessage` and `ClientMessage` types for WebSocket messages
- `ConnectionStatus`, `RoomState`, `LogEntry` for application state
- Proper type inference throughout the application

### State Management

State is managed through:
- `useWebSocket` hook for global connection state
- Local component state with `useState`
- Props drilling for component communication (can be upgraded to Context if needed)

## Architecture Highlights

### Idiomatic TypeScript
- Strict type checking enabled
- No `any` types used
- Proper interface definitions for all data structures
- Type-safe message handling

### Modern React/Preact Patterns
- Functional components with hooks
- Custom hooks for reusable logic
- Proper dependency arrays in `useEffect` and `useCallback`
- Ref usage for DOM manipulation when necessary

### Clean Separation of Concerns
- **Components**: Pure UI, minimal logic
- **Hooks**: Reusable stateful logic
- **Services**: Business logic and external communication
- **Types**: Centralized type definitions

### Performance Optimizations
- Debounced editor changes (300ms)
- `useCallback` for stable function references
- Proper cleanup in `useEffect` hooks
- Minimal re-renders through careful state design

## WebSocket Protocol

The application uses a custom secure protocol:
1. **Handshake Phase**: X25519 key exchange with verify data
2. **Encrypted Communication**: ChaCha20-Poly1305 AEAD with sequence numbers
3. **Message Types**: JSON messages for room management and document operations

## Message Types

### Client → Server
- `CreateRoom`, `JoinRoom`, `LeaveRoom`
- `Insert`, `Delete` (document operations)
- `RequestSync`, `SaveVersion`, `RestoreVersion`
- `ListVersions`, `GetActivityLog`

### Server → Client
- `RoomCreated`, `RoomJoined`
- `SyncResponse`, `Operation`
- `UserJoined`, `UserLeft`
- `VersionSaved`, `VersionList`, `VersionRestored`
- `ActivityLog`, `Error`, `Pong`

## Contributing

This is a complete rewrite focusing on:
- TypeScript best practices
- Modern React/Preact patterns
- Clean architecture
- Type safety
- Maintainability

## License

MIT
