//! Guided orbital transfers with thrust-limited delta-v and deterministic snap completion.

use std::collections::HashMap;

use crate::orbits::{OrbitParams, OrbitType};
use crate::small_body::BodyId;

/// When remaining corrective delta-v falls below this magnitude, complete the transfer.
pub const TRANSFER_SNAP_EPSILON: f64 = 1e-6;

/// Maximum position adjustment allowed on completion; larger gaps are velocity-only snaps.
pub const TRANSFER_SNAP_POSITION_EPSILON: f64 = 1e-4;

/// Status of an in-progress transfer burn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferBurnStatus {
    /// No transfer burn active (or cleared after acknowledging completion).
    #[default]
    Idle,
    /// Guidance is applying delta-v each step toward the target orbit.
    Burning,
    /// The body has snapped to the target orbit's canonical initial state.
    Finished,
}

/// Target orbit and progress tracking for a guided transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct GuidedTransfer {
    /// Target orbit type.
    pub orbit_type: OrbitType,
    /// Target orbit parameters (including `true_anomaly_rad` for the snap state).
    pub params: OrbitParams,
    /// Corrective delta-v magnitude when the transfer started.
    pub initial_remaining: f64,
    /// Most recently computed remaining corrective delta-v magnitude.
    pub last_remaining: f64,
    /// Current status.
    pub status: TransferBurnStatus,
    /// Whether the transfer primarily lowers circular altitude.
    pub lowering_altitude: bool,
}

impl GuidedTransfer {
    fn new(
        orbit_type: OrbitType,
        params: OrbitParams,
        initial_remaining: f64,
        lowering_altitude: bool,
    ) -> Self {
        Self {
            orbit_type,
            params,
            initial_remaining,
            last_remaining: initial_remaining,
            status: TransferBurnStatus::Burning,
            lowering_altitude,
        }
    }

    fn finish(&mut self) {
        self.last_remaining = 0.0;
        self.status = TransferBurnStatus::Finished;
    }

    /// Fraction complete in `[0, 1]`.
    pub fn progress(&self) -> f64 {
        if self.status == TransferBurnStatus::Finished {
            return 1.0;
        }
        if self.initial_remaining <= f64::EPSILON {
            return 1.0;
        }
        (1.0 - (self.last_remaining / self.initial_remaining)).clamp(0.0, 1.0)
    }
}

/// Per-body guided transfer state for a simulation.
#[derive(Debug, Clone, Default)]
pub struct TransferBurnTracker {
    transfers: HashMap<BodyId, GuidedTransfer>,
}

impl TransferBurnTracker {
    /// Starts a guided transfer toward the target orbit.
    pub fn start(
        &mut self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
        initial_remaining: f64,
        lowering_altitude: bool,
    ) {
        self.transfers.insert(
            id,
            GuidedTransfer::new(orbit_type, params, initial_remaining, lowering_altitude),
        );
    }

    /// Marks a transfer as finished (after snapping to the target state).
    pub fn finish(&mut self, id: BodyId) {
        if let Some(transfer) = self.transfers.get_mut(&id) {
            transfer.finish();
        }
    }

    /// Returns body ids with an active or finished transfer entry.
    pub fn body_ids(&self) -> Vec<BodyId> {
        self.transfers.keys().copied().collect()
    }

    /// Whether the active transfer lowers circular altitude.
    pub fn lowering_altitude(&self, id: BodyId) -> bool {
        self.transfers
            .get(&id)
            .map(|transfer| transfer.lowering_altitude)
            .unwrap_or(false)
    }

    /// Returns the target orbit for a body, if a transfer is registered.
    pub fn target(&self, id: BodyId) -> Option<(OrbitType, OrbitParams)> {
        self.transfers
            .get(&id)
            .map(|transfer| (transfer.orbit_type, transfer.params))
    }

    /// Updates the cached remaining delta-v magnitude for progress display.
    pub fn set_last_remaining(&mut self, id: BodyId, remaining: f64) {
        if let Some(transfer) = self.transfers.get_mut(&id) {
            transfer.last_remaining = remaining;
        }
    }

    /// Returns burn status for a body.
    pub fn status(&self, id: BodyId) -> TransferBurnStatus {
        self.transfers
            .get(&id)
            .map(|transfer| transfer.status)
            .unwrap_or(TransferBurnStatus::Idle)
    }

    /// Remaining corrective delta-v magnitude for an active or finished transfer.
    pub fn remaining(&self, id: BodyId) -> f64 {
        self.transfers
            .get(&id)
            .map(|transfer| transfer.last_remaining)
            .unwrap_or(0.0)
    }

    /// Burn progress in `[0, 1]`.
    pub fn progress(&self, id: BodyId) -> f64 {
        self.transfers
            .get(&id)
            .map(|transfer| transfer.progress())
            .unwrap_or(0.0)
    }

    /// Clears transfer state for a body (e.g. after acknowledging completion).
    pub fn clear(&mut self, id: BodyId) {
        self.transfers.remove(&id);
    }

    /// Clears all transfers.
    pub fn clear_all(&mut self) {
        self.transfers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{GuidedTransfer, TRANSFER_SNAP_EPSILON, TransferBurnStatus};
    use crate::orbits::{OrbitParams, OrbitType};

    #[test]
    fn progress_tracks_remaining_delta_v() {
        let mut transfer = GuidedTransfer::new(
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.1),
            0.01,
            false,
        );
        assert!((transfer.progress() - 0.0).abs() < 1e-9);
        transfer.last_remaining = 0.005;
        assert!((transfer.progress() - 0.5).abs() < 1e-9);
        transfer.finish();
        assert_eq!(transfer.status, TransferBurnStatus::Finished);
        assert!((transfer.progress() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn snap_epsilon_is_small() {
        const { assert!(TRANSFER_SNAP_EPSILON > 0.0) };
        const { assert!(TRANSFER_SNAP_EPSILON < 1e-3) };
    }
}
