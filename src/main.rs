//! ProofCapture CLI Verifier
//!
//! Verify ProofCapture recordings from the command line.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use proofcapture_cli::verify::{verify_sealed_bundle, verify_and_extract_sealed_bundle, verify_standard_bundle, verify_open_bundle, VerificationResult};
use proofcapture_cli::VerifyError;

/// ProofCapture CLI Verifier - Verify ProofCapture recordings
#[derive(Parser, Debug)]
#[command(name = "proofcapture-cli")]
#[command(author = "Best Day Labs")]
#[command(version)]
#[command(about = "Verify ProofCapture recordings from the command line")]
struct Args {
    /// Path to a proof bundle (.proofcapture, .proofbundle, or directory)
    #[arg(value_name = "PATH")]
    path: PathBuf,

    /// Password for sealed bundles (will prompt if not provided)
    #[arg(short, long)]
    password: Option<String>,

    /// Output format: text or json
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Extract audio file from sealed bundle to specified directory
    #[arg(short, long, value_name = "DIR")]
    extract: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Unknown format: {}. Use 'text' or 'json'", s)),
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    match run(&args) {
        Ok(result) => {
            print_success(&result, &args);
            ExitCode::SUCCESS
        }
        Err(e) => {
            print_error(&e, &args);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run(args: &Args) -> Result<VerificationResult, VerifyError> {
    let path = &args.path;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "proofcapture" => {
            // Sealed bundle - requires password
            let password = match &args.password {
                Some(p) => p.clone(),
                None => prompt_password()?,
            };

            if let Some(extract_dir) = &args.extract {
                let result = verify_and_extract_sealed_bundle(path, &password)?;

                fs::create_dir_all(extract_dir).map_err(|e| VerifyError::Io(e))?;

                let audio_path = extract_dir.join(&result.audio_filename);
                fs::write(&audio_path, &result.audio_data).map_err(|e| VerifyError::Io(e))?;

                eprintln!("Audio extracted to: {}", audio_path.display());

                Ok(VerificationResult {
                    manifest: result.manifest,
                    trust_level: result.trust_level,
                })
            } else {
                verify_sealed_bundle(path, &password)
            }
        }
        "proofbundle" => {
            // Open proof bundle - no password needed
            if args.extract.is_some() {
                eprintln!("Note: --extract only applies to sealed .proofcapture files.");
                eprintln!("      Open bundles already contain unencrypted media.");
            }
            verify_open_bundle(path)
        }
        _ => {
            // Standard bundle (directory or loose files)
            if args.extract.is_some() {
                eprintln!("Note: --extract only applies to sealed .proofcapture files.");
                eprintln!("      Standard bundles already contain the audio file.");
            }
            verify_standard_bundle(path)
        }
    }
}

fn prompt_password() -> Result<String, VerifyError> {
    eprint!("Password: ");
    io::stderr().flush().ok();

    let mut password = String::new();
    io::stdin()
        .read_line(&mut password)
        .map_err(|e| VerifyError::Io(e))?;

    Ok(password.trim().to_string())
}

fn print_success(result: &VerificationResult, args: &Args) {
    if args.format == OutputFormat::Json {
        print_success_json(result);
    } else {
        print_success_text(result, args.verbose);
    }
}

fn print_success_text(result: &VerificationResult, verbose: bool) {
    let reset = "\x1b[0m";
    let green = "\x1b[32m";
    let bold = "\x1b[1m";
    let level_color = result.trust_level.color_code();

    println!();
    println!("{}PROOFAUDIO VERIFICATION SUMMARY{}", bold, reset);
    println!("===============================");
    println!(
        "Status:      {}{}VERIFIED{}",
        bold, green, reset
    );
    println!(
        "Trust Level: {}{} ({}){}",
        level_color,
        result.trust_level.display_name(),
        result.trust_level.label(),
        reset
    );

    let m = &result.manifest;

    println!();
    println!("{}RECORDING DETAILS{}", bold, reset);
    println!("-----------------");
    println!("Captured:    {}", m.capture_start);
    println!("Duration:    {:.1}s", m.duration_seconds);
    println!("Format:      {} (M4A container)", m.audio_format.to_uppercase());
    println!("Size:        {} bytes", m.audio_size_bytes);

    if verbose {
        println!("Audio Hash:  {}", m.audio_hash);
    }

    println!();
    println!("{}CRYPTOGRAPHIC IDENTITY{}", bold, reset);
    println!("----------------------");
    println!("Device Key:  {}...", &m.device_key_id[..20.min(m.device_key_id.len())]);
    println!("App:         {} v{}", m.app_bundle_id, m.app_version);

    // Trust vectors
    println!();
    println!("{}TRUST VECTORS{}", bold, reset);
    println!("-------------");

    if let Some(loc) = &m.trust_vectors.location {
        println!("Location:");
        println!("  Start:     {:.6}, {:.6} (±{:.0}m)", loc.start.lat, loc.start.lon, loc.start.accuracy);
        println!("  End:       {:.6}, {:.6} (±{:.0}m)", loc.end.lat, loc.end.lon, loc.end.accuracy);
    } else {
        println!("Location:    Not captured");
    }

    if let Some(motion) = &m.trust_vectors.motion {
        let state = if motion.acceleration_variance < 0.01 {
            "Stationary"
        } else {
            "In motion"
        };
        println!("Motion:      {}", state);
        println!("  Accel Var: {:.6}", motion.acceleration_variance);
        println!("  Rot Var:   {:.6}", motion.rotation_variance);
        println!("  Duration:  {:.2}s", motion.duration);
        println!("  Samples:   {}", motion.sample_count);
    } else {
        println!("Motion:      Not captured");
    }

    if let Some(cont) = &m.trust_vectors.continuity {
        let status = if cont.uninterrupted {
            "Uninterrupted"
        } else {
            "Interrupted"
        };
        println!("Continuity:  {}", status);
        if !cont.interruption_events.is_empty() {
            println!("  Events:");
            for event in &cont.interruption_events {
                println!("    - {}: {}", event.timestamp, event.reason);
            }
        }
    } else {
        println!("Continuity:  Not tracked");
    }

    if let Some(clock) = &m.trust_vectors.clock {
        println!("Clock:");
        println!("  Start:     {}", clock.wall_clock_start);
        println!("  End:       {}", clock.wall_clock_end);
        println!("  Delta:     {:.6}s", clock.monotonic_delta);
        println!("  Time Zone: {}", clock.time_zone);
    } else {
        println!("Clock:       Not captured");
    }

    // Limitations
    println!();
    println!("{}LIMITATIONS{}", bold, reset);
    println!("-----------");
    println!("This verification proves capture integrity, NOT:");
    println!("- Who is speaking");
    println!("- That statements are true");
    println!("- Legal consent to record");
    println!("- Absence of AI-generated audio");
    println!();
}

fn print_success_json(result: &VerificationResult) {
    let m = &result.manifest;
    let json = serde_json::json!({
        "status": "verified",
        "trustLevel": result.trust_level.display_name(),
        "trustLevelLabel": result.trust_level.label(),
        "schemaVersion": m.schema_version,
        "recording": {
            "captureStart": m.capture_start,
            "captureEnd": m.capture_end,
            "durationSeconds": m.duration_seconds,
            "audioFormat": m.audio_format,
            "audioSizeBytes": m.audio_size_bytes,
            "audioHash": m.audio_hash
        },
        "identity": {
            "deviceKeyId": m.device_key_id,
            "publicKey": m.public_key,
            "appBundleId": m.app_bundle_id,
            "appVersion": m.app_version
        },
        "trustVectors": {
            "location": m.trust_vectors.location.as_ref().map(|l| serde_json::json!({
                "start": {
                    "lat": l.start.lat,
                    "lon": l.start.lon,
                    "accuracy": l.start.accuracy
                },
                "end": {
                    "lat": l.end.lat,
                    "lon": l.end.lon,
                    "accuracy": l.end.accuracy
                }
            })),
            "motion": m.trust_vectors.motion.as_ref().map(|mot| serde_json::json!({
                "accelerationVariance": mot.acceleration_variance,
                "rotationVariance": mot.rotation_variance,
                "duration": mot.duration,
                "sampleCount": mot.sample_count
            })),
            "continuity": m.trust_vectors.continuity.as_ref().map(|c| serde_json::json!({
                "uninterrupted": c.uninterrupted,
                "interruptionEvents": c.interruption_events.iter().map(|e| serde_json::json!({
                    "timestamp": e.timestamp,
                    "reason": e.reason
                })).collect::<Vec<_>>()
            })),
            "clock": m.trust_vectors.clock.as_ref().map(|c| serde_json::json!({
                "wallClockStart": c.wall_clock_start,
                "wallClockEnd": c.wall_clock_end,
                "monotonicDelta": c.monotonic_delta,
                "timeZone": c.time_zone
            }))
        },
        "signature": m.signature
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

fn print_error(error: &VerifyError, args: &Args) {
    if args.format == OutputFormat::Json {
        print_error_json(error);
    } else {
        print_error_text(error);
    }
}

fn print_error_text(error: &VerifyError) {
    let reset = "\x1b[0m";
    let red = "\x1b[31m";
    let bold = "\x1b[1m";

    eprintln!();
    eprintln!("{}PROOFAUDIO VERIFICATION SUMMARY{}", bold, reset);
    eprintln!("===============================");
    eprintln!("Status:      {}{}FAILED{}", bold, red, reset);
    eprintln!("Error:       {}", error);
    eprintln!();

    match error {
        VerifyError::HashMismatch => {
            eprintln!("The audio file does not match the cryptographic hash");
            eprintln!("recorded at capture time. This recording cannot be");
            eprintln!("verified as authentic.");
        }
        VerifyError::SignatureInvalid => {
            eprintln!("The digital signature is invalid. The manifest may have");
            eprintln!("been tampered with or was not created by ProofCapture.");
        }
        VerifyError::DecryptionFailed => {
            eprintln!("Could not decrypt the sealed proof. Please check your");
            eprintln!("password and try again.");
        }
        _ => {}
    }
    eprintln!();
}

fn print_error_json(error: &VerifyError) {
    let json = serde_json::json!({
        "status": "failed",
        "error": error.to_string(),
        "exitCode": error.exit_code()
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
