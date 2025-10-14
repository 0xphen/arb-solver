pub mod config;
pub mod error;
pub mod producer;
pub mod searcher;
pub mod simulator;
pub mod types;
pub mod writer;

use ::config::Config;
use arb_solver_core::GraphCSR;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, watch};

use arb_solver_core::solver::SPFASolver;
use config::load_config;
use producer::Producer;
use searcher::ArbSearcher;
use simulator::SimulatorStreamer;
use writer::Writer;

const CHANNEL_CAPACITY: usize = 10;

#[tokio::main]
async fn main() {
    let (sender, receiver) = mpsc::channel::<Vec<common::types::Edge>>(CHANNEL_CAPACITY);
    println!("Starting Pipeline with capacity: {}", CHANNEL_CAPACITY);

    let graph = GraphCSR::from_edges(0, &mut [], 1);
    let shared_graph = Arc::new(RwLock::new(graph));

    let config = load_config().expect("Failed to load config");

    let sim = SimulatorStreamer::new(config.simulator);

    let producer = Producer::new(sim);

    let (_shutdown_tx, shutdown_rx) = watch::channel(());
    let writer = Writer::new(Arc::clone(&shared_graph), receiver, shutdown_rx, 50);

    let searcher = ArbSearcher::new(
        Arc::clone(&shared_graph),
        config.searcher.interval_seconds,
        SPFASolver,
    );

    // Spawn the tasks
    let producer_handle = tokio::spawn(producer.run(sender));

    let writer_handle = tokio::spawn(writer.spawn_task());

    let searcher_handle = tokio::spawn(async move { searcher.seacrh_for_arbs().await });

    let _ = tokio::join!(producer_handle, writer_handle, searcher_handle);

    println!("Pipeline shut down.");
}
