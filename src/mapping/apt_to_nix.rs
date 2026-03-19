/// Map an apt package name to nixpkgs attribute(s).
/// Returns None for unmapped packages.
pub fn apt_to_nix(apt_pkg: &str) -> Option<Vec<&'static str>> {
    match apt_pkg {
        // core tools
        "curl" => Some(vec!["curl"]),
        "wget" => Some(vec!["wget"]),
        "git" => Some(vec!["git"]),
        "jq" => Some(vec!["jq"]),
        "rsync" => Some(vec!["rsync"]),
        "unzip" => Some(vec!["unzip"]),
        "zip" => Some(vec!["zip"]),
        "file" => Some(vec!["file"]),
        "tree" => Some(vec!["tree"]),
        "less" => Some(vec!["less"]),
        "htop" => Some(vec!["htop"]),
        "sudo" => Some(vec!["sudo"]),

        // editors
        "vim" | "vim-nox" => Some(vec!["vim"]),
        "nano" => Some(vec!["nano"]),

        // build tools
        "build-essential" => Some(vec!["gcc", "gnumake"]),
        "make" => Some(vec!["gnumake"]),
        "gcc" => Some(vec!["gcc"]),
        "g++" => Some(vec!["gcc"]),
        "cmake" => Some(vec!["cmake"]),
        "autoconf" => Some(vec!["autoconf"]),
        "automake" => Some(vec!["automake"]),
        "libtool" => Some(vec!["libtool"]),
        "pkg-config" => Some(vec!["pkg-config"]),

        // python
        "python3" | "python3-pip" | "python3-dev" => Some(vec!["python3"]),
        "python3-venv" => Some(vec!["python3"]),

        // libraries
        "libssl-dev" => Some(vec!["openssl"]),
        "libffi-dev" => Some(vec!["libffi"]),
        "zlib1g-dev" => Some(vec!["zlib"]),
        "libxml2-dev" => Some(vec!["libxml2"]),
        "libxslt1-dev" => Some(vec!["libxslt"]),
        "libyaml-dev" => Some(vec!["libyaml"]),
        "libreadline-dev" => Some(vec!["readline"]),
        "libncurses5-dev" | "libncursesw5-dev" => Some(vec!["ncurses"]),
        "libbz2-dev" => Some(vec!["bzip2"]),
        "liblzma-dev" => Some(vec!["xz"]),
        "libcurl4-openssl-dev" => Some(vec!["curl"]),
        "libpq-dev" => Some(vec!["postgresql"]),
        "libsqlite3-dev" => Some(vec!["sqlite"]),

        // geo/science
        "libboost-all-dev" => Some(vec!["boost"]),
        "libgdal-dev" | "gdal-bin" => Some(vec!["gdal"]),
        "proj-bin" => Some(vec!["proj"]),

        // databases / services
        "redis-server" => Some(vec!["redis"]),
        "postgresql" | "postgresql-client" => Some(vec!["postgresql"]),

        // network / security
        "openssh-client" => Some(vec!["openssh"]),
        "ca-certificates" => Some(vec!["cacert"]),
        "gnupg" | "gnupg2" => Some(vec!["gnupg"]),

        // no nix equivalent needed
        "software-properties-common" | "apt-transport-https" | "lsb-release" | "locales" => None,

        _ => None,
    }
}

/// Map a Docker base image name to nixpkgs attribute(s).
pub fn base_image_to_nix(image: &str) -> Option<Vec<&'static str>> {
    match image {
        "node" | "nodejs" => Some(vec!["nodejs"]),
        "python" => Some(vec!["python3"]),
        "postgres" | "postgresql" => Some(vec!["postgresql"]),
        "redis" => Some(vec!["redis"]),
        "golang" | "go" => Some(vec!["go"]),
        "ruby" => Some(vec!["ruby"]),
        "rust" => Some(vec!["rustc", "cargo"]),
        "mysql" | "mariadb" => Some(vec!["mariadb"]),
        "mongo" | "mongodb" => Some(vec!["mongodb"]),
        "nginx" => Some(vec!["nginx"]),
        // base OS images don't map to packages
        "alpine" | "ubuntu" | "debian" | "centos" | "fedora" | "archlinux" => None,
        _ => None,
    }
}

/// Known infrastructure images that should become process-compose services
/// rather than devShell packages.
pub fn is_infrastructure_image(image: &str) -> bool {
    matches!(
        image,
        "postgres"
            | "postgresql"
            | "redis"
            | "mysql"
            | "mariadb"
            | "mongo"
            | "mongodb"
            | "memcached"
            | "rabbitmq"
            | "kafka"
            | "zookeeper"
            | "elasticsearch"
            | "minio"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apt_to_nix_known() {
        assert_eq!(apt_to_nix("curl"), Some(vec!["curl"]));
        assert_eq!(apt_to_nix("build-essential"), Some(vec!["gcc", "gnumake"]));
    }

    #[test]
    fn test_apt_to_nix_unknown() {
        assert_eq!(apt_to_nix("some-unknown-pkg"), None);
    }

    #[test]
    fn test_base_image() {
        assert_eq!(base_image_to_nix("postgres"), Some(vec!["postgresql"]));
        assert_eq!(base_image_to_nix("ubuntu"), None);
    }

    #[test]
    fn test_infra_image() {
        assert!(is_infrastructure_image("postgres"));
        assert!(!is_infrastructure_image("ubuntu"));
        assert!(!is_infrastructure_image("node"));
    }
}
