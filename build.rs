use anyhow::{Ok, Result};
use substreams_ethereum::Abigen;

fn main() -> Result<(), anyhow::Error> {
    Abigen::new("ERC1155", "abi/erc1155.json")?
        .generate()?
        .write_to_file("src/abi/erc1155.rs")?;

    Ok(())
}
