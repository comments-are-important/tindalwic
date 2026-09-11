use bumpalo::Bump;
use clap::{Parser, Subcommand};
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use tindalwic::bumpalo::Arena;
use tindalwic::{Entry, Item};
use tindalwic_cli::random::Random;

fn parse_hex(s: &str) -> Result<u64, std::num::ParseIntError> {
    u64::from_str_radix(s, 16)
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// generate a file with random structure and values
    Random {
        /// how many nodes?
        #[arg(default_value_t = 10)]
        nodes: usize,
        /// use full range of chars instead of just lower-case ascii
        #[arg(long)]
        unicode: bool,
        /// specify the random seed
        #[arg(long, value_parser = parse_hex)]
        seed: Option<u64>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Random {
            nodes,
            unicode,
            seed,
        } => {
            let mut rng;
            let hex = if let Some(provided) = seed {
                rng = SmallRng::seed_from_u64(*provided);
                String::new()
            } else {
                let seed = rand::rng().random();
                rng = SmallRng::seed_from_u64(seed);
                format!("{:X}", seed)
            };
            let bump = &Bump::new();
            let arena = &mut Arena::new(bump);
            let mut random = Random::new(
                bump,
                arena,
                &mut rng,
                if *unicode {
                    ""
                } else {
                    "abcdefghijklmnopqrstuvwxyz"
                },
            )?;
            let file = if seed.is_some() {
                random.file(*nodes)
            } else {
                random.embedded(
                    "data".into(),
                    *nodes,
                    Entry {
                        key: "--seed".into(),
                        item: Item::text(&hex),
                        ..Default::default()
                    },
                )
            }?;
            print!("{}", file);
        }
    }
    Ok(())
}
