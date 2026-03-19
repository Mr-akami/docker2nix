use std::collections::HashMap;

use indexmap::IndexMap;

use crate::compose::parser::{ComposeFile, Service};
use crate::dockerfile::apt;
use crate::dockerfile::{Dockerfile, Instruction, Stage};
use crate::mapping::apt_to_nix;
use crate::nix::process_compose::InfraService;
use crate::nix::DevShellConfig;

/// Resolve a compose file + Dockerfile into devShell configs and infrastructure services.
///
/// Returns (shells, infra_services).
/// - In default mode: one merged "default" shell + infra services for process-compose.
/// - In per_service mode: one shell per service + infra services.
pub fn resolve(
    compose: &ComposeFile,
    dockerfile: &Dockerfile,
    per_service: bool,
) -> (Vec<DevShellConfig>, Vec<InfraService>) {
    let mut app_shells: Vec<DevShellConfig> = Vec::new();
    let mut infra_services: Vec<InfraService> = Vec::new();

    for (svc_name, svc) in &compose.services {
        let target = svc.build_target().unwrap_or("");
        let chain = build_stage_chain(dockerfile, target);

        // Check if this is an infrastructure service
        let base_image = chain
            .first()
            .map(|s| s.from.image.as_str())
            .unwrap_or("");

        if apt_to_nix::is_infrastructure_image(base_image) {
            let isvc = build_infra_service(svc_name, svc, base_image);
            infra_services.push(isvc);

            if per_service {
                // Also create a devShell for infra in per-service mode
                let mut shell = DevShellConfig::new(svc_name.clone());
                if let Some(pkgs) = apt_to_nix::base_image_to_nix(base_image) {
                    for pkg in pkgs {
                        shell.add_input(pkg);
                    }
                }
                for (k, v) in svc.env_map() {
                    shell.env_vars.insert(k, v);
                }
                app_shells.push(shell);
            }
        } else {
            let shell = build_shell_from_chain(svc_name, &chain, svc);
            app_shells.push(shell);
        }
    }

    if per_service {
        // In per-service mode, mark first app shell as "default" if no explicit default
        if !app_shells.is_empty() {
            app_shells[0].name = "default".to_string();
        }
        (app_shells, infra_services)
    } else {
        // Merge all app shells into one default shell
        let mut merged = DevShellConfig::new("default");
        for shell in &app_shells {
            merged.merge(shell);
        }
        // Also add infra packages to merged shell for convenience
        for isvc in &infra_services {
            if let Some(pkgs) = apt_to_nix::base_image_to_nix(&isvc.image) {
                for pkg in pkgs {
                    merged.add_input(pkg);
                }
            }
        }
        (vec![merged], infra_services)
    }
}

/// Resolve a standalone Dockerfile (no compose) into a single devShell config.
pub fn resolve_dockerfile(dockerfile: &Dockerfile) -> DevShellConfig {
    let mut shell = DevShellConfig::new("default");

    for stage in &dockerfile.stages {
        collect_from_stage(stage, &mut shell);

        // Map base image
        if let Some(pkgs) = apt_to_nix::base_image_to_nix(&stage.from.image) {
            for pkg in pkgs {
                shell.add_input(pkg);
            }
        }
    }

    shell
}

fn build_stage_chain<'a>(dockerfile: &'a Dockerfile, target: &str) -> Vec<&'a Stage> {
    if target.is_empty() {
        // No target specified, use last stage
        return dockerfile.stages.last().map(|s| vec![s]).unwrap_or_default();
    }

    let stage_map: HashMap<&str, &Stage> = dockerfile
        .stages
        .iter()
        .filter_map(|s| s.name.as_deref().map(|n| (n, s)))
        .collect();

    let mut chain = Vec::new();
    let mut current = stage_map.get(target).copied();

    while let Some(stage) = current {
        chain.push(stage);
        current = stage_map.get(stage.from.image.as_str()).copied();
    }

    chain.reverse();
    chain
}

fn build_shell_from_chain(
    svc_name: &str,
    chain: &[&Stage],
    svc: &Service,
) -> DevShellConfig {
    let mut shell = DevShellConfig::new(svc_name.to_string());

    // Map base image of the root stage
    if let Some(first) = chain.first() {
        if let Some(pkgs) = apt_to_nix::base_image_to_nix(&first.from.image) {
            for pkg in pkgs {
                shell.add_input(pkg);
            }
        }
    }

    // Collect from each stage in the chain
    for stage in chain {
        collect_from_stage(stage, &mut shell);
    }

    // Add compose-level environment
    for (k, v) in svc.env_map() {
        shell.env_vars.insert(k, v);
    }

    shell
}

fn collect_from_stage(stage: &Stage, shell: &mut DevShellConfig) {
    let mut unmapped: Vec<String> = Vec::new();

    for inst in &stage.instructions {
        match inst {
            Instruction::Run(cmd) => {
                // Extract and map apt packages
                let apt_pkgs = apt::extract_apt_packages(cmd);
                for pkg in &apt_pkgs {
                    match apt_to_nix::apt_to_nix(pkg) {
                        Some(nix_pkgs) => {
                            for np in nix_pkgs {
                                shell.add_input(np);
                            }
                        }
                        None => unmapped.push(pkg.clone()),
                    }
                }
            }
            Instruction::Env(key, val) => {
                // Skip internal Docker vars and ARG references
                if key == "DEBIAN_FRONTEND" {
                    continue;
                }
                // Detect NODE_VERSION → add nodejs
                if key == "NODE_VERSION" {
                    shell.add_input("nodejs");
                }
                // Don't include env vars that just reference ARGs like $EB_GH_TOKEN
                if !val.starts_with('$') {
                    shell.env_vars.insert(key.clone(), val.clone());
                }
            }
            _ => {}
        }
    }

    // Warn about unmapped packages
    for pkg in &unmapped {
        eprintln!("warning: unmapped apt package: {pkg}");
    }
}

fn build_infra_service(
    svc_name: &str,
    svc: &Service,
    base_image: &str,
) -> InfraService {
    InfraService {
        name: svc_name.to_string(),
        image: base_image.to_string(),
        command: svc.command_string(),
        environment: svc.env_map(),
        healthcheck_cmd: svc.healthcheck_cmd(),
        healthcheck_interval_secs: svc.healthcheck_interval_secs(),
        healthcheck_retries: svc.healthcheck_retries(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::parser;
    use crate::dockerfile::parser as df_parser;

    fn test_dockerfile() -> &'static str {
        r#"FROM ubuntu:22.04 AS base
ENV NODE_VERSION=20.0.0
RUN apt update && apt install -y curl build-essential && apt clean

FROM base AS deps
WORKDIR /app

FROM deps AS server
RUN apt update && apt install -y git python3-pip && apt clean
ENV PROJ_LIB=/usr/share/proj
"#
    }

    fn test_compose() -> &'static str {
        r#"
services:
  dev:
    build:
      context: .
      target: server
    environment:
      - IS_UNIX=true
    depends_on:
      - postgres
  postgres:
    build:
      context: .
      target: postgres
    environment:
      POSTGRES_USER: "postgres"
      POSTGRES_PASSWORD: "postgres"
    command: postgres -p 5432
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      retries: 3
"#
    }

    #[test]
    fn test_resolve_dockerfile_only() {
        let df = df_parser::parse(test_dockerfile());
        let shell = resolve_dockerfile(&df);
        assert!(shell.build_inputs.contains(&"curl".to_string()));
        assert!(shell.build_inputs.contains(&"nodejs".to_string()));
        assert!(shell.build_inputs.contains(&"git".to_string()));
        assert_eq!(shell.env_vars.get("NODE_VERSION").unwrap(), "20.0.0");
    }

    #[test]
    fn test_resolve_compose_default() {
        let df_content = format!(
            "{}\nFROM postgres:16 AS postgres\n",
            test_dockerfile()
        );
        let df = df_parser::parse(&df_content);
        let compose = parser::parse(test_compose()).unwrap();
        let (shells, infra) = resolve(&compose, &df, false);

        assert_eq!(shells.len(), 1);
        assert_eq!(shells[0].name, "default");
        // Should have packages from dev chain + postgres
        assert!(shells[0].build_inputs.contains(&"curl".to_string()));
        assert!(shells[0].build_inputs.contains(&"postgresql".to_string()));
        assert_eq!(shells[0].env_vars.get("IS_UNIX").unwrap(), "true");

        assert_eq!(infra.len(), 1);
        assert_eq!(infra[0].name, "postgres");
    }

    #[test]
    fn test_resolve_compose_per_service() {
        let df_content = format!(
            "{}\nFROM postgres:16 AS postgres\n",
            test_dockerfile()
        );
        let df = df_parser::parse(&df_content);
        let compose = parser::parse(test_compose()).unwrap();
        let (shells, infra) = resolve(&compose, &df, true);

        assert!(shells.len() >= 2);
        assert_eq!(infra.len(), 1);
    }

    #[test]
    fn test_stage_chain() {
        let df = df_parser::parse(test_dockerfile());
        let chain = build_stage_chain(&df, "server");
        assert_eq!(chain.len(), 3); // base -> deps -> server
        assert_eq!(chain[0].name.as_deref(), Some("base"));
        assert_eq!(chain[2].name.as_deref(), Some("server"));
    }
}
