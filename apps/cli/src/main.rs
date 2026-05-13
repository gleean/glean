//! Binary entrypoint for `glean`.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    glean_cli::run().await
}
