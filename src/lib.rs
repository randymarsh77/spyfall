use aes_gcm::aead::{Aead, AeadCore, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const LOCATIONS_FILE: &str = "locations.json";
const PRIME_BITS: usize = 48; // The more bits, the longer it takes to factor

// Default locations list (fallback if locations.json doesn't exist)
const DEFAULT_LOCATIONS: &[&str] = &[
    "airplane",
    "bank",
    "beach",
    "casino",
    "cathedral",
    "circus_tent",
    "corporate_party",
    "crusader_army",
    "day_spa",
    "embassy",
    "hospital",
    "hotel",
    "military_base",
    "movie_studio",
    "ocean_liner",
    "passenger_train",
    "pirate_ship",
    "polar_station",
    "police_station",
    "restaurant",
    "school",
    "service_station",
    "space_station",
    "submarine",
    "supermarket",
    "theater",
    "university",
    "world_war_ii_squad",
];

#[derive(Debug, Serialize, Deserialize)]
pub struct Challenge {
    pub semiprimes: Vec<String>, // Hex-encoded semiprimes
    pub id: String,              // Unique challenge ID
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub encrypted_location: String, // Hex-encoded encrypted location
    pub challenge_id: String,
}

// Load locations from config file or use default list
fn load_locations() -> Result<Vec<String>> {
    match std::fs::read_to_string(LOCATIONS_FILE) {
        Ok(content) => {
            let locations: Vec<String> = serde_json::from_str(&content)?;
            Ok(locations)
        }
        Err(_) => {
            // File doesn't exist or can't be read, use default locations
            Ok(DEFAULT_LOCATIONS.iter().map(|s| s.to_string()).collect())
        }
    }
}

// Generate a random prime of specified bit length
fn generate_prime(bits: usize) -> BigUint {
    let mut rng = rand::thread_rng();
    loop {
        let candidate = generate_random_odd(bits, &mut rng);
        if is_probably_prime(&candidate, 20) {
            return candidate;
        }
    }
}

// Generate a random odd number of specified bit length
fn generate_random_odd(bits: usize, rng: &mut impl Rng) -> BigUint {
    let bytes = bits.div_ceil(8);
    let mut data = vec![0u8; bytes];
    rng.fill(&mut data[..]);

    // Set the most significant bit to ensure the number has the right bit length
    data[0] |= 0x80;
    // Set the least significant bit to ensure it's odd
    data[bytes - 1] |= 0x01;

    BigUint::from_bytes_be(&data)
}

// Miller-Rabin primality test
fn is_probably_prime(n: &BigUint, k: usize) -> bool {
    if n <= &BigUint::one() {
        return false;
    }
    if n <= &(BigUint::one() + BigUint::one() + BigUint::one()) {
        return true;
    }
    if n.clone() % BigUint::from(2u32) == BigUint::zero() {
        return false;
    }

    // Write n-1 as d * 2^r
    let n_minus_1 = n - BigUint::one();
    let mut d = n_minus_1.clone();
    let mut r = 0;
    while d.clone() % BigUint::from(2u32) == BigUint::zero() {
        d /= BigUint::from(2u32);
        r += 1;
    }

    let mut rng = rand::thread_rng();
    for _ in 0..k {
        let a = generate_random_range(&BigUint::from(2u32), &(n - BigUint::from(2u32)), &mut rng);
        let mut x = mod_exp(&a, &d, n);

        if x == BigUint::one() || x == n_minus_1 {
            continue;
        }

        let mut composite = true;
        for _ in 0..(r - 1) {
            x = mod_exp(&x, &BigUint::from(2u32), n);
            if x == n_minus_1 {
                composite = false;
                break;
            }
        }

        if composite {
            return false;
        }
    }
    true
}

// Generate random number in range [min, max)
fn generate_random_range(min: &BigUint, max: &BigUint, rng: &mut impl Rng) -> BigUint {
    let range = max - min;
    let bytes = range.bits().div_ceil(8) as usize;
    loop {
        let mut data = vec![0u8; bytes];
        rng.fill(&mut data[..]);
        let candidate = BigUint::from_bytes_be(&data) % &range;
        if candidate < range {
            return min + candidate;
        }
    }
}

// Modular exponentiation
fn mod_exp(base: &BigUint, exp: &BigUint, modulus: &BigUint) -> BigUint {
    if modulus == &BigUint::one() {
        return BigUint::zero();
    }

    let mut result = BigUint::one();
    let mut base = base.clone() % modulus;
    let mut exp = exp.clone();

    while exp > BigUint::zero() {
        if exp.clone() % BigUint::from(2u32) == BigUint::one() {
            result = (result * &base) % modulus;
        }
        exp /= BigUint::from(2u32);
        base = (&base * &base) % modulus;
    }
    result
}

// Factor a semiprime (this is the proof-of-work)
fn factor_semiprime(n: &BigUint) -> Result<(BigUint, BigUint)> {
    // Try trial division with small primes first
    for p in 2..1000000u64 {
        let p_big = BigUint::from(p);
        if n % &p_big == BigUint::zero() {
            let q = n / &p_big;
            return Ok((p_big, q));
        }
    }

    // Pollard's rho algorithm for larger factors
    pollard_rho(n)
}

// Pollard's rho factorization algorithm
fn pollard_rho(n: &BigUint) -> Result<(BigUint, BigUint)> {
    if n % BigUint::from(2u32) == BigUint::zero() {
        return Ok((BigUint::from(2u32), n / BigUint::from(2u32)));
    }

    let mut x = BigUint::from(2u32);
    let mut y = BigUint::from(2u32);
    let mut d = BigUint::one();

    let f = |x: &BigUint| -> BigUint { (x * x + BigUint::one()) % n };

    while d == BigUint::one() {
        x = f(&x);
        y = f(&f(&y));
        d = gcd(&(if x > y { &x - &y } else { &y - &x }), n);
    }

    if d == *n {
        return Err(anyhow!("Failed to factor the semiprime"));
    }

    let p = d;
    let q = n / &p;
    Ok((p, q))
}

// Greatest Common Divisor using Euclidean algorithm
fn gcd(a: &BigUint, b: &BigUint) -> BigUint {
    if b == &BigUint::zero() {
        a.clone()
    } else {
        gcd(b, &(a % b))
    }
}

// Convert BigUint to AES key
fn bigint_to_aes_key(n: &BigUint) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(n.to_bytes_be());
    hasher.finalize().into()
}

// Encode challenge as base64 string
fn encode_challenge_base64(challenge: &Challenge) -> Result<String> {
    let json = serde_json::to_string(challenge)?;
    Ok(general_purpose::STANDARD.encode(json.as_bytes()))
}

// Decode challenge from base64 string or JSON
fn decode_challenge(input: &str) -> Result<Challenge> {
    // First try to parse as direct JSON
    if let Ok(challenge) = serde_json::from_str::<Challenge>(input) {
        return Ok(challenge);
    }

    // If that fails, try to decode as base64 first
    match general_purpose::STANDARD.decode(input) {
        Ok(decoded_bytes) => {
            let json_str = String::from_utf8(decoded_bytes)
                .map_err(|_| anyhow!("Invalid UTF-8 in base64 decoded data"))?;
            serde_json::from_str(&json_str)
                .map_err(|_| anyhow!("Invalid JSON in base64 decoded data"))
        }
        Err(_) => Err(anyhow!("Input is neither valid JSON nor valid base64")),
    }
}

// Encode response as base64 string
fn encode_response_base64(response: &Response) -> Result<String> {
    let json = serde_json::to_string(response)?;
    Ok(general_purpose::STANDARD.encode(json.as_bytes()))
}

// Decode response from base64 string or JSON
fn decode_response(input: &str) -> Result<Response> {
    // First try to parse as direct JSON
    if let Ok(response) = serde_json::from_str::<Response>(input) {
        return Ok(response);
    }

    // If that fails, try to decode as base64 first
    match general_purpose::STANDARD.decode(input) {
        Ok(decoded_bytes) => {
            let json_str = String::from_utf8(decoded_bytes)
                .map_err(|_| anyhow!("Invalid UTF-8 in base64 decoded data"))?;
            serde_json::from_str(&json_str)
                .map_err(|_| anyhow!("Invalid JSON in base64 decoded data"))
        }
        Err(_) => Err(anyhow!("Input is neither valid JSON nor valid base64")),
    }
}

// Encrypt location using AES-GCM
fn encrypt_location(location: &str, key: &BigUint) -> Result<String> {
    let key_bytes = bigint_to_aes_key(key);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, location.as_bytes())
        .map_err(|_| anyhow!("Encryption failed"))?;

    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(hex::encode(result))
}

// Decrypt location using AES-GCM
fn decrypt_location(encrypted_hex: &str, key: &BigUint) -> Result<String> {
    let encrypted_data = hex::decode(encrypted_hex).map_err(|_| anyhow!("Invalid hex encoding"))?;

    if encrypted_data.len() < 12 {
        return Err(anyhow!("Invalid encrypted data length"));
    }

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key_bytes = bigint_to_aes_key(key);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Decryption failed"))?;

    String::from_utf8(plaintext).map_err(|_| anyhow!("Invalid UTF-8 in decrypted data"))
}

// Deterministically select semiprime based on location
fn select_semiprime_for_location(semiprimes: &[String], location: &str) -> Result<BigUint> {
    let locations = load_locations()?;
    let mut sorted_locations = locations.clone();
    sorted_locations.sort();

    let index = sorted_locations
        .iter()
        .position(|loc| loc == location)
        .ok_or_else(|| anyhow!("Location '{}' not found in locations list", location))?;

    let semiprime_index = index % semiprimes.len();
    let semiprime_hex = &semiprimes[semiprime_index];
    let semiprime_bytes =
        hex::decode(semiprime_hex).map_err(|_| anyhow!("Invalid hex in semiprime"))?;

    Ok(BigUint::from_bytes_be(&semiprime_bytes))
}

pub fn handle_challenge(location: &str) -> Result<()> {
    let locations = load_locations()?;
    if !locations.contains(&location.to_string()) {
        return Err(anyhow!(
            "Location '{}' not found in locations list",
            location
        ));
    }

    let challenge_size = locations.len();
    println!("🔢 Generating challenge for location: {}", location);
    println!(
        "⏳ Creating {} semiprimes (one for each location)...",
        challenge_size
    );

    let mut semiprimes = Vec::new();
    for i in 0..challenge_size {
        if i % 5 == 0 && i > 0 {
            println!("Generated {}/{} semiprimes...", i, challenge_size);
        }

        let p = generate_prime(PRIME_BITS);
        let q = generate_prime(PRIME_BITS);
        let semiprime = &p * &q;
        semiprimes.push(hex::encode(semiprime.to_bytes_be()));
    }

    let challenge_id = hex::encode(Sha256::digest(format!(
        "{}:{}",
        location,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    )));
    let challenge = Challenge {
        semiprimes,
        id: challenge_id,
    };

    let base64_challenge = encode_challenge_base64(&challenge)?;

    println!("\n🎯 Challenge generated successfully!");
    println!("📋 Share this challenge string with other players:");
    println!("================================================================================");
    println!("{}", base64_challenge);
    println!("================================================================================");
    println!("\n💡 Players can use this string directly with the 'respond' and 'verify' commands");

    Ok(())
}

pub fn handle_respond(challenge_input: &str, location: &str) -> Result<()> {
    let challenge = decode_challenge(challenge_input)?;
    let locations = load_locations()?;

    if !locations.contains(&location.to_string()) {
        return Err(anyhow!(
            "Location '{}' not found in locations list",
            location
        ));
    }

    println!("🔍 Finding semiprime for location: {}", location);
    let semiprime = select_semiprime_for_location(&challenge.semiprimes, location)?;

    println!("⚡ Performing proof-of-work (factoring semiprime)...");
    println!("⏳ This will take a moment...");

    let start = std::time::Instant::now();
    let (p, q) = factor_semiprime(&semiprime)?;
    let elapsed = start.elapsed();

    println!("✅ Factorization complete in {:.2}s", elapsed.as_secs_f64());

    // Use the smaller prime for encryption
    let (smaller, _larger) = if p < q { (p, q) } else { (q, p) };

    println!("🔐 Encrypting location...");
    let encrypted_location = encrypt_location(location, &smaller)?;

    let response = Response {
        encrypted_location,
        challenge_id: challenge.id,
    };

    let base64_response = encode_response_base64(&response)?;

    println!("📤 Response generated successfully!");
    println!("📋 Share this response string:");
    println!("================================================================================");
    println!("{}", base64_response);
    println!("================================================================================");

    Ok(())
}

pub fn handle_verify(challenge_input: &str, response_input: &str, location: &str) -> Result<()> {
    let challenge = decode_challenge(challenge_input)?;
    let response = decode_response(response_input)?;

    if challenge.id != response.challenge_id {
        return Err(anyhow!("Challenge ID mismatch"));
    }

    let locations = load_locations()?;
    if !locations.contains(&location.to_string()) {
        return Err(anyhow!(
            "Location '{}' not found in locations list",
            location
        ));
    }

    println!("🔍 Verifying response for location: {}", location);

    // Select the correct semiprime for the known location
    let semiprime = select_semiprime_for_location(&challenge.semiprimes, location)?;

    println!("⚡ Performing proof-of-work for {}...", location);
    let start = std::time::Instant::now();

    let (p, q) = factor_semiprime(&semiprime)?;
    let elapsed = start.elapsed();

    println!("✅ Factorization complete in {:.2}s", elapsed.as_secs_f64());

    // Try both primes as decryption keys
    let keys = [&p, &q];
    for key in &keys {
        match decrypt_location(&response.encrypted_location, key) {
            Ok(decrypted) => {
                if decrypted == location {
                    println!("🎉 VERIFICATION SUCCESSFUL!");
                    println!("✅ The responder knows the location: {}", location);
                    return Ok(());
                } else {
                    println!("❌ VERIFICATION FAILED!");
                    println!(
                        "🕵️ Decrypted location '{}' doesn't match expected location '{}'",
                        decrypted, location
                    );
                    println!("🕵️ The responder appears to be the spy (doesn't know the location)");
                    return Ok(());
                }
            }
            Err(_) => continue,
        }
    }

    println!("❌ VERIFICATION FAILED!");
    println!("🕵️ Could not decrypt the response with the correct key");
    println!("🕵️ The responder appears to be the spy (doesn't know the location)");
    Ok(())
}

pub fn handle_brute(challenge_input: &str, response_input: &str) -> Result<()> {
    let challenge = decode_challenge(challenge_input)?;
    let response = decode_response(response_input)?;

    if challenge.id != response.challenge_id {
        return Err(anyhow!("Challenge ID mismatch"));
    }

    let locations = load_locations()?;
    println!("🕵️ SPY MODE: Brute forcing all locations in parallel...");
    println!(
        "⚡ Using {} CPU cores for maximum speed",
        rayon::current_num_threads()
    );

    let start_total = std::time::Instant::now();

    // Shared flag to signal when a location is found
    let found = Arc::new(AtomicBool::new(false));

    // Use parallel iterator to test all locations simultaneously
    let result = locations
        .par_iter()
        .enumerate()
        .find_map_any(|(_i, location)| {
            // Check if another thread already found the answer
            if found.load(Ordering::Relaxed) {
                println!("🛑 [Thread {}] Cancelling work for {} - location already found", rayon::current_thread_index().unwrap_or(0), location);
                return None;
            }

            println!("🧪 [Thread {}] Testing location: {}", rayon::current_thread_index().unwrap_or(0), location);

            // Select the semiprime for this location
            let semiprime = match select_semiprime_for_location(&challenge.semiprimes, location) {
                Ok(sp) => sp,
                Err(_) => return None,
            };

            // Check again before starting expensive factorization
            if found.load(Ordering::Relaxed) {
                println!("🛑 [Thread {}] Cancelling factorization for {} - location already found", rayon::current_thread_index().unwrap_or(0), location);
                return None;
            }

            println!("⚡ [Thread {}] Factoring semiprime for {}...", rayon::current_thread_index().unwrap_or(0), location);
            let start = std::time::Instant::now();

            // Factor the semiprime
            let (p, q) = match factor_semiprime(&semiprime) {
                Ok(factors) => factors,
                Err(_) => {
                    println!("❌ [Thread {}] Failed to factor semiprime for {}", rayon::current_thread_index().unwrap_or(0), location);
                    return None;
                }
            };

            let elapsed = start.elapsed();

            // Check one more time before decryption
            if found.load(Ordering::Relaxed) {
                println!("🛑 [Thread {}] Cancelling decryption for {} - location already found", rayon::current_thread_index().unwrap_or(0), location);
                return None;
            }

            println!("✅ [Thread {}] Factorization for {} complete in {:.2}s", rayon::current_thread_index().unwrap_or(0), location, elapsed.as_secs_f64());

            // Try both primes as decryption keys
            let keys = [&p, &q];
            for key in &keys {
                match decrypt_location(&response.encrypted_location, key) {
                    Ok(decrypted) => {
                        if decrypted == *location {
                            // Signal other threads to stop
                            found.store(true, Ordering::Relaxed);
                            println!("🎯 [Thread {}] LOCATION FOUND! Signalling other threads to stop...", rayon::current_thread_index().unwrap_or(0));
                            return Some((location.clone(), elapsed));
                        }
                    }
                    Err(_) => continue,
                }
            }
            None
        });

    let total_elapsed = start_total.elapsed();

    match result {
        Some((location, factor_time)) => {
            println!("\n🎉 BRUTE FORCE SUCCESSFUL!");
            println!("🎯 The secret location is: {}", location);
            println!("⏱️  Factorization time: {:.2}s", factor_time.as_secs_f64());
            println!("⏱️  Total time: {:.2}s", total_elapsed.as_secs_f64());
        }
        None => {
            println!("\n❌ BRUTE FORCE FAILED!");
            println!("⏱️  Total time: {:.2}s", total_elapsed.as_secs_f64());
            println!("🕵️ Could not determine the location from any semiprime");
        }
    }

    Ok(())
}

pub fn handle_locations() -> Result<()> {
    let locations = load_locations()?;

    println!("📍 Available Spyfall Locations:");
    println!("===============================");

    for (i, location) in locations.iter().enumerate() {
        println!("{:2}. {}", i + 1, location);
    }

    println!("===============================");
    println!("Total: {} locations", locations.len());

    // Check if using default or file-based locations
    if std::path::Path::new(LOCATIONS_FILE).exists() {
        println!("📄 Loaded from: {}", LOCATIONS_FILE);
    } else {
        println!(
            "📄 Using built-in default locations (no {} found)",
            LOCATIONS_FILE
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_function() {
        let result = challenge("test input".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_respond_function() {
        let result = respond("test input".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_function() {
        let result = verify("test input".to_string());
        assert!(result.is_ok());
    }
}
