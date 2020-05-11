mod cli;
mod config;
mod state;
mod ui;

use crate::cli::*;
use crate::config::ShellConfig;
use crate::state::RemoteCluster;
use crate::ui::*;
use log::{debug, warn};
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::sync::Arc;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let config = ShellConfig::new();
    warn!("Config {:?}", config);

    let mut clusters = HashMap::new();

    let active = if config.clusters().is_empty() {
        let cluster = RemoteCluster::new(
            opt.connection_string,
            opt.username,
            opt.password,
            opt.bucket,
        );
        clusters.insert("default".into(), cluster);
        String::from("default")
    } else {
        let mut active = None;
        for (k, v) in config.clusters() {
            let name = k.clone();
            let cluster = RemoteCluster::new(
                v.connstr().into(),
                v.username().into(),
                v.password().into(),
                v.default_bucket(),
            );
            clusters.insert(name.clone(), cluster);
            if opt.cluster.as_ref().is_some() {
                if &name == opt.cluster.as_ref().unwrap() {
                    active = Some(name.clone())
                }
            } else if active.is_none() {
                active = Some(k.clone());
            }
        }
        active.unwrap()
    };

    let state = Arc::new(State::new(clusters, active));

    if opt.ui {
        tokio::task::spawn(spawn_and_serve(state.clone()));
    }

    let mut syncer = nu_cli::EnvironmentSyncer::new();
    let mut context = nu_cli::create_default_context(&mut syncer, true)?;
    context.add_commands(vec![
        // Performs analytics queries
        nu_cli::whole_stream_command(Analytics::new(state.clone())),
        // Performs kv get operations
        nu_cli::whole_stream_command(Get::new(state.clone())),
        // Performs kv upsert operations
        nu_cli::whole_stream_command(Upsert::new(state.clone())),
        // Performs kv insert operations
        nu_cli::whole_stream_command(Insert::new(state.clone())),
        // Performs kv replace operations
        nu_cli::whole_stream_command(Replace::new(state.clone())),
        // Displays cluster manager node infos
        nu_cli::whole_stream_command(Nodes::new(state.clone())),
        // Displays cluster manager bucket infos
        nu_cli::whole_stream_command(Buckets::new(state.clone())),
        // Performs n1ql queries
        nu_cli::whole_stream_command(Query::new(state.clone())),
        // Manages local cluster references
        nu_cli::whole_stream_command(Clusters::new(state.clone())),
        // Create fake data based on templates
        nu_cli::whole_stream_command(FakeData::new(state.clone())),
        // Displays indexes
        nu_cli::whole_stream_command(Indexes::new(state.clone())),
        // Allows to switch clusters, buckets and collections on the fly
        nu_cli::whole_stream_command(UseCmd::new(state.clone())),
        nu_cli::whole_stream_command(UseCluster::new(state.clone())),
    ]);

    if let Some(c) = opt.command {
        nu_cli::run_pipeline_standalone(c, opt.stdin, &mut context, true).await?;
        return Ok(());
    }

    if let Some(s) = opt.script {
        let file = File::open(s)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if !line.starts_with('#') {
                nu_cli::run_pipeline_standalone(line, opt.stdin, &mut context, true).await?;
            }
        }
        return Ok(());
    }

    nu_cli::cli(Some(syncer), Some(context)).await
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "The Couchbase Shell",
    about = "Alternative Shell and UI for Couchbase Server and Cloud"
)]
struct CliOptions {
    #[structopt(long = "connstring", default_value = "couchbase://localhost")]
    connection_string: String,
    #[structopt(long = "ui")]
    ui: bool,
    #[structopt(short = "u", long = "username", default_value = "Administrator")]
    username: String,
    #[structopt(short = "p", long = "password", default_value = "password")]
    password: String,
    #[structopt(long = "cluster")]
    cluster: Option<String>,
    #[structopt(long = "bucket")]
    bucket: Option<String>,
    #[structopt(long = "command", short = "c")]
    command: Option<String>,
    #[structopt(long = "script")]
    script: Option<String>,
    #[structopt(long = "stdin")]
    stdin: bool,
}
