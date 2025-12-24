# BearShare Client

WebSocket client for the BearShare collaborative editor. Connects to the server and enables real-time document editing.

## Running the Client

First, start the server (in a separate terminal):

```bash
# Option 1: With SQLite (simpler, for testing)
export DATABASE_URL="sqlite:bearshare.db"
cargo run -p server

# Option 2: With MySQL (production)
docker-compose -f docker/local.docker-compose.yml up -d
export DATABASE_URL="mysql://root:password@127.0.0.1:3307/bearshare"
cargo run -p server
```

Then start the client:

```bash
cargo run -p client
```

Or specify a custom server URL:

```bash
SERVER_URL="ws://your-server:9001/ws" cargo run -p client
```

## Commands

| Command | Shortcut | Description |
| --- | --- | --- |
| `create <name> <password> [content]` | `c` | Create a new room |
| `join <room_id> <password>` | `j` | Join an existing room |
| `leave` | `l` | Leave the current room |
| `insert <pos> <text>` | `i` | Insert text at position |
| `delete <pos> <len>` | `d` | Delete characters |
| `show` | `s` | Show document content |
| `sync` | - | Sync with server |
| `status` | - | Show connection info |
| `ping` | - | Ping server |
| `help` | `h` | Show help |
| `quit` | `q` | Exit client |

## Example Session

```sh
> create myroom secret123 Hello World
[info] Creating room 'myroom'...

╔══════════════════════════════════════════════════════════════╗
║                     Room Created Successfully                ║
╠══════════════════════════════════════════════════════════════╣
║  Room ID:  abc123-def456-...                                 ║
║  Site ID:  1                                                 ║
╚══════════════════════════════════════════════════════════════╝

> show
─────────────────────────────────────────
Hello World
─────────────────────────────────────────

> insert 5 ,
[local] Inserted ',' at position 5
[local] Document: Hello, World

> delete 6 1
[local] Deleted 1 chars at position 6
[local] Document: Hello,World
```
