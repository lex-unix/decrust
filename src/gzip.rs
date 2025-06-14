use anyhow::{self, Result, bail, ensure};

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

        // NOTE: rest of the stream is compressed data, CRC32, and ISIZE

        Ok(())
    }

    pub fn decode(&mut self) -> Result<Vec<u8>> {
        self.parse_header()?;

        let mut bitstream = BitStream::new(&self.input_stream[self.pos..]);

        infalte(&mut bitstream)
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

// Most of the following code is adapted from madler/zlib
// Available at: https://github.com/madler/zlib/blob/master/contrib/puff/puff.c

fn infalte(bitstream: &mut BitStream) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    loop {
        let is_final = bitstream.read(1)? == 1;
        let block_type = bitstream.read(2)?;

        match block_type {
            0 => uncompressed(bitstream, &mut output)?,
            1 => huff_fixed(bitstream, &mut output)?,
            2 => huff_dynamic(bitstream, &mut output)?,
            _ => bail!("reserved block type"),
        };

        if is_final {
            break;
        }
    }

    Ok(output)
}

fn uncompressed(bitstream: &mut BitStream, output: &mut Vec<u8>) -> Result<()> {
    bitstream.discard();

    let lencom = bitstream.get_bytes(4)?;
    let len = &lencom[..2];
    let com = &lencom[2..];

    let len = u16::from_le_bytes([len[0], len[1]]);
    let com = u16::from_le_bytes([com[0], com[1]]);

    ensure!(com == !len, "one's complement verification failed");

    let data = bitstream.get_bytes(len as usize)?;
    output.extend_from_slice(data);

    Ok(())
}

#[derive(Debug, Default)]
struct Huffman {
    symbols: Vec<u32>,
    count: Vec<u32>,
}

fn huff_fixed(bitstream: &mut BitStream, output: &mut Vec<u8>) -> Result<()> {
    let mut lengths: [u32; 288] = [0; 288];
    lengths[..=143].fill(8);
    lengths[144..=255].fill(9);
    lengths[256..=279].fill(7);
    lengths[280..=287].fill(8);

    let distances: [u32; 30] = [5; 30];

    let len_huff = huff_table(&lengths);
    let dist_huff = huff_table(&distances);

    codes(bitstream, output, &len_huff, &dist_huff)
}

fn huff_dynamic(bitstream: &mut BitStream, output: &mut Vec<u8>) -> Result<()> {
    let order: [u16; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];

    let mut lengths: [u32; 19] = [0; 19];

    let hlit = bitstream.read(5)? + 257;
    let hdist = bitstream.read(5)? + 1;
    let hclen = bitstream.read(4)? + 4;

    for i in 0..hclen {
        lengths[order[i as usize] as usize] = bitstream.read(3)?;
    }
    for i in hclen..19 {
        lengths[order[i as usize] as usize] = 0;
    }

    let lencode_huff = huff_table(&lengths);
    let mut lengths: [u32; 316] = [0; 316];
    let mut index: u32 = 0;

    while index < hlit + hdist {
        let mut symbol = decode(bitstream, &lencode_huff)?;
        if symbol < 16 {
            lengths[index as usize] = symbol;
            index += 1;
        } else {
            let mut len = 0;
            match symbol {
                16 => {
                    len = lengths[(index - 1) as usize];
                    symbol = 3 + bitstream.read(2)?;
                }
                17 => {
                    symbol = 3 + bitstream.read(3)?;
                }
                _ => {
                    symbol = 11 + bitstream.read(7)?;
                }
            }
            while symbol != 0 {
                lengths[index as usize] = len;
                symbol -= 1;
                index += 1;
            }
        }
    }

    let len_huff = huff_table(&lengths[..hlit as usize]);
    let dist_huff = huff_table(&lengths[(hlit as usize)..]);

    codes(bitstream, output, &len_huff, &dist_huff)
}

fn codes(
    bitstream: &mut BitStream,
    output: &mut Vec<u8>,
    len_huff: &Huffman,
    dist_huff: &Huffman,
) -> Result<()> {
    let lens = [
        /* Size base for length codes 257..285 */
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    let lext = [
        /* Extra bits for length codes 257..285 */
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    let dists = [
        /* Offset base for distance codes 0..29 */
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    let dext = [
        /* Extra bits for distance codes 0..29 */
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    loop {
        let symbol = decode(bitstream, &len_huff)?;
        match symbol {
            0..256 => {
                output.push(symbol as u8);
            }
            257..285 => {
                let symbol = symbol - 257;
                ensure!(symbol < 29, "invalid symbol len");
                let mut len = lens[symbol as usize] + bitstream.read(lext[symbol as usize])?;
                let symbol = decode(bitstream, &dist_huff)?;
                let dist = dists[symbol as usize] + bitstream.read(dext[symbol as usize])?;
                ensure!((dist as usize) < output.len(), "invalid len symbol");
                while len > 0 {
                    let literal = output[output.len() - (dist as usize)];
                    output.push(literal);
                    len -= 1;
                }
            }
            256 => break,
            _ => unreachable!("shouldn't happen"),
        }
    }

    Ok(())
}

fn huff_table(code_lengths: &[u32]) -> Huffman {
    let mut huff = Huffman::default();
    huff.count.resize(16, 0);
    huff.symbols.resize(288, 0);

    for symbol in 0..code_lengths.len() {
        let index = code_lengths[symbol];
        huff.count[index as usize] += 1;
    }

    let mut offsets = [0; 16];
    for i in 1..15 {
        offsets[i + 1] = offsets[i] + huff.count[i];
    }

    for symbol in 0..code_lengths.len() {
        if code_lengths[symbol] != 0 {
            let symbol_len = code_lengths[symbol];
            let offset = offsets[symbol_len as usize];
            huff.symbols[offset as usize] = symbol as u32;
            offsets[symbol_len as usize] += 1;
        }
    }

    huff
}

fn decode(bitstream: &mut BitStream, huff: &Huffman) -> Result<u32, anyhow::Error> {
    let mut code: u32 = 0;
    let mut first: u32 = 0;
    let mut index: u32 = 0;

    for len in 1..=15 {
        // Read the next bit and append it to the current code
        code |= bitstream.read(1)? as u32;

        let count = huff.count[len] as u32;

        if code - first < count {
            let symbol_index = index + (code - first);
            let result = huff.symbols[symbol_index as usize];
            return Ok(result);
        }

        index += count;
        first += count;
        first <<= 1;
        code <<= 1;
    }

    Err(anyhow::anyhow!("unable to decode"))
}

struct BitStream<'a> {
    bytes: &'a [u8],
    byte_pos: usize,
    buf: u32,
    bits_in_buf: u32,
}

impl<'a> BitStream<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_pos: 0,
            buf: 0,
            bits_in_buf: 0,
        }
    }

    fn read(&mut self, need: u32) -> Result<u32> {
        let mut val = self.buf;

        while self.bits_in_buf < need {
            if self.byte_pos == self.bytes.len() {
                bail!(
                    "Unexpected EOF: total bytes - {}, byte_pos - {}, need - {}",
                    self.bytes.len(),
                    self.byte_pos,
                    need
                )
            }
            val |= (self.bytes[self.byte_pos] as u32) << self.bits_in_buf;
            self.byte_pos += 1;
            self.bits_in_buf += 8;
        }

        self.buf = val >> need;
        self.bits_in_buf -= need;

        Ok(val & ((1 << need) - 1))
    }

    fn get_bytes(&mut self, need: usize) -> Result<&[u8]> {
        if self.byte_pos + need >= self.bytes.len() {
            return Err(anyhow::anyhow!("Unexpected EOF"));
        }

        let s = &self.bytes[self.byte_pos..self.byte_pos + need];
        self.byte_pos += need;

        Ok(s)
    }

    fn discard(&mut self) {
        self.buf = 0;
        self.bits_in_buf = 0;
    }
}
