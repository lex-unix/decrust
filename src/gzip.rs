use anyhow::{self, Result, ensure};

const ID1: u8 = 0x1F;
const ID2: u8 = 0x8B;
const COMPRESSION_METHOD: u8 = 0x8;

// const FTEXT: u8 = 1 << 0;
const FHCRC: u8 = 1 << 1;
const FEXTRA: u8 = 1 << 2;
const FNAME: u8 = 1 << 3;
const FCOMMENT: u8 = 1 << 4;

#[derive(Debug, Default)]
pub struct Header {
    pub comment: String,
    pub extra: Vec<u8>,
    pub modtime: u32,
    pub name: String,
    pub os: u8,
}

#[derive(Debug)]
pub struct Decoder<'a> {
    pub header: Header,
    pub pos: usize,
    pub input_stream: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            header: Header::default(),
            input_stream: input,
            pos: 0,
        }
    }

    pub fn parse_header(&mut self) -> Result<()> {
        if self.pos != 0 {
            // header already parsed
            anyhow::bail!("Header already parsed");
        }

        anyhow::ensure!(
            self.input_stream.len() >= 10,
            "Input too short for gzip header"
        );

        let id1 = self.read_byte()?;
        ensure!(
            id1 == ID1,
            "Invalid gzip magic byte 1: expected {:#x}, got {:#x}",
            ID1,
            id1
        );

        let id2 = self.read_byte()?;
        ensure!(
            id2 == ID2,
            "Invalid gzip magic byte 2: expected {:#x}, got {:#x}",
            ID2,
            id2
        );

        let cm = self.read_byte()?;
        ensure!(
            cm == COMPRESSION_METHOD,
            "Unsupported compression method: {}",
            cm
        );

        let flags = self.read_byte()?;

        let mtime_bytes = self.read_bytes(4)?;
        self.header.modtime = u32::from_le_bytes([
            mtime_bytes[0],
            mtime_bytes[1],
            mtime_bytes[2],
            mtime_bytes[3],
        ]);

        // TODO: look at RFC how extra flags are used for decompression
        // XFL = 2 - used maximum compression
        // XFL = 4 - used fastest compression
        let _xfl = self.read_byte()?;

        self.header.os = self.read_byte()?;

        if flags & FEXTRA != 0 {
            let xlen_bytes = self.read_bytes(2)?;
            let xlen = u16::from_le_bytes([xlen_bytes[0], xlen_bytes[1]]) as usize;
            self.header.extra = self.read_bytes(xlen)?.to_vec();
        }

        if flags & FNAME != 0 {
            self.header.name = self.read_null_terminated()?;
        }

        if flags & FCOMMENT != 0 {
            self.header.comment = self.read_null_terminated()?;
        }

        if flags & FHCRC != 0 {
            // TODO: CRC16
            let _crc16 = self.read_bytes(2)?;
        }

        // NOTE: following is compressed data, CRC32, and ISIZE

        Ok(())
    }

    fn read_byte(&mut self) -> Result<u8> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    fn read_bytes(&mut self, count: usize) -> Result<&[u8]> {
        ensure!(
            self.pos + count <= self.input_stream.len(),
            "Unexpected EOF"
        );
        let bytes = &self.input_stream[self.pos..self.pos + count];
        self.pos += count;
        Ok(bytes)
    }

    fn read_null_terminated(&mut self) -> Result<String> {
        let mut bytes = Vec::new();
        loop {
            let byte = self.read_byte()?;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("Invalid UTF-8 in string: {}", e))
    }
}
