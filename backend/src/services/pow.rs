use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

pub fn generate_pow_prefix() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const PREFIX_LEN: usize = 32;

    let mut rng = rand::rng();

    (0..PREFIX_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn verify_pow(solution: &str, prefix: &str, complexity: i16, creation: &DateTime<Utc>) -> bool {
    const POW_MAX_AGE_SECONDS: u64 = 3600;
    let age = Utc::now().signed_duration_since(*creation).num_seconds() as u64;
    if age > POW_MAX_AGE_SECONDS {
        return false;
    }

    let mut hasher = Sha256::new();
    let input = format!("{}:{}", prefix, solution);
    hasher.update(input.as_bytes());
    let hash_result = hasher.finalize();

    let mut leading_zero_bits = 0;
    for byte in hash_result.iter() {
        if *byte == 0 {
            leading_zero_bits += 8;
        } else {
            leading_zero_bits += byte.leading_zeros() as usize;
            break;
        }
    }

    leading_zero_bits >= complexity as usize
}
