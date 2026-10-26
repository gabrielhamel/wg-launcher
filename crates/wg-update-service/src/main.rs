use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, process::ExitCode};
use wg_update_service::{FileService, FileServiceOptions};

#[derive(Debug, Parser)]
#[command(
    name = "wg-update-service",
    version,
    about = "Serve a file-backed WGUS-compatible game release"
)]
struct Args {
    /// Directory containing metadata.xml, patches_chain.xml, and packages/.
    #[arg(long, value_name = "DIRECTORY")]
    input: PathBuf,

    /// TCP address on which the service listens.
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,

    /// URL path prefix used by Product.host. Use an empty value for no prefix.
    #[arg(long, default_value = "updates")]
    prefix: String,

    /// Public origin inserted for __BASE_URL__ (defaults to the listen address).
    #[arg(long, value_name = "URL")]
    public_url: Option<String>,

    /// Append one tab-separated line per HTTP request to this file.
    #[arg(long, value_name = "FILE")]
    access_log: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let options = FileServiceOptions {
        input: args.input,
        listen: args.listen,
        prefix: args.prefix,
        public_url: args.public_url,
        access_log: args.access_log,
    };
    match FileService::start(options) {
        Ok(service) => {
            println!("WG update service listening on {}", service.url());
            println!("Product host: {}", service.product_host());
            std::thread::park();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("wg-update-service: {error}");
            ExitCode::FAILURE
        }
    }
}
