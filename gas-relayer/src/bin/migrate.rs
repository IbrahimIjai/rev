use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    cli::run_cli(gas_relayer::migration::Migrator).await;
}
