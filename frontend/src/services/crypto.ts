import { chacha20poly1305 } from '@noble/ciphers/chacha';
import { x25519 } from '@noble/curves/ed25519';
import { sha256 } from '@noble/hashes/sha256';
import { hkdf } from '@noble/hashes/hkdf';
import { hmac } from '@noble/hashes/hmac';
import { randomBytes } from '@noble/hashes/utils';

// Protocol Constants
const VERSION = 1;
const HS_MAGIC = new Uint8Array([0x42, 0x53, 0x48, 0x53]); // "BSHS"
const REC_MAGIC = new Uint8Array([0x42, 0x53, 0x52, 0x43]); // "BSRC"

const HS_CLIENT_HELLO = 1;
const HS_SERVER_HELLO = 2;
const HS_CLIENT_FINISHED = 3;
const HS_SERVER_FINISHED = 4;
const REC_APPLICATION_DATA = 0x17;

const HS_HEADER_LEN = 11;
const REC_HEADER_LEN = 19;

// Secure Channel Class
export class SecureChannel {
  private sendKey: Uint8Array;
  private recvKey: Uint8Array;
  private sendSeq: bigint = 0n;
  private recvSeq: bigint = 0n;

  constructor(sendKey: Uint8Array, recvKey: Uint8Array) {
    this.sendKey = sendKey;
    this.recvKey = recvKey;
  }

  encrypt(plaintext: Uint8Array): Uint8Array {
    const seq = this.sendSeq;
    this.sendSeq++;

    // Build header
    const header = new Uint8Array(REC_HEADER_LEN);
    header.set(REC_MAGIC, 0);
    header[4] = (VERSION >> 8) & 0xff;
    header[5] = VERSION & 0xff;
    header[6] = REC_APPLICATION_DATA;

    // Sequence number (big-endian 64-bit)
    const seqBytes = new Uint8Array(8);
    let s = seq;
    for (let i = 7; i >= 0; i--) {
      seqBytes[i] = Number(s & 0xffn);
      s >>= 8n;
    }
    header.set(seqBytes, 7);

    // Plaintext length (big-endian 32-bit)
    const len = plaintext.length;
    header[15] = (len >> 24) & 0xff;
    header[16] = (len >> 16) & 0xff;
    header[17] = (len >> 8) & 0xff;
    header[18] = len & 0xff;

    // Nonce: 4 zero bytes + 8-byte sequence
    const nonce = new Uint8Array(12);
    nonce.set(seqBytes, 4);

    // Encrypt with ChaCha20-Poly1305
    const chacha = chacha20poly1305(this.sendKey, nonce, header);
    const ciphertext = chacha.encrypt(plaintext);

    // Combine header + ciphertext
    const frame = new Uint8Array(header.length + ciphertext.length);
    frame.set(header, 0);
    frame.set(ciphertext, header.length);

    return frame;
  }

  decrypt(frame: Uint8Array): Uint8Array {
    if (frame.length < REC_HEADER_LEN + 16) {
      throw new Error('Record too short');
    }

    // Verify magic
    if (!arraysEqual(frame.slice(0, 4), REC_MAGIC)) {
      throw new Error('Bad record magic');
    }

    // Verify version
    const version = (frame[4] << 8) | frame[5];
    if (version !== VERSION) {
      throw new Error('Unsupported version: ' + version);
    }

    // Verify record type
    if (frame[6] !== REC_APPLICATION_DATA) {
      throw new Error('Unexpected record type: ' + frame[6]);
    }

    // Extract sequence number
    let seq = 0n;
    for (let i = 7; i < 15; i++) {
      seq = (seq << 8n) | BigInt(frame[i]);
    }

    if (seq !== this.recvSeq) {
      throw new Error(`Unexpected sequence: got ${seq}, expected ${this.recvSeq}`);
    }
    this.recvSeq++;

    // Extract plaintext length
    const plaintextLen = (frame[15] << 24) | (frame[16] << 16) | (frame[17] << 8) | frame[18];

    // Verify total length
    const expectedLen = REC_HEADER_LEN + plaintextLen + 16; // +16 for tag
    if (frame.length !== expectedLen) {
      throw new Error(`Length mismatch: got ${frame.length}, expected ${expectedLen}`);
    }

    // Extract header and ciphertext
    const header = frame.slice(0, REC_HEADER_LEN);
    const ciphertext = frame.slice(REC_HEADER_LEN);

    // Nonce
    const nonce = new Uint8Array(12);
    const seqBytes = frame.slice(7, 15);
    nonce.set(seqBytes, 4);

    // Decrypt
    const chacha = chacha20poly1305(this.recvKey, nonce, header);
    return chacha.decrypt(ciphertext);
  }
}

// Helper Functions
function arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

function encodeHandshakeFrame(hsType: number, payload: Uint8Array): Uint8Array {
  const frame = new Uint8Array(HS_HEADER_LEN + payload.length);
  frame.set(HS_MAGIC, 0);
  frame[4] = (VERSION >> 8) & 0xff;
  frame[5] = VERSION & 0xff;
  frame[6] = hsType;
  const len = payload.length;
  frame[7] = (len >> 24) & 0xff;
  frame[8] = (len >> 16) & 0xff;
  frame[9] = (len >> 8) & 0xff;
  frame[10] = len & 0xff;
  frame.set(payload, HS_HEADER_LEN);
  return frame;
}

function decodeHandshakeFrame(frame: Uint8Array): { hsType: number; payload: Uint8Array } {
  if (frame.length < HS_HEADER_LEN) {
    throw new Error('Handshake frame too short');
  }
  if (!arraysEqual(frame.slice(0, 4), HS_MAGIC)) {
    throw new Error('Bad handshake magic');
  }
  const version = (frame[4] << 8) | frame[5];
  if (version !== VERSION) {
    throw new Error('Unsupported version: ' + version);
  }
  const hsType = frame[6];
  const payloadLen = (frame[7] << 24) | (frame[8] << 16) | (frame[9] << 8) | frame[10];
  if (frame.length !== HS_HEADER_LEN + payloadLen) {
    throw new Error('Payload length mismatch');
  }
  return { hsType, payload: frame.slice(HS_HEADER_LEN) };
}

function hkdfExpand(ikm: Uint8Array, info: Uint8Array, length: number): Uint8Array {
  return hkdf(sha256, ikm, undefined, info, length);
}

function hmacSha256(key: Uint8Array, data: Uint8Array): Uint8Array {
  return hmac(sha256, key, data);
}

function sha256Hash(data: Uint8Array): Uint8Array {
  return sha256(data);
}

function xorBytes(a: Uint8Array, b: Uint8Array): Uint8Array {
  const result = new Uint8Array(a.length);
  for (let i = 0; i < a.length; i++) {
    result[i] = a[i] ^ b[i];
  }
  return result;
}

function concat(...arrays: Uint8Array[]): Uint8Array {
  const totalLen = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

// Handshake Protocol
export function performHandshake(ws: WebSocket): Promise<SecureChannel> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error('Handshake timeout'));
    }, 10000);

    let state = 'waiting_server_hello';
    let clientPrivate: Uint8Array;
    let clientPublic: Uint8Array;
    let transcript = new Uint8Array(0);
    let handshakeKey: Uint8Array;
    let sharedSecret: Uint8Array;

    // Generate X25519 keypair
    clientPrivate = randomBytes(32);
    clientPublic = x25519.getPublicKey(clientPrivate);

    // Generate random
    const clientRandom = randomBytes(32);

    // Build ClientHello payload: random (32) + pubkey (32)
    const chPayload = concat(clientRandom, clientPublic);
    const chFrame = encodeHandshakeFrame(HS_CLIENT_HELLO, chPayload);

    transcript = concat(transcript, chFrame);

    // Send ClientHello
    ws.send(chFrame);
    console.log('Sent ClientHello');

    ws.onmessage = (event: MessageEvent) => {
      try {
        const data = new Uint8Array(event.data as ArrayBuffer);

        if (state === 'waiting_server_hello') {
          const { hsType, payload } = decodeHandshakeFrame(data);

          if (hsType !== HS_SERVER_HELLO) {
            throw new Error('Expected ServerHello, got ' + hsType);
          }

          if (payload.length !== 64) {
            throw new Error('Invalid ServerHello payload length');
          }

          const serverRandom = payload.slice(0, 32);
          const serverPublic = payload.slice(32, 64);

          transcript = concat(transcript, data);

          // Compute ECDH shared secret
          sharedSecret = x25519.getSharedSecret(clientPrivate, serverPublic);

          // Derive handshake key
          handshakeKey = hkdfExpand(sharedSecret, new TextEncoder().encode('bearshare handshake key'), 32);

          // Compute ClientFinished MAC
          const th = sha256Hash(transcript);
          const clientFinished = hmacSha256(handshakeKey, th);

          // Send ClientFinished
          const cfFrame = encodeHandshakeFrame(HS_CLIENT_FINISHED, clientFinished);
          transcript = concat(transcript, cfFrame);
          ws.send(cfFrame);

          console.log('Sent ClientFinished');
          state = 'waiting_server_finished';
        } else if (state === 'waiting_server_finished') {
          const { hsType, payload } = decodeHandshakeFrame(data);

          if (hsType !== HS_SERVER_FINISHED) {
            throw new Error('Expected ServerFinished, got ' + hsType);
          }

          if (payload.length !== 32) {
            throw new Error('Invalid ServerFinished verify_data length');
          }

          const serverVerifyData = payload;

          // Verify server's verify_data
          const th = sha256Hash(transcript);
          const expectedMac = hmacSha256(handshakeKey, th);

          if (!arraysEqual(serverVerifyData, expectedMac)) {
            throw new Error('Server verify_data mismatch');
          }

          console.log('Verified ServerFinished');

          transcript = concat(transcript, data);

          // Derive application keys
          let c2sKey = hkdfExpand(sharedSecret, new TextEncoder().encode('bearshare app c2s key'), 32);
          let s2cKey = hkdfExpand(sharedSecret, new TextEncoder().encode('bearshare app s2c key'), 32);

          const finalTh = sha256Hash(transcript);
          c2sKey = xorBytes(c2sKey, finalTh);
          s2cKey = xorBytes(s2cKey, finalTh);

          // Client sends with c2s, receives with s2c
          const channel = new SecureChannel(c2sKey, s2cKey);

          console.log('Handshake complete');

          clearTimeout(timeout);
          resolve(channel);
        }
      } catch (error) {
        clearTimeout(timeout);
        reject(error);
      }
    };

    ws.onerror = (error) => {
      clearTimeout(timeout);
      reject(error);
    };

    ws.onclose = () => {
      clearTimeout(timeout);
      reject(new Error('WebSocket closed during handshake'));
    };
  });
}
