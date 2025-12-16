// Plot storage management
// This module handles storing and retrieving plot data

use crate::messages::plot::PlotData;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Plot storage structure
#[derive(Clone)]
pub struct PlotStorage {
    plots: Arc<Mutex<HashMap<String, PlotData>>>,
}

impl PlotStorage {
    /// Create a new PlotStorage instance
    pub fn new() -> Self {
        Self {
            plots: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add or update a plot
    pub async fn insert(&self, plot: PlotData) {
        let mut plots_guard = self.plots.lock().await;
        plots_guard.insert(plot.id.clone(), plot);
    }

    /// Get a plot by ID
    #[allow(dead_code)]
    pub async fn get(&self, plot_id: &str) -> Option<PlotData> {
        let plots_guard = self.plots.lock().await;
        plots_guard.get(plot_id).cloned()
    }

    /// Get all plots
    pub async fn get_all(&self) -> Vec<PlotData> {
        let plots_guard = self.plots.lock().await;
        plots_guard.values().cloned().collect()
    }

    /// Remove a plot by ID
    pub async fn remove(&self, plot_id: &str) -> bool {
        let mut plots_guard = self.plots.lock().await;
        plots_guard.remove(plot_id).is_some()
    }

    /// Clear all plots
    pub async fn clear(&self) {
        let mut plots_guard = self.plots.lock().await;
        plots_guard.clear();
    }

    /// Get a clone of the plots HashMap for use in HTTP handlers
    pub fn clone_plots(&self) -> Arc<Mutex<HashMap<String, PlotData>>> {
        self.plots.clone()
    }
}

impl Default for PlotStorage {
    fn default() -> Self {
        Self::new()
    }
}

