use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod compose;
mod dockerfile;
mod mapping;
mod nix;

#[derive(Parser)]
#[command(name = "docker2nix", about = "Generate Nix Flake devShell from Dockerfile/docker-compose.yml")]
struct Cli {
    /// Path to Dockerfile
    #[arg(short = 'f', long = "file")]
    dockerfile: Option<PathBuf>,

    /// Path to docker-compose.yml
    #[arg(short = 'c', long = "compose")]
    compose: Option<PathBuf>,

    /// Output file path (default: stdout)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Dry run (print to stdout even if -o is set)
    #[arg(long)]
    dry_run: bool,

    /// Generate per-service devShells (compose only)
    #[arg(long)]
    per_service: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.dockerfile.is_none() && cli.compose.is_none() {
        bail!("specify -f <Dockerfile> or -c <compose.yaml>");
    }

    if cli.per_service && cli.compose.is_none() {
        bail!("--per-service requires -c <compose.yaml>");
    }

    let (flake_nix, process_compose_yml) = if let Some(compose_path) = &cli.compose {
        // Compose mode
        let compose_content =
            std::fs::read_to_string(compose_path).context("failed to read compose file")?;
        let compose_file = compose::parser::parse(&compose_content)?;

        // Find Dockerfile path relative to compose file
        let compose_dir = compose_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let dockerfile_path = cli
            .dockerfile
            .clone()
            .unwrap_or_else(|| compose_dir.join("Dockerfile"));
        let dockerfile_content =
            std::fs::read_to_string(&dockerfile_path).context("failed to read Dockerfile")?;
        let dockerfile = dockerfile::parser::parse(&dockerfile_content);

        let (shells, infra_services) =
            compose::resolve::resolve(&compose_file, &dockerfile, cli.per_service);

        let flake = nix::flake_gen::generate_flake(&shells, !infra_services.is_empty());
        let pc = if infra_services.is_empty() {
            None
        } else {
            Some(nix::process_compose::generate(&infra_services))
        };
        (flake, pc)
    } else if let Some(dockerfile_path) = &cli.dockerfile {
        // Dockerfile-only mode
        let content =
            std::fs::read_to_string(dockerfile_path).context("failed to read Dockerfile")?;
        let dockerfile = dockerfile::parser::parse(&content);
        let shell = compose::resolve::resolve_dockerfile(&dockerfile);
        let flake = nix::flake_gen::generate_flake(&[shell], false);
        (flake, None)
    } else {
        unreachable!()
    };

    // Output flake.nix
    if cli.dry_run || cli.output.is_none() {
        println!("{flake_nix}");
    } else if let Some(ref out) = cli.output {
        std::fs::write(out, &flake_nix).context("failed to write flake.nix")?;
        eprintln!("wrote {}", out.display());
    }

    // Output process-compose.yml
    if let Some(pc) = process_compose_yml {
        if cli.dry_run || cli.output.is_none() {
            eprintln!("--- process-compose.yml ---");
            eprintln!("{pc}");
        } else if let Some(ref out) = cli.output {
            let pc_path = out.parent().unwrap_or_else(|| std::path::Path::new(".")).join("process-compose.yml");
            std::fs::write(&pc_path, &pc).context("failed to write process-compose.yml")?;
            eprintln!("wrote {}", pc_path.display());
        }
    }

    Ok(())
}
