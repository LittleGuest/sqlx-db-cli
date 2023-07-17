use std::error::Error;

use clap::Parser;
use sqlx_db_cli::Generator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut gen = Generator::parse();
    gen.run().await?;
    Ok(())
}
