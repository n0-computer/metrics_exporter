use std::{path::PathBuf, time::Duration};

use anyhow::{Error, Result};
use bytes::Bytes;
use clap::{Parser, Subcommand};
use metrics_exporter::config::Config;
use tokio::sync::mpsc;
use tracing::{debug, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(arg_required_else_help = true)]
    Run { cfg: Option<PathBuf> },
    #[clap(arg_required_else_help = false)]
    RunWithoutConfig {
        scrape_addr: Option<String>,
        push_addr: Option<String>,
        instance: Option<String>,
        job: Option<String>,
        push_interval: Option<u64>,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    match args.command {
        Commands::Run { cfg } => {
            let cfg = cfg.unwrap_or_else(|| PathBuf::from("me_config.toml"));
            let cfg = Config::new_from_cfg(cfg).unwrap();
            run_exporter(cfg).await?;
        }
        Commands::RunWithoutConfig {
            scrape_addr,
            push_addr,
            instance,
            job,
            push_interval,
        } => {
            let cfg = Config::new_from_args(scrape_addr, push_addr, instance, job, push_interval);
            run_exporter(cfg).await?;
        }
    }
    Ok(())
}

async fn scrape(scrape_endpoint: &str) -> Result<Bytes, Error> {
    let resp = reqwest::get(scrape_endpoint).await?.bytes().await?;
    Ok(resp)
}

async fn push(push_endpoint: &str, buff: Bytes, push_client: &reqwest::Client) {
    let res = match push_client.post(push_endpoint).body(buff).send().await {
        Ok(res) => res,
        Err(e) => {
            warn!("failed to push metrics: {}", e);
            return;
        }
    };
    match res.status() {
        reqwest::StatusCode::OK => {
            debug!("pushed metrics to gateway");
        }
        _ => {
            warn!("failed to push metrics to gateway: {:?}", res);
            let body = res.text().await.unwrap();
            warn!("error body: {}", body);
        }
    }
}

async fn run_exporter(cfg: Config) -> Result<()> {
    let push_endpoint = format!(
        "{}/metrics/job/{}/instance/{}",
        cfg.push_addr, cfg.job, cfg.instance
    );
    let (tx, mut rx) = mpsc::channel(32);

    println!("Starting exporter...");
    println!("{}", cfg);

    tokio::spawn(async move {
        let push_client = reqwest::Client::new();
        loop {
            let buff = rx.recv().await.unwrap();
            push(&push_endpoint, buff, &push_client).await;
        }
    });
    loop {
        tokio::time::sleep(Duration::from_secs(cfg.scrape_interval)).await;
        let buff = match scrape(&cfg.scrape_addr).await {
            Ok(buff) => buff,
            Err(e) => {
                warn!("failed to scrape metrics: {}", e);
                continue;
            }
        };
        let t = tx.clone();
        tokio::spawn(async move {
            t.send(buff).await.unwrap();
        });
    }
}
