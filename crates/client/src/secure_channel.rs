// Client-side secure channel implementation

use anyhow::{anyhow, bail, Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305,
};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::Message;
use x25519_dalek::{EphemeralSecret, PublicKey};
use zeroize::Zeroize;

pub const VERSION: u16 = 1;

const HS_MAGIC: [u8; 4] = *b"BSHS";
const REC_MAGIC: [u8; 4] = *b"BSRC";

const HS_CLIENT_HELLO: u8 = 1;
const HS_SERVER_HELLO: u8 = 2;
const HS_CLIENT_FINISHED: u8 = 3;
const HS_SERVER_FINISHED: u8 = 4;

const REC_APPLICATION_DATA: u8 = 0x17;

const HS_HEADER_LEN: usize = 4 + 2 + 1 + 4;
const REC_HEADER_LEN: usize = 4 + 2 + 1 + 8 + 4;
const AEAD_TAG_LEN: usize = 16;

// Secure writer for encrypting outbound messages
pub struct SecureWrite {
    cipher: ChaCha20Poly1305,
    send_seq: u64,
}

// Secure reader for decrypting inbound messages
pub struct SecureRead {
    cipher: ChaCha20Poly1305,
    recv_seq: u64,
}

impl SecureWrite {
    // Encrypt a plaintext message into a framed record
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let seq = self.send_seq;
        self.send_seq = self
            .send_seq
            .checked_add(1)
            .ok_or_else(|| anyhow!("send sequence overflow"))?;

        let mut header = Vec::with_capacity(REC_HEADER_LEN);
        header.extend_from_slice(&REC_MAGIC);
        header.extend_from_slice(&VERSION.to_be_bytes());
        header.push(REC_APPLICATION_DATA);
        header.extend_from_slice(&seq.to_be_bytes());
        header.extend_from_slice(&(plaintext.len() as u32).to_be_bytes());

        let mut nonce = [0u8; 12];
        nonce[4..].copy_from_slice(&seq.to_be_bytes());

        let ciphertext = self
            .cipher
            .encrypt(
                (&nonce).into(),
                Payload {
                    msg: plaintext,
                    aad: &header,
                },
            )
            .map_err(|_| anyhow!("record encryption failed"))?;

        let mut frame = header;
        frame.extend_from_slice(&ciphertext);
        Ok(frame)
    }
}

impl SecureRead {
    // Decrypt a framed record into plaintext
    pub fn decrypt(&mut self, frame: &[u8]) -> Result<Vec<u8>> {
        if frame.len() < REC_HEADER_LEN + AEAD_TAG_LEN {
            bail!("record too short");
        }

        if &frame[0..4] != REC_MAGIC {
            bail!("bad record magic");
        }

        let version = u16::from_be_bytes([frame[4], frame[5]]);
        if version != VERSION {
            bail!("unsupported record version: {}", version);
        }

        let rec_type = frame[6];
        if rec_type != REC_APPLICATION_DATA {
            bail!("unexpected record type: {}", rec_type);
        }

        let seq = u64::from_be_bytes(frame[7..15].try_into().unwrap());
        let plaintext_len = u32::from_be_bytes(frame[15..19].try_into().unwrap()) as usize;

        if seq != self.recv_seq {
            bail!(
                "unexpected recv seq: got {}, expected {}",
                seq,
                self.recv_seq
            );
        }
        self.recv_seq = self
            .recv_seq
            .checked_add(1)
            .ok_or_else(|| anyhow!("recv sequence overflow"))?;

        let expected_len = REC_HEADER_LEN + plaintext_len + AEAD_TAG_LEN;
        if frame.len() != expected_len {
            bail!(
                "record length mismatch: got {}, expected {}",
                frame.len(),
                expected_len
            );
        }

        let header = &frame[..REC_HEADER_LEN];
        let ciphertext = &frame[REC_HEADER_LEN..];

        let mut nonce = [0u8; 12];
        nonce[4..].copy_from_slice(&seq.to_be_bytes());

        let plaintext = self
            .cipher
            .decrypt(
                (&nonce).into(),
                Payload {
                    msg: ciphertext,
                    aad: header,
                },
            )
            .map_err(|_| anyhow!("record authentication failed (bad tag)"))?;

        Ok(plaintext)
    }
}

// Perform client-side handshake to establish secure channel
pub async fn client_handshake<S, R, E>(
    sender: &mut S,
    receiver: &mut R,
) -> Result<(SecureWrite, SecureRead)>
where
    S: Sink<Message, Error = E> + Unpin,
    R: Stream<Item = std::result::Result<Message, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    // Generate client ephemeral keys
    let client_secret = EphemeralSecret::random_from_rng(OsRng);
    let client_pub = PublicKey::from(&client_secret);

    // Generate client random
    let mut client_random = [0u8; 32];
    OsRng.fill_bytes(&mut client_random);

    // Build and send ClientHello
    let mut ch_payload = Vec::with_capacity(64);
    ch_payload.extend_from_slice(&client_random);
    ch_payload.extend_from_slice(client_pub.as_bytes());

    let ch_bytes = encode_handshake_frame(HS_CLIENT_HELLO, &ch_payload);
    sender
        .send(Message::Binary(ch_bytes.clone().into()))
        .await
        .map_err(|e| anyhow!("failed to send ClientHello: {}", e))?;

    // Receive ServerHello
    let (sh_type, sh_payload, sh_bytes) = recv_handshake_frame(receiver)
        .await
        .context("waiting for ServerHello")?;

    if sh_type != HS_SERVER_HELLO {
        bail!("expected ServerHello, got hs_type={}", sh_type);
    }
    if sh_payload.len() != 64 {
        bail!("ServerHello payload wrong size");
    }

    let _server_random = &sh_payload[0..32];
    let server_pub_bytes: [u8; 32] = sh_payload
        .get(32..64)
        .ok_or_else(|| anyhow!("server_hello payload too short for pubkey"))?
        .try_into()
        .map_err(|_| anyhow!("server pubkey wrong length"))?;
    let server_pub = PublicKey::from(server_pub_bytes);

    // Build transcript
    let mut transcript = Vec::new();
    transcript.extend_from_slice(&ch_bytes);
    transcript.extend_from_slice(&sh_bytes);

    // Compute shared secret
    let shared = client_secret.diffie_hellman(&server_pub);
    let mut handshake_key = hkdf_expand(shared.as_bytes(), b"bearshare handshake key", 32)?;

    // Send ClientFinished
    let client_finished = finished_mac(&handshake_key, &transcript)?;
    let cf_bytes = encode_handshake_frame(HS_CLIENT_FINISHED, &client_finished);
    sender
        .send(Message::Binary(cf_bytes.clone().into()))
        .await
        .map_err(|e| anyhow!("failed to send ClientFinished: {}", e))?;

    transcript.extend_from_slice(&cf_bytes);

    // Receive ServerFinished
    let (sf_type, sf_payload, sf_bytes) = recv_handshake_frame(receiver)
        .await
        .context("waiting for ServerFinished")?;

    if sf_type != HS_SERVER_FINISHED {
        handshake_key.zeroize();
        bail!("expected ServerFinished, got hs_type={}", sf_type);
    }
    if sf_payload.len() != 32 {
        handshake_key.zeroize();
        bail!("ServerFinished wrong size");
    }

    // Verify server finished
    {
        let th = Sha256::digest(&transcript);
        let mut mac = <Hmac<Sha256> as hmac::Mac>::new_from_slice(&handshake_key)
            .map_err(|_| anyhow!("bad hmac key"))?;
        mac.update(&th);
        mac.verify_slice(&sf_payload)
            .map_err(|_| anyhow!("ServerFinished verify failed"))?;
    }

    transcript.extend_from_slice(&sf_bytes);

    // Derive application keys (client writes with c2s, reads with s2c)
    let mut c2s_key = hkdf_expand(shared.as_bytes(), b"bearshare app c2s key", 32)?;
    let mut s2c_key = hkdf_expand(shared.as_bytes(), b"bearshare app s2c key", 32)?;

    let th = Sha256::digest(&transcript);
    xor_in_place(&mut c2s_key, &th)?;
    xor_in_place(&mut s2c_key, &th)?;

    handshake_key.zeroize();

    // Client writes with c2s key, reads with s2c key
    let write = SecureWrite {
        cipher: ChaCha20Poly1305::new_from_slice(&c2s_key)
            .map_err(|_| anyhow!("bad c2s key"))?,
        send_seq: 0,
    };

    let read = SecureRead {
        cipher: ChaCha20Poly1305::new_from_slice(&s2c_key)
            .map_err(|_| anyhow!("bad s2c key"))?,
        recv_seq: 0,
    };

    c2s_key.zeroize();
    s2c_key.zeroize();

    Ok((write, read))
}

async fn recv_handshake_frame<R, E>(receiver: &mut R) -> Result<(u8, Vec<u8>, Vec<u8>)>
where
    R: Stream<Item = std::result::Result<Message, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    let msg = receiver
        .next()
        .await
        .ok_or_else(|| anyhow!("socket closed during handshake"))?
        .map_err(|e| anyhow!("ws receive error during handshake: {}", e))?;

    let Message::Binary(bytes) = msg else {
        bail!("expected Binary handshake frame");
    };

    let bytes_vec = bytes.to_vec();
    let (hs_type, payload) = decode_handshake_frame(&bytes_vec)?;
    Ok((hs_type, payload, bytes_vec))
}

fn encode_handshake_frame(hs_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HS_HEADER_LEN + payload.len());
    out.extend_from_slice(&HS_MAGIC);
    out.extend_from_slice(&VERSION.to_be_bytes());
    out.push(hs_type);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn decode_handshake_frame(frame: &[u8]) -> Result<(u8, Vec<u8>)> {
    if frame.len() < HS_HEADER_LEN {
        bail!("handshake frame too short");
    }
    if &frame[0..4] != HS_MAGIC {
        bail!("bad handshake magic");
    }
    let version = u16::from_be_bytes([frame[4], frame[5]]);
    if version != VERSION {
        bail!("unsupported handshake version: {}", version);
    }
    let hs_type = frame[6];
    let payload_len = u32::from_be_bytes(frame[7..11].try_into().unwrap()) as usize;
    if frame.len() != HS_HEADER_LEN + payload_len {
        bail!("handshake payload length mismatch");
    }
    Ok((hs_type, frame[11..].to_vec()))
}

fn hkdf_expand(ikm: &[u8], info: &[u8], out_len: usize) -> Result<Vec<u8>> {
    use hkdf::Hkdf;
    let hk = Hkdf::<Sha256>::new(None, ikm);
    let mut out = vec![0u8; out_len];
    hk.expand(info, &mut out)
        .map_err(|_| anyhow!("hkdf expand failed"))?;
    Ok(out)
}

fn finished_mac(handshake_key: &[u8], transcript: &[u8]) -> Result<Vec<u8>> {
    let th = Sha256::digest(transcript);
    let mut mac = <Hmac<Sha256> as hmac::Mac>::new_from_slice(handshake_key)
        .map_err(|_| anyhow!("bad hmac key"))?;
    mac.update(&th);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn xor_in_place(dst: &mut [u8], src: &[u8]) -> Result<()> {
    if src.len() < dst.len() {
        bail!("xor source too short");
    }
    for i in 0..dst.len() {
        dst[i] ^= src[i];
    }
    Ok(())
}
