//! Trust level computation.
//!
//! Computes trust levels (A, B, C) based on present trust vectors.
//! Level A is highest, Level C is lowest.

use crate::manifest::TrustVectors;

/// Trust level indicating verification completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Level A: Full verification with all context vectors and uninterrupted continuity.
    /// This is the highest trust level.
    A,

    /// Level B: Verified capture with location and motion context.
    B,

    /// Level C: Basic verified capture (audio hash + signature + timestamp only).
    /// This is the minimum trust level for a verified recording.
    C,
}

impl TrustLevel {
    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            TrustLevel::A => "Level A",
            TrustLevel::B => "Level B",
            TrustLevel::C => "Level C",
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            TrustLevel::A => "Verified Continuous Capture",
            TrustLevel::B => "Verified Capture + Context",
            TrustLevel::C => "Verified Capture",
        }
    }

    /// Explanation of what this trust level means.
    pub fn explanation(&self) -> &'static str {
        match self {
            TrustLevel::A => {
                "This recording was captured continuously without interruption, with full context."
            }
            TrustLevel::B => {
                "This recording was captured by ProofCapture with location and motion context."
            }
            TrustLevel::C => "This recording was captured by ProofCapture and has not been modified.",
        }
    }

    /// ANSI color code for terminal output.
    pub fn color_code(&self) -> &'static str {
        match self {
            TrustLevel::A => "\x1b[32m", // Green
            TrustLevel::B => "\x1b[34m", // Blue
            TrustLevel::C => "\x1b[33m", // Orange/Yellow
        }
    }
}

/// Compute trust level from trust vectors.
///
/// Rules:
/// - Level A: location + motion + continuity.uninterrupted
/// - Level B: location + motion
/// - Level C: default (valid signature only)
pub fn compute_trust_level(vectors: &TrustVectors) -> TrustLevel {
    let has_location = vectors.location.is_some();
    let has_motion = vectors.motion.is_some();
    let is_uninterrupted = vectors
        .continuity
        .as_ref()
        .map(|c| c.uninterrupted)
        .unwrap_or(false);

    if has_location && has_motion && is_uninterrupted {
        TrustLevel::A
    } else if has_location && has_motion {
        TrustLevel::B
    } else {
        TrustLevel::C
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ContinuityVector, LocationSnapshot, LocationVector, MotionVector};

    fn make_location() -> LocationVector {
        LocationVector {
            start: LocationSnapshot {
                lat: 37.775,
                lon: -122.418,
                accuracy: 65.0,
            },
            end: LocationSnapshot {
                lat: 37.775,
                lon: -122.418,
                accuracy: 65.0,
            },
        }
    }

    fn make_motion() -> MotionVector {
        MotionVector {
            acceleration_variance: 0.001,
            rotation_variance: 0.001,
            duration: 60.0,
            sample_count: 600,
        }
    }

    fn make_continuity(uninterrupted: bool) -> ContinuityVector {
        ContinuityVector {
            uninterrupted,
            interruption_events: vec![],
        }
    }

    #[test]
    fn test_level_a() {
        let vectors = TrustVectors {
            location: Some(make_location()),
            motion: Some(make_motion()),
            continuity: Some(make_continuity(true)),
            clock: None,
        };
        assert_eq!(compute_trust_level(&vectors), TrustLevel::A);
    }

    #[test]
    fn test_level_b() {
        let vectors = TrustVectors {
            location: Some(make_location()),
            motion: Some(make_motion()),
            continuity: None,
            clock: None,
        };
        assert_eq!(compute_trust_level(&vectors), TrustLevel::B);
    }

    #[test]
    fn test_level_b_interrupted() {
        let vectors = TrustVectors {
            location: Some(make_location()),
            motion: Some(make_motion()),
            continuity: Some(make_continuity(false)),
            clock: None,
        };
        assert_eq!(compute_trust_level(&vectors), TrustLevel::B);
    }

    #[test]
    fn test_level_c_no_vectors() {
        let vectors = TrustVectors {
            location: None,
            motion: None,
            continuity: None,
            clock: None,
        };
        assert_eq!(compute_trust_level(&vectors), TrustLevel::C);
    }

    #[test]
    fn test_level_c_location_only() {
        let vectors = TrustVectors {
            location: Some(make_location()),
            motion: None,
            continuity: None,
            clock: None,
        };
        assert_eq!(compute_trust_level(&vectors), TrustLevel::C);
    }
}
