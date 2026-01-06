import { useState } from 'preact/hooks';

interface ConnectPageProps {
  onConnect: (url: string) => Promise<void>;
  onSuccess: () => void;
}

export function ConnectPage({ onConnect, onSuccess }: ConnectPageProps) {
  const [serverAddress, setServerAddress] = useState('127.0.0.1');
  const [port, setPort] = useState('9001');
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConnect = async () => {
    setIsConnecting(true);
    setError(null);
    
    const url = `ws://${serverAddress}:${port}/ws`;
    
    try {
      await onConnect(url);
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      setIsConnecting(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !isConnecting) {
      handleConnect();
    }
  };

  return (
    <div className="page connect-page">
      <div className="connect-card">
        <div className="logo-section">
          <div className="logo-icon">
            <svg viewBox="0 0 100 100" className="bear-icon">
              <circle cx="50" cy="50" r="40" fill="currentColor" opacity="0.2"/>
              <circle cx="25" cy="25" r="12" fill="currentColor"/>
              <circle cx="75" cy="25" r="12" fill="currentColor"/>
              <circle cx="50" cy="55" r="30" fill="currentColor"/>
              <circle cx="38" cy="48" r="5" fill="#1a1a2e"/>
              <circle cx="62" cy="48" r="5" fill="#1a1a2e"/>
              <ellipse cx="50" cy="62" rx="8" ry="6" fill="#1a1a2e"/>
            </svg>
          </div>
          <h1>BearShare</h1>
          <p className="tagline">Collaborative editing, secured.</p>
        </div>

        <div className="connect-form">
          <div className="form-row">
            <div className="form-group address-group">
              <label>Server Address</label>
              <input
                type="text"
                value={serverAddress}
                onChange={(e) => setServerAddress((e.target as HTMLInputElement).value)}
                onKeyDown={handleKeyDown}
                placeholder="127.0.0.1"
                disabled={isConnecting}
              />
            </div>
            <div className="form-group port-group">
              <label>Port</label>
              <input
                type="text"
                value={port}
                onChange={(e) => setPort((e.target as HTMLInputElement).value)}
                onKeyDown={handleKeyDown}
                placeholder="9001"
                disabled={isConnecting}
              />
            </div>
          </div>

          {error && (
            <div className="error-message">
              <span className="error-icon">⚠</span>
              {error}
            </div>
          )}

          <button
            className="btn btn-connect"
            onClick={handleConnect}
            disabled={isConnecting}
          >
            {isConnecting ? (
              <>
                <span className="spinner"></span>
                Connecting...
              </>
            ) : (
              <>
                <span className="connect-icon">→</span>
                Connect to Server
              </>
            )}
          </button>
        </div>

        <div className="footer-note">
          <p>End-to-end encrypted • Real-time sync</p>
        </div>
      </div>
    </div>
  );
}
