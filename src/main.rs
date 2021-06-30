use reqwest::Client;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ReqwestError")]
    ReqwestError(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

fn create_client(timeout: Duration) -> Result<Client> {
    Client::builder()
        .gzip(true)
        .brotli(true)
        .timeout(timeout)
        .build()
        .map_err(Error::ReqwestError)
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = create_client(Duration::from_millis(5000))?;
    let res = client.get("https://example.com").send().await?.text().await?;
    println!("{}", res);
    Ok(())
}
