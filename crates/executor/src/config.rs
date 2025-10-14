use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct SearcherConfig {
    pub interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SimulatorConfig {
    pub total_nodes: usize,
    pub batch_size: usize,
    pub simulation_interval_ms: u64,
    pub rate_fluctuation_bps: f64, // Base points to actual float conversion
    pub rebuild_limit: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub searcher: SearcherConfig,
    pub simulator: SimulatorConfig,
}