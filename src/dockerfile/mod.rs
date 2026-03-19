pub mod apt;
pub mod parser;

#[derive(Debug, Clone)]
pub struct Dockerfile {
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub name: Option<String>,
    pub from: FromDirective,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct FromDirective {
    pub image: String,
    pub tag: Option<String>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Run(String),
    Env(String, String),
    Arg(String, Option<String>),
    Workdir(String),
    Expose(u16),
    Copy(String),
    Cmd(String),
    Entrypoint(String),
    Other(String),
}
