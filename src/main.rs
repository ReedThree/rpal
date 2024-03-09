use clap::Parser;
use rpal::run;
use rpal::CLIError;
use rpal::Cli;
fn main() -> Result<(), CLIError> {
    let cli = Cli::parse();

    run(cli)?;

    Ok(())
}
