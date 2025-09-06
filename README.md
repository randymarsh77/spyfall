# Spyfall CLI

A command-line interface tool for spyfall operations.

# Cryptographic Spyfall CLI

A cryptographic implementation of the popular party game Spyfall, where players prove knowledge of secret locations using proof-of-work and zero-knowledge verification techniques.

## Game Overview

Spyfall is a social deduction game where:
- All players know a set of possible locations
- One location is chosen for the round
- Everyone except the "spy" knows the location
- Players must prove they know the location without revealing it

This cryptographic version replaces social deduction with mathematical proofs using:
- **Proof-of-work** via integer factorization
- **Zero-knowledge verification** through encryption
- **Parallel brute-force attacks** for sophisticated spies

## Installation & Setup

### Using Nix (Recommended)

```bash
git clone <repository>
cd spyfall
nix develop  # Enter development environment with Rust toolchain
```

### Manual Setup

Requires Rust 1.70+ and Cargo:

```bash
git clone <repository>
cd spyfall
cargo build
```

## Commands

### Development Aliases (in Nix shell)

- `build` - Compile the project (`cargo build`)
- `test` - Run tests (`cargo test`)
- `lint` - Run linter (`cargo clippy`)
- `format` - Format code (`cargo fmt`)
- `spyfall <args>` - Run CLI (`cargo run -- <args>`)

### Game Commands

#### 1. List Available Locations
```bash
spyfall locations
```
Shows all available locations (28 by default). Works with custom `locations.json` or built-in defaults.

#### 2. Generate Challenge
```bash
spyfall challenge "hotel"
```
- Creates cryptographic challenge for a specific location
- Generates semiprimes (one per location) for proof-of-work
- Outputs base64-encoded challenge string

#### 3. Respond to Challenge
```bash
spyfall respond "<base64-challenge>" "hotel"
```
- Proves knowledge of the location through factorization
- Encrypts location with derived key
- Outputs base64-encoded response

#### 4. Verify Response (Honest Players)
```bash
spyfall verify "<base64-challenge>" "<base64-response>" "hotel"
```
- Verifies that responder knows the correct location
- Requires knowing the location yourself
- Uses zero-knowledge proof verification

#### 5. Brute Force Attack (Spy Mode)
```bash
spyfall brute "<base64-challenge>" "<base64-response>"
```
- Attempts all locations in parallel
- Uses all CPU cores for maximum speed
- Designed to give spies the best possible advantage

## Example Workflow

```bash
# Player 1 (Questioner) generates challenge
spyfall challenge "hotel"
# Outputs: eyJzZW1pcHJpbWVzIjpbIjE2N...

# Player 2 (Responder) proves they know the location
spyfall respond "eyJzZW1pcHJpbWVzIjpbIjE2N..." "hotel"
# Outputs: eyJlbmNyeXB0ZWRfbG9jYXRpb24...

# Player 3 (Verifier) checks the response
spyfall verify "eyJzZW1pcHJpbWVzIjpbIjE2N..." "eyJlbmNyeXB0ZWRfbG9jYXRpb24..." "hotel"
# Outputs: ✅ The responder knows the location: hotel

# Spy attempts brute force
spyfall brute "eyJzZW1pcHJpbWVzIjpbIjE2N..." "eyJlbmNyeXB0ZWRfbG9jYXRpb24..."
# Outputs: 🎯 The secret location is: hotel
```

## Locations

The game includes 28 default locations:
- airplane, bank, beach, casino, cathedral, circus_tent
- corporate_party, crusader_army, day_spa, embassy
- hospital, hotel, military_base, movie_studio
- ocean_liner, passenger_train, pirate_ship, polar_station
- police_station, restaurant, school, service_station
- space_station, submarine, supermarket, theater
- university, world_war_ii_squad

Custom locations can be added via `locations.json`.

## Technical Details

### Cryptographic Components

- **Semiprimes**: Products of two large primes (configurable bit size)
- **Factorization**: Proof-of-work using trial division + Pollard's rho
- **Encryption**: AES-256-GCM with SHA-256 key derivation
- **Encoding**: Base64 for easy copy/paste sharing

### Security Model

- **Honest players**: Factor 1 semiprime (~seconds)
- **Spies**: Must factor N semiprimes in parallel
- **Defense**: Increase prime size or location count
- **Attack**: Add CPU cores or specialized hardware

### Performance

With 48-bit primes (96-bit semiprimes):
- Single factorization: 0.1-2 seconds
- Parallel brute force: 0.1-2 seconds with 8+ cores
- Memory usage: <10MB
- Challenge size: ~2KB base64-encoded

## Configuration

### Adjusting Difficulty

Edit `src/lib.rs`:
```rust
const PRIME_BITS: usize = 48; // Increase for harder factorization
```

### Custom Locations

Create `locations.json`:
```json
[
  "your_location_1",
  "your_location_2",
  "..."
]
```

## Development

### Project Structure

```
src/
├── main.rs       # CLI interface and argument parsing
├── lib.rs        # Core cryptographic implementation
tests/
├── integration_tests.rs
locations.json    # Custom location list (optional)
flake.nix        # Nix development environment
Cargo.toml       # Rust dependencies
```

### Key Dependencies

- `clap` - Command-line interface
- `num-bigint` - Large integer arithmetic
- `aes-gcm` - Authenticated encryption
- `rayon` - Parallel processing
- `serde` - JSON serialization

### Contributing

1. Enter development environment: `nix develop`
2. Make changes
3. Test: `test`
4. Lint: `lint`
5. Format: `format`
6. Build: `build`

## Security Considerations

This is a **research prototype** demonstrating cryptographic concepts. Not suitable for production security applications.

### Known Limitations

- Parallel attacks can dramatically reduce security
- Fixed location set limits entropy
- No formal security proofs
- Performance varies significantly with hardware

### Educational Value

Demonstrates:
- Proof-of-work systems and their limitations
- Zero-knowledge verification concepts
- Cryptographic protocol design
- Arms race between honest users and adversaries

## License

MIT License - see LICENSE file for details.

## Further Reading

See `blog.md` for detailed technical explanation and analysis of the cryptographic approach.

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Testing

```bash
cargo test
```

## Code Quality

```bash
# Run linter
cargo clippy

# Format code
cargo fmt

# Auto-rebuild on changes
cargo watch -x run
```
