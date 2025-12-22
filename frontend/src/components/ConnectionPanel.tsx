import { useState } from 'preact/hooks';

interface ConnectionPanelProps {
  onConnect: (url: string) => Promise<void>;
  isConnected: boolean;
}

export function ConnectionPanel({ onConnect, isConnected }: ConnectionPanelProps) {
  const [serverUrl, setServerUrl] = useState('ws://localhost:9001/ws');
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConnect = async () => {
    setIsConnecting(true);
    setError(null);
    try {
      await onConnect(serverUrl);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      setIsConnecting(false);
    }
  };

  if (isConnected) {
    return null;
  }

  return (
    <div className="panel">
      <h2>🔌 Connect</h2>
      <div className="form-group">
        <label>Server URL</label>
        <input
          type="text"
          value={serverUrl}
          onChange={(e) => setServerUrl((e.target as HTMLInputElement).value)}
          placeholder="ws://server:port/ws"
        />
      </div>
      {error && <div className="error-message">{error}</div>}
      <button
        className="btn btn-primary"
        onClick={handleConnect}
        disabled={isConnecting}
      >
        {isConnecting ? 'Connecting...' : 'Connect'}
      </button>
    </div>
  );
}
