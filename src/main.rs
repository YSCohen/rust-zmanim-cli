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
use clap_complete::Shell;
use clap_complete_nushell::Nushell;
use cli::{Cli, Command, CompletionShell};

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
            let out = &mut std::io::stdout();
            match args.shell {
                CompletionShell::Bash => clap_complete::generate(Shell::Bash, &mut cmd, name, out),
                CompletionShell::Elvish => {
                    clap_complete::generate(Shell::Elvish, &mut cmd, name, out)
                }
                CompletionShell::Fish => clap_complete::generate(Shell::Fish, &mut cmd, name, out),
                CompletionShell::Nushell => clap_complete::generate(Nushell, &mut cmd, name, out),
                CompletionShell::PowerShell => {
                    clap_complete::generate(Shell::PowerShell, &mut cmd, name, out)
                }
                CompletionShell::Zsh => clap_complete::generate(Shell::Zsh, &mut cmd, name, out),
            }
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
