use super::{Dockerfile, FromDirective, Instruction, Stage};

pub fn parse(content: &str) -> Dockerfile {
    let lines = join_continuation_lines(content);
    let mut stages = Vec::new();
    let mut current_stage: Option<Stage> = None;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(from) = parse_from(trimmed) {
            if let Some(stage) = current_stage.take() {
                stages.push(stage);
            }
            current_stage = Some(Stage {
                name: from.alias.clone(),
                from,
                instructions: Vec::new(),
            });
        } else if let Some(ref mut stage) = current_stage {
            if let Some(instruction) = parse_instruction(trimmed) {
                stage.instructions.push(instruction);
            }
        }
    }

    if let Some(stage) = current_stage {
        stages.push(stage);
    }

    Dockerfile { stages }
}

fn join_continuation_lines(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        if current.is_empty() {
            current = line.to_string();
        } else {
            current.push(' ');
            current.push_str(line.trim());
        }

        if current.trim_end().ends_with('\\') {
            let trimmed_len = current.trim_end().len();
            current.truncate(trimmed_len - 1);
        } else {
            result.push(current);
            current = String::new();
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

fn parse_from(line: &str) -> Option<FromDirective> {
    let upper = line.to_uppercase();
    if !upper.starts_with("FROM ") {
        return None;
    }

    let rest = line[5..].trim();
    // Strip --platform=... flag
    let rest = if rest.starts_with("--") {
        rest.split_whitespace()
            .skip(1)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        rest.to_string()
    };

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let image_tag = tokens[0];
    let (image, tag) = if let Some(pos) = image_tag.find(':') {
        (
            image_tag[..pos].to_string(),
            Some(image_tag[pos + 1..].to_string()),
        )
    } else {
        (image_tag.to_string(), None)
    };

    // Look for AS alias (case-insensitive)
    let alias = if tokens.len() >= 3 && tokens[1].eq_ignore_ascii_case("as") {
        Some(tokens[2].to_string())
    } else {
        None
    };

    Some(FromDirective { image, tag, alias })
}

fn parse_instruction(line: &str) -> Option<Instruction> {
    let upper = line.to_uppercase();

    if upper.starts_with("RUN ") {
        let cmd = strip_run_flags(&line[4..]);
        Some(Instruction::Run(cmd.trim().to_string()))
    } else if upper.starts_with("ENV ") {
        let rest = line[4..].trim();
        // ENV KEY=VALUE or ENV KEY VALUE
        if let Some(eq_pos) = rest.find('=') {
            let key = rest[..eq_pos].trim().to_string();
            let val = rest[eq_pos + 1..].trim().to_string();
            // Handle $VAR references by keeping them as-is
            Some(Instruction::Env(key, val))
        } else {
            let mut parts = rest.splitn(2, ' ');
            let key = parts.next()?.trim().to_string();
            let val = parts.next().unwrap_or("").trim().to_string();
            Some(Instruction::Env(key, val))
        }
    } else if upper.starts_with("ARG ") {
        let rest = line[4..].trim();
        if let Some(eq_pos) = rest.find('=') {
            Some(Instruction::Arg(
                rest[..eq_pos].trim().to_string(),
                Some(rest[eq_pos + 1..].trim().to_string()),
            ))
        } else {
            Some(Instruction::Arg(rest.to_string(), None))
        }
    } else if upper.starts_with("WORKDIR ") {
        Some(Instruction::Workdir(line[8..].trim().to_string()))
    } else if upper.starts_with("EXPOSE ") {
        let port_str = line[7..].trim().split('/').next()?;
        port_str
            .parse::<u16>()
            .ok()
            .map(Instruction::Expose)
    } else if upper.starts_with("COPY ") {
        Some(Instruction::Copy(line[5..].trim().to_string()))
    } else if upper.starts_with("CMD ") {
        Some(Instruction::Cmd(line[4..].trim().to_string()))
    } else if upper.starts_with("ENTRYPOINT ") {
        Some(Instruction::Entrypoint(line[11..].trim().to_string()))
    } else {
        Some(Instruction::Other(line.to_string()))
    }
}

/// Strip BuildKit flags like --mount=type=cache,... from RUN instructions
fn strip_run_flags(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with("--") {
        // Skip flag tokens until we find one that doesn't start with --
        if let Some(pos) = s.find(|c: char| !c.is_ascii_graphic() || c == '&' || c == '|') {
            let remaining = s[pos..].trim();
            if remaining.starts_with("--") {
                return strip_run_flags(remaining);
            }
            return remaining;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_from_simple() {
        let from = parse_from("FROM ubuntu:22.04").unwrap();
        assert_eq!(from.image, "ubuntu");
        assert_eq!(from.tag.as_deref(), Some("22.04"));
        assert!(from.alias.is_none());
    }

    #[test]
    fn test_parse_from_with_alias() {
        let from = parse_from("FROM ubuntu:22.04 AS base").unwrap();
        assert_eq!(from.image, "ubuntu");
        assert_eq!(from.alias.as_deref(), Some("base"));
    }

    #[test]
    fn test_parse_from_lowercase_as() {
        let from = parse_from("FROM python:3.7 as roadAi").unwrap();
        assert_eq!(from.image, "python");
        assert_eq!(from.alias.as_deref(), Some("roadAi"));
    }

    #[test]
    fn test_continuation_lines() {
        let input = "RUN apt update && apt install -y \\\n curl \\\n git \\\n && apt clean";
        let lines = join_continuation_lines(input);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("curl"));
        assert!(lines[0].contains("git"));
        assert!(lines[0].contains("apt clean"));
    }

    #[test]
    fn test_parse_env() {
        let inst = parse_instruction("ENV NODE_VERSION=24.11.1").unwrap();
        match inst {
            Instruction::Env(k, v) => {
                assert_eq!(k, "NODE_VERSION");
                assert_eq!(v, "24.11.1");
            }
            _ => panic!("expected Env"),
        }
    }

    #[test]
    fn test_parse_full_dockerfile() {
        let content = "\
FROM ubuntu:22.04 AS base
ENV FOO=bar
RUN apt install -y curl

FROM base AS app
RUN apt install -y git
";
        let df = parse(content);
        assert_eq!(df.stages.len(), 2);
        assert_eq!(df.stages[0].name.as_deref(), Some("base"));
        assert_eq!(df.stages[1].name.as_deref(), Some("app"));
        assert_eq!(df.stages[1].from.image, "base");
    }

    #[test]
    fn test_strip_run_flags() {
        let input = "--mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile";
        let result = strip_run_flags(input);
        assert_eq!(result, "pnpm install --frozen-lockfile");
    }
}
