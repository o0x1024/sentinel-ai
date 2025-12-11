//! License Generator Tool
//! 
//! This tool is used to generate license keys for Sentinel AI.
//! It should be run by the software vendor, NOT distributed to end users.
//!
//! Usage:
//!   license_generator generate-keys     - Generate a new key pair
//!   license_generator sign <machine_id> - Sign a license for a machine ID

use std::fs;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Serialize, Deserialize};

const KEYS_FILE: &str = "license_keys.json";

#[derive(Serialize, Deserialize)]
struct KeyPairStore {
    public_key: String,
    private_key: String,
}

#[derive(Serialize, Deserialize)]
struct LicenseKey {
    machine_id: String,
    signature: String,
    metadata: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_help();
        return;
    }
    
    match args[1].as_str() {
        "generate-keys" | "gen" => generate_keys(),
        "sign" => {
            if args.len() < 3 {
                eprintln!("Error: Machine ID required");
                eprintln!("Usage: license_generator sign <machine_id> [metadata]");
                return;
            }
            let metadata = args.get(3).map(|s| s.as_str());
            sign_license(&args[2], metadata);
        }
        "verify" => {
            if args.len() < 3 {
                eprintln!("Error: License key required");
                eprintln!("Usage: license_generator verify <license_key>");
                return;
            }
            verify_license(&args[2]);
        }
        "show-public-key" | "pubkey" => show_public_key(),
        "help" | "-h" | "--help" => print_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
        }
    }
}

fn print_help() {
    println!(r#"
Sentinel AI License Generator
==============================

Commands:
  generate-keys, gen     Generate a new Ed25519 key pair
  sign <machine_id>      Sign a license for the given machine ID
                         Optional: add metadata as third argument
  verify <license_key>   Verify a license key
  show-public-key, pubkey Show the public key to embed in application
  help                   Show this help

Examples:
  license_generator gen
  license_generator sign ABCD-1234-EFGH-5678
  license_generator sign ABCD-1234-EFGH-5678 "Customer: John Doe"
  license_generator verify <base64_license_key>
  license_generator pubkey

Notes:
  - Keep the private key secure! Never share it.
  - The public key should be embedded in the application (crypto.rs).
  - Machine IDs are in format: XXXX-XXXX-XXXX-XXXX
"#);
}

fn generate_keys() {
    println!("Generating new Ed25519 key pair...\n");
    
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    
    let private_key_b64 = BASE64.encode(signing_key.to_bytes());
    let public_key_b64 = BASE64.encode(verifying_key.to_bytes());
    
    let key_store = KeyPairStore {
        public_key: public_key_b64.clone(),
        private_key: private_key_b64.clone(),
    };
    
    // Save to file
    let json = serde_json::to_string_pretty(&key_store).unwrap();
    fs::write(KEYS_FILE, &json).expect("Failed to write keys file");
    
    println!("Keys generated and saved to: {}\n", KEYS_FILE);
    println!("=== PUBLIC KEY (embed in application) ===");
    println!("{}\n", public_key_b64);
    println!("=== PRIVATE KEY (keep secret!) ===");
    println!("{}\n", private_key_b64);
    println!("⚠️  WARNING: Keep the private key secure!");
    println!("⚠️  The public key should be embedded in sentinel-license/src/crypto.rs");
}

fn load_keys() -> Option<KeyPairStore> {
    let content = fs::read_to_string(KEYS_FILE).ok()?;
    serde_json::from_str(&content).ok()
}

fn sign_license(machine_id: &str, metadata: Option<&str>) {
    println!("Signing license for machine ID: {}\n", machine_id);
    
    // Load keys
    let keys = match load_keys() {
        Some(k) => k,
        None => {
            eprintln!("Error: No keys found. Run 'license_generator generate-keys' first.");
            return;
        }
    };
    
    // Decode private key
    let private_key_bytes = match BASE64.decode(&keys.private_key) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error decoding private key: {}", e);
            return;
        }
    };
    
    let key_array: [u8; 32] = match private_key_bytes.try_into() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Error: Invalid private key length");
            return;
        }
    };
    
    let signing_key = SigningKey::from_bytes(&key_array);
    
    // Parse machine ID (remove dashes and convert to hash)
    let machine_id_clean = machine_id.replace("-", "").to_lowercase();
    let machine_id_hash = if machine_id_clean.len() == 64 {
        // Already a full hash
        hex::decode(&machine_id_clean).unwrap_or_else(|_| {
            // Treat as display string, hash it
            let mut hasher = Sha256::new();
            hasher.update(machine_id.as_bytes());
            hasher.finalize().to_vec()
        })
    } else {
        // Display format, need to reconstruct (this is a limitation)
        // In practice, user should provide full hash
        eprintln!("Note: Using display format machine ID. For best results, provide full hash.");
        let mut hasher = Sha256::new();
        hasher.update(machine_id.as_bytes());
        hasher.finalize().to_vec()
    };
    
    // Create message to sign
    let mut hasher = Sha256::new();
    hasher.update(&machine_id_hash);
    if let Some(meta) = metadata {
        hasher.update(meta.as_bytes());
    }
    let message: [u8; 32] = hasher.finalize().into();
    
    // Sign
    let signature = signing_key.sign(&message);
    
    // Create license key
    let license = LicenseKey {
        machine_id: hex::encode(&machine_id_hash),
        signature: BASE64.encode(signature.to_bytes()),
        metadata: metadata.map(|s| s.to_string()),
    };
    
    // Encode to final format
    let json = serde_json::to_string(&license).unwrap();
    let license_key = BASE64.encode(json.as_bytes());
    
    println!("=== LICENSE KEY ===");
    println!("{}\n", license_key);
    println!("Machine ID: {}", license.machine_id);
    if let Some(meta) = &license.metadata {
        println!("Metadata: {}", meta);
    }
}

fn verify_license(license_key: &str) {
    println!("Verifying license key...\n");
    
    // Load keys
    let keys = match load_keys() {
        Some(k) => k,
        None => {
            eprintln!("Error: No keys found. Run 'license_generator generate-keys' first.");
            return;
        }
    };
    
    // Decode public key
    let public_key_bytes = match BASE64.decode(&keys.public_key) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error decoding public key: {}", e);
            return;
        }
    };
    
    let key_array: [u8; 32] = match public_key_bytes.try_into() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Error: Invalid public key length");
            return;
        }
    };
    
    let verifying_key = match VerifyingKey::from_bytes(&key_array) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Error creating verifying key: {}", e);
            return;
        }
    };
    
    // Decode license
    let license_json = match BASE64.decode(license_key.trim()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error decoding license key: {}", e);
            return;
        }
    };
    
    let license: LicenseKey = match serde_json::from_slice(&license_json) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error parsing license: {}", e);
            return;
        }
    };
    
    // Decode machine ID and signature
    let machine_id_bytes = match hex::decode(&license.machine_id) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error decoding machine ID: {}", e);
            return;
        }
    };
    
    let sig_bytes = match BASE64.decode(&license.signature) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error decoding signature: {}", e);
            return;
        }
    };
    
    let sig_array: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Error: Invalid signature length");
            return;
        }
    };
    
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);
    
    // Recreate message
    let mut hasher = Sha256::new();
    hasher.update(&machine_id_bytes);
    if let Some(ref meta) = license.metadata {
        hasher.update(meta.as_bytes());
    }
    let message: [u8; 32] = hasher.finalize().into();
    
    // Verify
    match verifying_key.verify(&message, &signature) {
        Ok(_) => {
            println!("✅ License is VALID");
            println!("Machine ID: {}", license.machine_id);
            if let Some(meta) = &license.metadata {
                println!("Metadata: {}", meta);
            }
        }
        Err(_) => {
            println!("❌ License is INVALID");
        }
    }
}

fn show_public_key() {
    let keys = match load_keys() {
        Some(k) => k,
        None => {
            eprintln!("Error: No keys found. Run 'license_generator generate-keys' first.");
            return;
        }
    };
    
    println!("=== PUBLIC KEY ===");
    println!("Copy this to sentinel-license/src/crypto.rs:\n");
    println!("const EMBEDDED_PUBLIC_KEY: &str = \"{}\";", keys.public_key);
}
