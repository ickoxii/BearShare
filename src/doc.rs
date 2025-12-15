use std::io::{self, Read, Write};

#[derive(Debug, Clone)]
pub enum Op {
    Insert { pos: usize, text: String },
    Delete { pos: usize, len: usize },
}

#[derive(Debug, Clone)]
pub struct Document {
    text: String,
    ops: Vec<Op>,
}

impl Document {
    pub fn new() -> Document {
        Document {
            text: String::new(),
            ops: Vec::new(),
        }
    }

    pub fn from_text(s: String) -> Document {
        Document { text: s, ops: Vec::new() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn ops(&self) -> &Vec<Op> {
        &self.ops
    }

    // Editing operations
    pub fn insert(&mut self, pos: usize, s: &str) -> io::Result<()> {
        if pos > self.text.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "insert pos out of bounds"));
        }
        self.text.insert_str(pos, s);
        self.ops.push(Op::Insert { pos, text: s.to_string() });
        Ok(())
    }

    pub fn delete(&mut self, pos: usize, len: usize) -> io::Result<()> {
        if pos > self.text.len() || pos + len > self.text.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "delete range out of bounds"));
        }
        self.text.replace_range(pos..pos+len, "");
        self.ops.push(Op::Delete { pos, len });
        Ok(())
    }

    // Replay ops into a fresh document (useful for later syncing)
    pub fn apply_ops(base_text: String, ops: &[Op]) -> io::Result<Document> {
        let mut d = Document::from_text(base_text);
        for op in ops {
            match op {
                Op::Insert { pos, text } => d.insert(*pos, text)?,
                Op::Delete { pos, len } => d.delete(*pos, *len)?,
            }
        }
        Ok(d)
    }

    // ---- persistence (manual binary format) ----
    //
    // File format:
    // [u32 text_len][text bytes]
    // [u32 ops_count]
    // repeated ops:
    //   [u8 tag] 0=Insert,1=Delete
    //   Insert:
    //     [u32 pos][u32 str_len][str bytes]
    //   Delete:
    //     [u32 pos][u32 len]
    //
    pub fn save<W: Write>(&self, mut w: W) -> io::Result<()> {
        // text
        write_u32(&mut w, self.text.len() as u32)?;
        w.write_all(self.text.as_bytes())?;

        // ops
        write_u32(&mut w, self.ops.len() as u32)?;
        for op in &self.ops {
            match op {
                Op::Insert { pos, text } => {
                    w.write_all(&[0u8])?;
                    write_u32(&mut w, *pos as u32)?;
                    write_u32(&mut w, text.len() as u32)?;
                    w.write_all(text.as_bytes())?;
                }
                Op::Delete { pos, len } => {
                    w.write_all(&[1u8])?;
                    write_u32(&mut w, *pos as u32)?;
                    write_u32(&mut w, *len as u32)?;
                }
            }
        }
        Ok(())
    }

    pub fn load<R: Read>(mut r: R) -> io::Result<Document> {
        // text
        let text_len = read_u32(&mut r)? as usize;
        let mut text_buf = vec![0u8; text_len];
        r.read_exact(&mut text_buf)?;
        let text = String::from_utf8(text_buf)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "text not utf8"))?;

        // ops
        let ops_count = read_u32(&mut r)? as usize;
        let mut ops = Vec::with_capacity(ops_count);

        for _ in 0..ops_count {
            let mut tag = [0u8; 1];
            r.read_exact(&mut tag)?;
            match tag[0] {
                0 => {
                    let pos = read_u32(&mut r)? as usize;
                    let slen = read_u32(&mut r)? as usize;
                    let mut sbuf = vec![0u8; slen];
                    r.read_exact(&mut sbuf)?;
                    let s = String::from_utf8(sbuf)
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "insert text not utf8"))?;
                    ops.push(Op::Insert { pos, text: s });
                }
                1 => {
                    let pos = read_u32(&mut r)? as usize;
                    let len = read_u32(&mut r)? as usize;
                    ops.push(Op::Delete { pos, len });
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "bad op tag")),
            }
        }

        Ok(Document { text, ops })
    }
}

impl Op {
    // Encode a single Op into bytes, same tags as your file format.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Op::Insert { pos, text } => {
                out.push(0u8);
                out.extend_from_slice(&(*pos as u32).to_le_bytes());
                out.extend_from_slice(&(text.len() as u32).to_le_bytes());
                out.extend_from_slice(text.as_bytes());
            }
            Op::Delete { pos, len } => {
                out.push(1u8);
                out.extend_from_slice(&(*pos as u32).to_le_bytes());
                out.extend_from_slice(&(*len as u32).to_le_bytes());
            }
        }
        out
    }

    // Decode exactly one Op from bytes.
    pub fn from_bytes(bytes: &[u8]) -> std::io::Result<Op> {
        use std::io::{Error, ErrorKind};

        if bytes.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "empty op"));
        }
        let tag = bytes[0];

        let read_u32 = |i: usize| -> std::io::Result<u32> {
            if i + 4 > bytes.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "truncated u32"));
            }
            let mut b = [0u8; 4];
            b.copy_from_slice(&bytes[i..i+4]);
            Ok(u32::from_le_bytes(b))
        };

        match tag {
            0 => {
                let pos = read_u32(1)? as usize;
                let slen = read_u32(5)? as usize;
                let start = 9;
                let end = start + slen;
                if end > bytes.len() {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "truncated string"));
                }
                let s = std::str::from_utf8(&bytes[start..end])
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "utf8"))?
                    .to_string();
                Ok(Op::Insert { pos, text: s })
            }
            1 => {
                let pos = read_u32(1)? as usize;
                let len = read_u32(5)? as usize;
                Ok(Op::Delete { pos, len })
            }
            _ => Err(Error::new(ErrorKind::InvalidData, "bad op tag")),
        }
    }
}


// Little endian helpers

fn write_u32<W: Write>(w: &mut W, x: u32) -> io::Result<()> {
    w.write_all(&x.to_le_bytes())
}

fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}
