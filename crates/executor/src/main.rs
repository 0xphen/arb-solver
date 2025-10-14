pub mod config;
pub mod csv_streamer;
pub mod error;
pub mod producer;
pub mod searcher;
pub mod sim_streamer;
pub mod types;
pub mod writer;

use std::env;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, mpsc::Sender};
use tokio::task::JoinHandle;

use arb_solver_core::GraphCSR;
use arb_solver_core::solver::SPFASolver;
use common::types::Edge;
use csv_streamer::CsvStreamer;
use producer::Producer;
use searcher::ArbSearcher;
use sim_streamer::SimulatorStreamer;
use types::{DataSource, JoinHandleResult, SharedGraph};
use writer::Writer;

const REBUILD_LIMIT: usize = 100;

#[tokio::main]
async fn main() {
    let source = parse_args();
    let config = config::load_config().expect("Failed to load config");

    let shared_graph = Arc::new(RwLock::new(GraphCSR::from_edges(0, &mut [], REBUILD_LIMIT)));

    let (sender, receiver) = mpsc::channel::<Vec<Edge>>(config.executor.buffer_size);

    // Spawn tasks
    let producer_handle = spawn_producer(&source, sender, &config);
    let writer_handle = spawn_writer(shared_graph.clone(), receiver, config.writer.batch_capacity);
    let searcher_handle = spawn_searcher(shared_graph.clone(), config.searcher.interval_seconds);

    let _ = tokio::join!(writer_handle, searcher_handle, producer_handle);

    println!("Pipeline shut down.");
}

/// Parse command-line arguments to determine data source
fn parse_args() -> DataSource {
    let args: Vec<String> = env::args().collect();
    let source = args
        .get(1)
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "sim".to_string());

    match source.as_str() {
        "sim" => DataSource::SIM,
        "csv" => {
            let path = args.get(2).expect("CSV path required for CSV mode").clone();
            DataSource::CSV(path)
        }
        _ => {
            eprintln!(
                "Usage: {} <SIM|CSV> [path_to_csv]\n  - SIM: run simulated data stream\n  - CSV: read updates from a CSV file",
                args[0]
            );
            std::process::exit(1);
        }
    }
}

pub fn spawn_producer(
    source: &DataSource,
    sender: Sender<Vec<Edge>>,
    config: &config::Config,
) -> JoinHandle<()> {
    match source {
        DataSource::SIM => {
            println!("Starting SimulatorStreamer producer task...");
            let streamer = SimulatorStreamer::new(config.simulator.clone());
            let producer = Producer::new(streamer);
            producer.spawn(sender)
        }
        DataSource::CSV(path) => {
            println!("Starting CsvStreamer producer task...");
            let streamer = CsvStreamer::new(path.clone(), config.producer.batch_size);
            let producer = Producer::new(streamer);
            producer.spawn(sender)
        }
    }
}

/// Spawn writer task
fn spawn_writer(
    shared_graph: SharedGraph,
    receiver: mpsc::Receiver<Vec<Edge>>,
    batch_capacity: usize,
) -> JoinHandleResult {
    let writer = Writer::new(shared_graph, receiver, batch_capacity);
    tokio::spawn(writer.process_updates())
}

/// Spawn searcher task
fn spawn_searcher(shared_graph: Arc<RwLock<GraphCSR>>, interval_seconds: u64) -> JoinHandleResult {
    let searcher = ArbSearcher::new(shared_graph, interval_seconds, SPFASolver);
    tokio::spawn(async move { searcher.seacrh_for_arbs().await })
}
