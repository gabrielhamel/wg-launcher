mod catalog;
mod cli;
mod installation;
mod self_update;
mod terminal;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    cli::run().await
}
