import { SecureChannel, performHandshake } from './crypto';
import type { ServerMessage, ClientMessage, LogEntry } from '../types';

type MessageHandler = (message: ServerMessage) => void;
type LogHandler = (entry: LogEntry) => void;

export class WebSocketService {
  private ws: WebSocket | null = null;
  private secureChannel: SecureChannel | null = null;
  private messageHandler: MessageHandler | null = null;
  private logHandler: LogHandler | null = null;

  setMessageHandler(handler: MessageHandler): void {
    this.messageHandler = handler;
  }

  setLogHandler(handler: LogHandler): void {
    this.logHandler = handler;
  }

  private log(message: string, type: 'info' | 'success' | 'error' = 'info'): void {
    if (this.logHandler) {
      this.logHandler({
        timestamp: new Date(),
        message,
        type,
      });
    }
  }

  async connect(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.log('Connecting to server...', 'info');
        this.ws = new WebSocket(url);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = async () => {
          try {
            this.log('WebSocket connected, performing handshake...', 'info');
            if (!this.ws) throw new Error('WebSocket is null');
            
            this.secureChannel = await performHandshake(this.ws);
            this.log('Secure channel established!', 'success');
            this.setupEncryptedMessageHandler();
            resolve();
          } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            this.log('Handshake failed: ' + errorMessage, 'error');
            reject(error);
          }
        };

        this.ws.onerror = (error) => {
          this.log('WebSocket error', 'error');
          reject(error);
        };

        this.ws.onclose = () => {
          this.log('Connection closed', 'info');
          this.secureChannel = null;
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  private setupEncryptedMessageHandler(): void {
    if (!this.ws) return;

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        if (!this.secureChannel) {
          this.log('Received message but no secure channel', 'error');
          return;
        }

        const encrypted = new Uint8Array(event.data as ArrayBuffer);
        const decrypted = this.secureChannel.decrypt(encrypted);
        const json = new TextDecoder().decode(decrypted);
        const message = JSON.parse(json) as ServerMessage;

        if (this.messageHandler) {
          this.messageHandler(message);
        }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        this.log('Failed to decrypt message: ' + errorMessage, 'error');
      }
    };
  }

  send(message: ClientMessage): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN || !this.secureChannel) {
      this.log('Cannot send: not connected or secure channel not established', 'error');
      return;
    }

    try {
      const json = JSON.stringify(message);
      const plaintext = new TextEncoder().encode(json);
      const encrypted = this.secureChannel.encrypt(plaintext);
      this.ws.send(encrypted);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      this.log('Failed to send message: ' + errorMessage, 'error');
    }
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
      this.secureChannel = null;
    }
  }

  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN && this.secureChannel !== null;
  }
}
