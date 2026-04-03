mod backend;
mod cli;
mod config;
mod counter_api;
mod doctor;
mod guest;
mod kvm;
mod launcher;
mod net;
mod qemu;
mod wizard;

use std::process::ExitCode;

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,minivm=debug".into()),
        )
        .with_target(false)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    match cli::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
