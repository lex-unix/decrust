> [!WARNING]
> This crate was written for learning purposes and contains an untested, unoptimized version of the DEFLATE algorithm.
> For production use, consider well-tested libraries like `flate2`.

# `decrust`

A simple gzip decompression tool written in Rust, implementing the DEFLATE algorithm (decoding part) from scratch.

## Why?

The motivation is simply to learn new things. This is my first project written in Rust, and also the first time I delve into compression algorithms.

I've decided to implement the DEFLATE algorithm because I wanted to learn more about bit arithmetic and how to use it in real-world programs.

## What it does

This tool can decompress gzip files by implementing:

- Gzip header parsing (including optional fields like filename, comments, extra data)
- DEFLATE decompression with support for:
  - Uncompressed blocks
  - Fixed Huffman coding
  - Dynamic Huffman coding
- Bit-level stream reading

## Usage

The tool will decompress the gzip file and print:

- The decoded content (if it's valid UTF-8)
- Parsed header information (filename, modification time, OS, etc.)

## Implementation Notes

The DEFLATE implementation was referenced from [madler/zlib](https://github.com/madler/zlib/blob/master/contrib/puff/puff.c).

## Learning Resources

- [RFC 1951 - DEFLATE Specification](https://tools.ietf.org/html/rfc1951)
- [RFC 1952 - GZIP Specification](https://tools.ietf.org/html/rfc1952)
