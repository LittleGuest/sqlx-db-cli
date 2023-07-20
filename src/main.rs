use clap::Parser;
use sqlx_db_cli::Generator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut gen = Generator::parse();
    gen.run().await?;
    Ok(())
}
