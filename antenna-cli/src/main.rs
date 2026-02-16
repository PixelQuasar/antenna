use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "cargo-antenna")]
#[command(bin_name = "cargo-antenna")]
enum Cli {
    Antenna(AntennaArgs),
}

#[derive(clap::Args)]
struct AntennaArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(long, default_value = "./shared")]
        shared: String,

        #[arg(long, default_value = "./client")]
        client: String,

        #[arg(short, long, default_value = "./frontend/src/antenna")]
        out: String,
    },
}

fn main() -> Result<()> {
    let Cli::Antenna(args) = Cli::parse();

    match args.command {
        Commands::Build {
            shared,
            client,
            out,
        } => {
            println!("{}", "🚀 Starting Antenna Build...".green().bold());

            let out_path = Path::new(&out);
            let types_path = out_path.join("types");
            let wasm_path = out_path.join("wasm");

            if out_path.exists() {
                fs::remove_dir_all(out_path)?;
            }
            fs::create_dir_all(&types_path)?;
            fs::create_dir_all(&wasm_path)?;

            println!("{}", "📦 Generating TypeScript definitions...".cyan());
            run_type_gen(&shared, &types_path)?;

            println!("{}", "📦 Compiling WebAssembly...".cyan());
            run_wasm_pack(&client, &wasm_path)?;

            println!("{}", "✨ Build completed successfully!".green().bold());
            println!("   📂 Types: {}", types_path.display());
            println!("   📂 WASM:  {}", wasm_path.display());
        }
    }

    Ok(())
}

fn run_type_gen(shared_path: &str, out_dir: &Path) -> Result<()> {
    let out_abs = fs::canonicalize(out_dir).unwrap_or(out_dir.to_path_buf());
    let status = Command::new("cargo")
        .arg("test")
        .env("TS_RS_EXPORT_DIR", out_abs)
        .current_dir(shared_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run cargo test for type generation")?;

    if !status.success() {
        anyhow::bail!("Type generation failed");
    }
    Ok(())
}

fn run_wasm_pack(client_path: &str, out_dir: &Path) -> Result<()> {
    let out_abs = fs::canonicalize(out_dir).unwrap_or(out_dir.to_path_buf());
    let status = Command::new("wasm-pack")
        .args(["build", "--target", "web", "--out-dir"])
        .arg(out_abs)
        .current_dir(client_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run wasm-pack. Is it installed?")?;

    if !status.success() {
        anyhow::bail!("WASM build failed");
    }
    Ok(())
}
