use std::fs;

use anyhow::{Context, Result};
use encrust::gzip::Decoder;

fn main() -> Result<()> {
    let Some(filename) = std::env::args().nth(1) else {
        return Err(anyhow::anyhow!("file must be provided"));
    };

    let input = fs::read(filename).context("read provided file")?;
    let mut decoder = Decoder::new(&input);
    match decoder.decode() {
        Ok(output) => {
            if let Ok(decoded) = String::from_utf8(output) {
                println!("Got decoded contents:\n{decoded}");
            } else {
                println!("invalid utf-8 string");
            }
            println!(
                "Parsed Header:\nName: {}, ModTime: {}, OS: {}, Extra: {:?}, Comment: {}",
                decoder.header.name,
                decoder.header.modtime,
                decoder.header.os,
                decoder.header.extra,
                decoder.header.comment
            );
        }
        Err(e) => return Err(e),
    };

    Ok(())
}
