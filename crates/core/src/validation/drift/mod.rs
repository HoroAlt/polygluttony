//! Weighted multi-signal drift detection (layer 2). Port of
//! `validation/drift_detector.py`. Threshold 0.7; weights are calibrated —
//! keep in sync with the Python reference.

pub mod signals;

mod drift;
pub use drift::*;
