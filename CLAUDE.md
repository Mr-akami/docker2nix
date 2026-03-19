# CLAUDE.md

## 概要

Dockerfile/docker-compose.ymlからNix Flake(devShell)を自動生成するRust CLIツール。

## 開発環境

Nix Flakeで管理。direnvまたは`nix develop`で入る。

```sh
# direnv (推奨、.envrcあり)
direnv allow

# または手動
nix develop
```

## 含まれるツール

- Rust stable (rustc, cargo)
- rust-analyzer, clippy, rustfmt
- cargo-watch, cargo-edit

## よく使うコマンド

```sh
cargo build          # ビルド
cargo run            # 実行
cargo test           # テスト
cargo clippy         # lint
cargo fmt            # フォーマット
cargo watch -x run   # ファイル変更時に自動実行
```

## 環境変数

- `RUST_BACKTRACE=1` がデフォルトで有効
