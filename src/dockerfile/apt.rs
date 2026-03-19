/// Extract apt/apt-get package names from a RUN instruction body.
pub fn extract_apt_packages(run_cmd: &str) -> Vec<String> {
    let mut packages = Vec::new();

    for segment in run_cmd.split("&&") {
        let tokens: Vec<&str> = segment.split_whitespace().collect();

        let is_apt = tokens
            .first()
            .map_or(false, |t| *t == "apt" || *t == "apt-get");
        if !is_apt {
            continue;
        }

        let install_pos = tokens.iter().position(|t| *t == "install");
        let Some(pos) = install_pos else {
            continue;
        };

        for token in &tokens[pos + 1..] {
            // skip flags
            if token.starts_with('-') {
                continue;
            }
            // stop at shell operators
            if *token == "&&" || *token == "||" || *token == "|" || *token == ";" {
                break;
            }
            packages.push(token.to_string());
        }
    }

    packages
}

/// Extract pip package names from a RUN instruction body.
pub fn extract_pip_packages(run_cmd: &str) -> Vec<String> {
    let mut packages = Vec::new();

    for segment in run_cmd.split("&&") {
        let tokens: Vec<&str> = segment.split_whitespace().collect();

        let is_pip = tokens
            .first()
            .map_or(false, |t| *t == "pip" || *t == "pip3" || *t == "pip2");
        if !is_pip {
            continue;
        }

        let install_pos = tokens.iter().position(|t| *t == "install");
        let Some(pos) = install_pos else {
            continue;
        };

        for token in &tokens[pos + 1..] {
            if token.starts_with('-') {
                continue;
            }
            if *token == "&&" || *token == "||" || *token == "|" || *token == ";" {
                break;
            }
            packages.push(token.to_string());
        }
    }

    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_apt_packages() {
        let cmd = "apt update && apt install -y  curl  build-essential  && apt clean  && rm -rf /var/lib/apt/lists/*";
        let pkgs = extract_apt_packages(cmd);
        assert_eq!(pkgs, vec!["curl", "build-essential"]);
    }

    #[test]
    fn test_extract_apt_get() {
        let cmd = "apt-get update && apt install -y redis-server";
        let pkgs = extract_apt_packages(cmd);
        assert_eq!(pkgs, vec!["redis-server"]);
    }

    #[test]
    fn test_extract_multiple_apt_calls() {
        let cmd = "apt update && apt install -y python3-pip libgdal-dev git && apt clean";
        let pkgs = extract_apt_packages(cmd);
        assert_eq!(pkgs, vec!["python3-pip", "libgdal-dev", "git"]);
    }

    #[test]
    fn test_extract_pip_packages() {
        let cmd = "pip3 install ezdxf && pip3 install langchain-core langchain-openai";
        let pkgs = extract_pip_packages(cmd);
        assert_eq!(pkgs, vec!["ezdxf", "langchain-core", "langchain-openai"]);
    }

    #[test]
    fn test_no_apt_install() {
        let cmd = "apt update && apt clean";
        let pkgs = extract_apt_packages(cmd);
        assert!(pkgs.is_empty());
    }
}
