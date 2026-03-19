use indexmap::IndexMap;

/// A service to run via process-compose.
#[derive(Debug, Clone)]
pub struct InfraService {
    pub name: String,
    pub image: String,
    pub command: Option<String>,
    pub environment: IndexMap<String, String>,
    pub healthcheck_cmd: Option<String>,
    pub healthcheck_interval_secs: Option<u64>,
    pub healthcheck_retries: Option<u64>,
}

/// Generate process-compose.yml content from infrastructure services.
pub fn generate(services: &[InfraService]) -> String {
    let mut out = String::new();
    out.push_str("version: \"0.5\"\n\nprocesses:\n");

    for svc in services {
        out.push_str(&format!("  {}:\n", svc.name));

        // Command
        let cmd = svc.command.clone().unwrap_or_else(|| {
            default_command_for_image(&svc.image, &svc.environment)
        });
        if cmd.contains('\n') {
            out.push_str("    command: |\n");
            for line in cmd.lines() {
                out.push_str(&format!("      {line}\n"));
            }
        } else {
            out.push_str(&format!("    command: \"{cmd}\"\n"));
        }

        // Environment
        if !svc.environment.is_empty() {
            out.push_str("    environment:\n");
            for (k, v) in &svc.environment {
                out.push_str(&format!("      - \"{k}={v}\"\n"));
            }
        }

        // Readiness probe
        if let Some(ref hc) = svc.healthcheck_cmd {
            out.push_str("    readiness_probe:\n");
            out.push_str("      exec:\n");
            out.push_str(&format!("        command: \"{hc}\"\n"));
            if let Some(interval) = svc.healthcheck_interval_secs {
                out.push_str(&format!("      period_seconds: {interval}\n"));
            }
            if let Some(retries) = svc.healthcheck_retries {
                out.push_str(&format!("      failure_threshold: {retries}\n"));
            }
        }

        out.push('\n');
    }

    out
}

fn default_command_for_image(image: &str, env: &IndexMap<String, String>) -> String {
    match image {
        "postgres" | "postgresql" => {
            let port = env
                .iter()
                .find(|(k, _)| k.contains("PORT"))
                .map(|(_, v)| v.as_str())
                .unwrap_or("5432");
            format!(
                "export PGDATA=\"${{PGDATA:-./.pgdata}}\"\n\
                 if [ ! -d \"$PGDATA\" ]; then\n\
                 \x20 initdb -D \"$PGDATA\" -U postgres\n\
                 fi\n\
                 postgres -D \"$PGDATA\" -p {port}"
            )
        }
        "redis" => "redis-server".to_string(),
        _ => format!("echo 'No default command for {image}; please configure manually'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_postgres() {
        let svc = InfraService {
            name: "postgres".into(),
            image: "postgres".into(),
            command: Some("postgres -c log_statement=all -p 3003".into()),
            environment: IndexMap::from([
                ("POSTGRES_USER".into(), "postgres".into()),
                ("POSTGRES_PASSWORD".into(), "postgres".into()),
            ]),
            healthcheck_cmd: Some("pg_isready -U postgres -d postgres -p 3003".into()),
            healthcheck_interval_secs: Some(5),
            healthcheck_retries: Some(3),
        };
        let output = generate(&[svc]);
        assert!(output.contains("postgres:"));
        assert!(output.contains("pg_isready"));
        assert!(output.contains("POSTGRES_USER=postgres"));
    }
}
