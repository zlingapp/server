use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

static ARGON2_CONFIG: argon2::Config = argon2::Config {
    variant: argon2::Variant::Argon2id,
    version: argon2::Version::Version13,
    mem_cost: 4096,
    time_cost: 1,
    lanes: 2,
    thread_mode: argon2::ThreadMode::Parallel,
    secret: &[],
    ad: &[],
    hash_length: 32,
};

fn generate_12b_nonce() -> [u8; 12] {
    rand::thread_rng().gen()
}

pub fn hash(plaintext: &str) -> String {
    let salt = generate_12b_nonce();
    let hashed = argon2::hash_encoded(plaintext.as_bytes(), &salt, &ARGON2_CONFIG).unwrap();
    hashed
}

pub fn verify(plaintext: &str, hash: &str) -> bool {
    return argon2::verify_encoded(hash, plaintext.as_bytes()).unwrap();
}

type HmacSha256 = Hmac<Sha256>;

pub fn generate_token_sig_key() -> [u8; 32] {
    rand::thread_rng().gen()
}

pub fn sign(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);

    let result = mac.finalize().into_bytes();
    result.to_vec()
}

pub fn verify_signature(key: &[u8], data: &[u8], signature: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);

    mac.verify_slice(signature).is_ok()
}
