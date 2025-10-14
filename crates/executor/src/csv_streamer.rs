use csv::ReaderBuilder;
use serde::Deserialize;
use std::fs::File;
use tokio::sync::mpsc::Sender;

use super::error::Error;
use super::types::UpdateStreamer;
use common::types::Edge;

// Helper struct for CSV parsing
#[derive(Debug, Deserialize, Default)]
pub struct CsvRecord {
    #[serde(rename = "from")]
    pub from_node: usize,

    #[serde(rename = "to")]
    pub to_node: usize,

    #[serde(rename = "rate")]
    pub rate_value: f64,
}

pub struct CsvStreamer {
    path: String,
    batch_size: usize,
}

impl CsvStreamer {
    pub fn new(path: String, batch_size: usize) -> Self {
        CsvStreamer { path, batch_size }
    }

    fn parse_csv_to_edges(&self) -> Result<Vec<Edge>, Error> {
        let file = File::open(&self.path).map_err(|e| {
            eprintln!("Failed to read file {}: {:?}", self.path, e);
            Error::IoError(e)
        })?;

        let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

        let mut edges = Vec::new();

        for result in rdr.deserialize() {
            let record: CsvRecord = result?;
            edges.push((record.from_node, record.to_node, record.rate_value));
        }
        Ok(edges)
    }
}

#[async_trait::async_trait]
impl UpdateStreamer for CsvStreamer {
    async fn run_stream(self, sender: Sender<Vec<Edge>>) -> Result<(), Error> {
        let all_edges = self.parse_csv_to_edges()?;
        let total_edges = all_edges.len();
        let mut edges_sent = 0;

        println!("CsvStreamer: Starting transfer of {} edges...", total_edges);

        for chunk in all_edges.chunks(self.batch_size) {
            let batch: Vec<Edge> = chunk.to_vec();
            if let Err(e) = sender.send(batch).await {
                eprintln!(
                    "CsvStreamer shutting down: Writer receiver dropped during send. Error: {}",
                    e
                );
                return Err(Error::ChannelSendFailed);
            }

            edges_sent += chunk.len();
        }

        println!(
            "CsvStreamer: Successfully transferred {} edges in batches.",
            edges_sent
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const MOCK_CSV_CONTENT: &str = "\
id,from,to,rate,pool_id,kind
1,0,1,1.05,10001,F
2,1,2,0.95,10002,F
3,2,0,1.001,10003,F
4,5,6,1.2,10004,F
";

    const BATCH_SIZE: usize = 10;

    #[test]
    fn test_parse_csv_to_edges_success() {
        // Create a temporary file with the mock content.
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(MOCK_CSV_CONTENT.as_bytes())
            .expect("Failed to write mock content");

        let path = temp_file
            .path()
            .to_str()
            .expect("Failed to get path string");

        let streamer = CsvStreamer::new(path.to_string(), BATCH_SIZE);
        let result = streamer.parse_csv_to_edges();

        assert!(
            result.is_ok(),
            "Parsing failed with error: {:?}",
            result.err()
        );

        let edges = result.unwrap();

        let expected_edges: Vec<Edge> =
            vec![(0, 1, 1.05), (1, 2, 0.95), (2, 0, 1.001), (5, 6, 1.2)];

        assert_eq!(edges.len(), 4, "Should have parsed 4 edges.");
        assert_eq!(
            edges, expected_edges,
            "Parsed edges do not match expected data."
        );
    }

    #[test]
    fn test_parse_csv_to_edges_file_not_found() {
        let streamer = CsvStreamer::new("non_existent_file.csv".to_string(), BATCH_SIZE);
        let result = streamer.parse_csv_to_edges();

        assert!(
            result.is_err(),
            "Should have failed to open non-existent file."
        );

        if let Err(Error::IoError(e)) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        } else {
            panic!("Expected IoError, got: {:?}", result.err());
        }
    }
}
