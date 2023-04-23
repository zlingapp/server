use argon2;
use rand::RngCore;

static ARGON2_CONFIG: argon2::Config = argon2::Config {
    variant: argon2::Variant::Argon2id,
    version: argon2::Version::Version13,
    mem_cost: 4096,
    time_cost: 1,
    lanes: 2,
    thread_mode: argon2::ThreadMode::Parallel,
    secret: &[],
    ad: &[],
    hash_length: 32
};

fn generate_12b_nonce() -> [u8; 12] {
    // generate a random nonce
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    return nonce;
}

pub fn hash(plaintext: &str) -> String {
    let salt = generate_12b_nonce();
    let hashed = argon2::hash_encoded(plaintext.as_bytes(), &salt, &ARGON2_CONFIG).unwrap();
    return hashed;
}

pub fn verify(plaintext: &str, hash: &str) -> bool {
    return argon2::verify_encoded(hash, plaintext.as_bytes()).unwrap();
}