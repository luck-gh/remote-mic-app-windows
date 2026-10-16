use base64::Engine;
use minisign_verify::{PublicKey, Signature};

fn decoded(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        base64::engine::general_purpose::STANDARD.decode(value.trim())?,
    )?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "Usage: verify-updater-signing <installer> <signature> <tauri.conf.json>".into(),
        );
    }
    let config: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[3])?)?;
    let key = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .ok_or("Configured updater public key missing")?;
    let public_key = PublicKey::decode(&decoded(key)?)?;
    let signature = Signature::decode(&decoded(&std::fs::read_to_string(&args[2])?)?)?;
    public_key.verify(&std::fs::read(&args[1])?, &signature, true)?;
    println!("updater_signing result=passed configured_key_matches_artifact=true");
    Ok(())
}
