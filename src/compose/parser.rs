use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ComposeFile {
    pub services: IndexMap<String, Service>,
}

#[derive(Debug, Deserialize)]
pub struct Service {
    pub build: Option<BuildConfig>,
    pub image: Option<String>,
    pub ports: Option<Vec<String>>,
    pub environment: Option<Environment>,
    pub depends_on: Option<DependsOn>,
    pub command: Option<Command>,
    pub healthcheck: Option<Healthcheck>,
    pub volumes: Option<serde_yaml::Value>,
    pub entrypoint: Option<Command>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BuildConfig {
    Simple(String),
    Detailed(BuildDetails),
}

#[derive(Debug, Deserialize)]
pub struct BuildDetails {
    pub context: Option<String>,
    pub dockerfile: Option<String>,
    pub target: Option<String>,
    pub args: Option<IndexMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Environment {
    List(Vec<String>),
    Map(IndexMap<String, serde_yaml::Value>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependsOn {
    List(Vec<String>),
    Map(IndexMap<String, serde_yaml::Value>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Command {
    Simple(String),
    List(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct Healthcheck {
    pub test: Option<serde_yaml::Value>,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub retries: Option<u64>,
}

impl Service {
    pub fn build_target(&self) -> Option<&str> {
        match &self.build {
            Some(BuildConfig::Detailed(d)) => d.target.as_deref(),
            _ => None,
        }
    }

    /// Extract environment variables as key-value pairs.
    pub fn env_map(&self) -> IndexMap<String, String> {
        match &self.environment {
            Some(Environment::List(list)) => {
                let mut map = IndexMap::new();
                for item in list {
                    if let Some(eq) = item.find('=') {
                        map.insert(item[..eq].to_string(), item[eq + 1..].to_string());
                    }
                }
                map
            }
            Some(Environment::Map(m)) => m
                .iter()
                .map(|(k, v)| {
                    let val = match v {
                        serde_yaml::Value::String(s) => s.clone(),
                        other => format!("{other:?}"),
                    };
                    (k.clone(), val)
                })
                .collect(),
            None => IndexMap::new(),
        }
    }

    pub fn command_string(&self) -> Option<String> {
        match &self.command {
            Some(Command::Simple(s)) => Some(s.clone()),
            Some(Command::List(l)) => Some(l.join(" ")),
            None => None,
        }
    }

    pub fn healthcheck_cmd(&self) -> Option<String> {
        let hc = self.healthcheck.as_ref()?;
        let test = hc.test.as_ref()?;
        match test {
            serde_yaml::Value::Sequence(seq) => {
                // ["CMD-SHELL", "pg_isready ..."] or ["CMD", "pg_isready", ...]
                let strings: Vec<String> = seq
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if strings.first().map(|s| s.as_str()) == Some("CMD-SHELL") {
                    Some(strings[1..].join(" "))
                } else if strings.first().map(|s| s.as_str()) == Some("CMD") {
                    Some(strings[1..].join(" "))
                } else {
                    Some(strings.join(" "))
                }
            }
            serde_yaml::Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn healthcheck_interval_secs(&self) -> Option<u64> {
        let interval = self.healthcheck.as_ref()?.interval.as_ref()?;
        parse_duration_secs(interval)
    }

    pub fn healthcheck_retries(&self) -> Option<u64> {
        self.healthcheck.as_ref()?.retries
    }
}

fn parse_duration_secs(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('s') {
        n.parse().ok()
    } else if let Some(n) = s.strip_suffix('m') {
        n.parse::<u64>().ok().map(|m| m * 60)
    } else {
        s.parse().ok()
    }
}

pub fn parse(content: &str) -> Result<ComposeFile> {
    serde_yaml::from_str(content).context("failed to parse compose file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compose() {
        let yaml = r#"
services:
  web:
    build:
      context: .
      target: server
    environment:
      - FOO=bar
      - BAZ=qux
    ports:
      - "3000:3000"
  db:
    build:
      context: .
      target: postgres
    environment:
      POSTGRES_USER: "postgres"
    command: postgres -p 5432
    healthcheck:
      test: ["CMD-SHELL", "pg_isready"]
      interval: 5s
      retries: 3
"#;
        let compose = parse(yaml).unwrap();
        assert_eq!(compose.services.len(), 2);

        let web = &compose.services["web"];
        assert_eq!(web.build_target(), Some("server"));
        let env = web.env_map();
        assert_eq!(env["FOO"], "bar");

        let db = &compose.services["db"];
        assert_eq!(db.command_string().as_deref(), Some("postgres -p 5432"));
        assert_eq!(db.healthcheck_cmd().as_deref(), Some("pg_isready"));
        assert_eq!(db.healthcheck_interval_secs(), Some(5));
    }
}
