use rand::{distributions::Alphanumeric, Rng};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generates:
/// - raw API key (to return once to client)
/// - hashed API key (to store in DB)
pub fn generate_api_key() -> (String, String) {
    // 1️⃣ Generate random part
    let random_part: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();

    // 2️⃣ Raw API key (what client sees)
    let raw_key = format!("dodo_live_{}", random_part);

    // 3️⃣ Load secret (VERY IMPORTANT)
    let secret = std::env::var("API_KEY_SECRET")
        .expect("API_KEY_SECRET must be set");

    // 4️⃣ HMAC-SHA256 hash
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");

    mac.update(raw_key.as_bytes());

    let hashed_key = hex::encode(mac.finalize().into_bytes());

    (raw_key, hashed_key)
}

pub fn hash_api_key(raw_key: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let secret = std::env::var("API_KEY_SECRET")
        .expect("API_KEY_SECRET must be set");

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(raw_key.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}
