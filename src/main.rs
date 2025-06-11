use std::fs;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kos::gzip::Decoder;
use kos::huffman::Huffman;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Extract,
    Compress,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Extract) => {
            let input = fs::read("./test.txt.gz").context("read test file")?;
            let mut decoder = Decoder::new(&input);
            match decoder.parse_header() {
                Ok(()) => println!(
                    "Name: {}, ModTime: {}, OS: {}",
                    decoder.header.name, decoder.header.modtime, decoder.header.os
                ),
                Err(e) => return Err(e),
            };
        }
        Some(Commands::Compress) => {
            let huffman = Huffman::from("A_DEAD_DAD_CEDED_A_BAD_BABE_A_BEADED_ABACA_BED");
            let encoded = huffman.encode();
            println!("{encoded}");
            println!("{}", huffman.decode(&encoded));
        }
        _ => {
            println!("this can happen?");
        }
    }

    Ok(())
}
