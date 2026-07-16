//! `zmanim` - a command-line tool for computing Jewish zmanim.

mod cli;
mod commands;
mod compute;
mod config;
mod dates;
mod output;
mod resolve;
mod zman_names;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, Command};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        // list zmanim
        Some(Command::List(args)) => {
            commands::list::run(args);
            Ok(())
        }

        // manage configured locations
        Some(Command::Locations(args)) => commands::locations::run(args, cli.config.clone()),

        // generate shell completions
        Some(Command::Completions(args)) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(args.shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }

        // no subcommand = default = calculate zmanim
        None => run_compute(&cli),
    }
}

fn run_compute(cli: &Cli) -> Result<()> {
    // resolve settings from args, env vars, config, & defaults
    let path = config::config_path(cli.config.clone())?;
    let config = config::load(&path)?;
    let settings = resolve::resolve(&cli.compute, &config)?;

    // compute zmanim from resolved settings
    let grid = compute::compute(&settings);

    // render & print computed zmanim based on display settings
    let out = output::render(&grid, &settings);
    print!("{out}");
    Ok(())
}
