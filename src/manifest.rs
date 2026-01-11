//! Manifest parsing and canonicalization.
//!
//! Handles parsing of SignedAudioManifest from iOS and
//! canonicalization for signature verification.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::crypto::sha256_bytes;
use crate::error::{Result, VerifyError};

/// Current supported schema version.
pub const CURRENT_SCHEMA_VERSION: i32 = 1;

/// The signed audio manifest structure from iOS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedAudioManifest {
    pub schema_version: i32,
    pub audio_hash: String,
    pub audio_format: String,
    pub audio_size_bytes: i64,
    pub capture_start: String,
    pub capture_end: String,
    pub duration_seconds: f64,
    pub app_version: String,
    pub app_bundle_id: String,
    pub device_key_id: String,
    pub public_key: String,
    pub trust_vectors: TrustVectors,
    pub signature: String,
}

/// Trust vectors container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustVectors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationVector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<MotionVector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuity: Option<ContinuityVector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock: Option<ClockVector>,
}

/// Location trust vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationVector {
    pub start: LocationSnapshot,
    pub end: LocationSnapshot,
}

/// Location snapshot at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationSnapshot {
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
}

/// Motion trust vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionVector {
    pub acceleration_variance: f64,
    pub rotation_variance: f64,
    pub duration: f64,
    pub sample_count: i32,
}

/// Continuity trust vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityVector {
    pub uninterrupted: bool,
    pub interruption_events: Vec<InterruptionEvent>,
}

/// An interruption event during recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionEvent {
    pub timestamp: String,
    pub reason: String,
}

/// Clock trust vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockVector {
    pub wall_clock_start: String,
    pub wall_clock_end: String,
    pub monotonic_delta: f64,
    pub time_zone: String,
}

impl SignedAudioManifest {
    /// Parse manifest from JSON bytes.
    pub fn from_json(json_bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(json_bytes).map_err(|_| VerifyError::ManifestMalformed)
    }

    /// Validate schema version is supported.
    pub fn validate_schema(&self) -> Result<()> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(VerifyError::SchemaUnsupported {
                version: self.schema_version,
            });
        }
        Ok(())
    }

    /// Compute the canonical hash for signature verification.
    ///
    /// This must exactly match iOS's HashingService.computeManifestHash():
    /// - Exclude the "signature" field
    /// - Sort keys alphabetically (recursive)
    /// - Compact JSON (no whitespace)
    /// - UTF-8 encoding
    pub fn compute_canonical_hash(&self) -> Result<[u8; 32]> {
        // Parse to generic Value to manipulate
        let json_str = serde_json::to_string(self).map_err(|_| VerifyError::ManifestMalformed)?;
        let mut value: Value =
            serde_json::from_str(&json_str).map_err(|_| VerifyError::ManifestMalformed)?;

        // Remove signature field
        if let Value::Object(ref mut map) = value {
            map.remove("signature");
        }

        // Serialize with sorted keys (serde_json sorts by default for BTreeMap)
        // We need to ensure keys are sorted - convert to sorted representation
        let canonical = canonicalize_json(&value)?;

        Ok(sha256_bytes(canonical.as_bytes()))
    }
}

/// Compute canonical hash directly from JSON bytes (preserves original formatting).
/// This is the preferred method as it preserves the original number formatting.
pub fn compute_canonical_hash_from_bytes(json_bytes: &[u8]) -> Result<[u8; 32]> {
    // Parse to generic Value
    let mut value: Value =
        serde_json::from_slice(json_bytes).map_err(|_| VerifyError::ManifestMalformed)?;

    // Remove signature field
    if let Value::Object(ref mut map) = value {
        map.remove("signature");
    }

    // Canonicalize (sort keys, compact)
    let canonical = canonicalize_json(&value)?;

    Ok(sha256_bytes(canonical.as_bytes()))
}

/// Recursively sort JSON object keys and produce compact output.
fn canonicalize_json(value: &Value) -> Result<String> {
    match value {
        Value::Object(map) => {
            // Sort keys and recursively canonicalize values
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));

            let pairs: Vec<String> = sorted
                .into_iter()
                .map(|(k, v)| {
                    let canonical_v = canonicalize_json(v)?;
                    Ok(format!("\"{}\":{}", k, canonical_v))
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(format!("{{{}}}", pairs.join(",")))
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(canonicalize_json)
                .collect::<Result<Vec<_>>>()?;
            Ok(format!("[{}]", items.join(",")))
        }
        Value::String(s) => Ok(format!("\"{}\"", escape_json_string(s))),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        Value::Null => Ok("null".to_string()),
    }
}

/// Escape special characters in JSON strings.
/// Note: iOS JSONEncoder escapes forward slashes, so we must too for compatibility.
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '/' => result.push_str("\\/"),  // iOS escapes forward slashes
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_simple_object() {
        let json: Value = serde_json::json!({
            "b": 2,
            "a": 1
        });
        let canonical = canonicalize_json(&json).unwrap();
        assert_eq!(canonical, r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn test_canonicalize_nested_object() {
        let json: Value = serde_json::json!({
            "z": {"b": 2, "a": 1},
            "a": "test"
        });
        let canonical = canonicalize_json(&json).unwrap();
        assert_eq!(canonical, r#"{"a":"test","z":{"a":1,"b":2}}"#);
    }
}
