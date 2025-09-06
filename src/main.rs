use anyhow::Result;
use clap::{Parser, Subcommand};
use spyfall::{handle_brute, handle_challenge, handle_locations, handle_respond, handle_verify};

#[derive(Parser)]
#[command(name = "spyfall")]
#[command(about = "A cryptographic Spyfall CLI game")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a cryptographic challenge for a given location
    Challenge {
        /// The secret location for this round
        location: String,
    },
    /// Respond to a challenge by proving knowledge of the location
    Respond {
        /// The challenge (base64 string or JSON)
        challenge: String,
        /// The location you know
        location: String,
    },
    /// Verify a response to determine if the responder knows the location
    Verify {
        /// The challenge (base64 string or JSON)
        challenge: String,
        /// The response (base64 string or JSON) to verify
        response: String,
        /// The location you know (for verification)
        location: String,
    },
    /// Brute force all locations to find which one the responder knows (spy mode)
    Brute {
        /// The challenge (base64 string or JSON)
        challenge: String,
        /// The response (base64 string or JSON) to brute force
        response: String,
    },
    /// List all available locations
    Locations,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Challenge { location } => handle_challenge(&location),
        Commands::Respond {
            challenge,
            location,
        } => handle_respond(&challenge, &location),
        Commands::Verify {
            challenge,
            response,
            location,
        } => handle_verify(&challenge, &response, &location),
        Commands::Brute {
            challenge,
            response,
        } => handle_brute(&challenge, &response),
        Commands::Locations => handle_locations(),
    }
}
