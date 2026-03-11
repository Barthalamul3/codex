## Installing & building

### System requirements

| Requirement                 | Details                                                         |
| --------------------------- | --------------------------------------------------------------- |
| Operating systems           | macOS 12+, Ubuntu 20.04+/Debian 10+, or Windows 11 **via WSL2** |
| Git (optional, recommended) | 2.23+ for built-in PR helpers                                   |
| RAM                         | 4-GB minimum (8-GB recommended)                                 |

### DotSlash

The GitHub Release also contains a [DotSlash](https://dotslash-cli.com/) file for the Codex CLI named `codex`. Using a DotSlash file makes it possible to make a lightweight commit to source control to ensure all contributors use the same version of an executable, regardless of what platform they use for development.

### Build from source

```bash
# Clone the repository and navigate to the root of the Cargo workspace.
git clone https://github.com/openai/codex.git
cd codex/codex-rs

# Install the Rust toolchain, if necessary.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt
rustup component add clippy
# Install helper tools used by the workspace justfile:
cargo install just
# Optional: install nextest for the `just test` helper
cargo install --locked cargo-nextest

# Build Codex.
cargo build

# Launch the TUI with a sample prompt.
cargo run --bin codex -- "explain this codebase to me"

# After making changes, use the root justfile helpers (they default to codex-rs):
just fmt
just fix -p <crate-you-touched>

# Run the relevant tests (project-specific is fastest), for example:
cargo test -p codex-tui
# If you have cargo-nextest installed, `just test` runs the test suite via nextest:
just test
# Avoid `--all-features` for routine local runs because it increases build
# time and `target/` disk usage by compiling additional feature combinations.
# If you specifically want full feature coverage, use:
cargo test --all-features
```

### `ccodex` and RxT sidecar setup

This section is only for the custom `ccodex` workflow on the
`feature/per-turn-memory-architecture` branch. Stock `codex` does not require
the RxT sidecar.

`ccodex` can use an external RxT-backed memory brain when:

- `features.memory_os_brain_candidates` is enabled
- `CCODEX_RXT_BRAIN_URL` points at a running sidecar

The runtime code expects the sidecar analyze endpoint to look like:

```bash
http://127.0.0.1:8788/analyze
```

#### 1. Build and install the custom `ccodex` wrapper

From the repo root:

```bash
./scripts/install_ccodex_wrapper.sh
```

That script:

- builds the `ccodex` release binary
- installs the runtime to `~/.local/share/ccodex/bin/ccodex-custom`
- installs a `ccodex` wrapper at `~/.local/bin/ccodex`

#### 2. Create a Python environment for RxT

The sidecar imports `torch` and `rxlm`, and current `rxlm` releases require
Python 3.12 or newer. A local virtualenv keeps those dependencies isolated from
the Rust workspace.

From the repo root:

```bash
python3.12 -m venv .venv_rxt
source .venv_rxt/bin/activate
python -m pip install --upgrade pip
pip install torch
pip install rxlm transformers tokenizers huggingface_hub datasets tensorboard
```

If you are running on NVIDIA hardware and want the faster path recommended by
the RxLM project, install a compatible `flash-attn` wheel after `torch`. If you
do not have a compatible CUDA setup, skip it and use the default CPU or CUDA
runtime.

#### 3. Export the Hugging Face token and RxT runtime settings

The sidecar loads model artifacts from Hugging Face. Export one of the token
variables before starting it:

```bash
export HF_TOKEN=...
# or
export HUGGING_FACE_HUB_TOKEN=...
```

Useful optional runtime settings:

```bash
export CCODEX_RXT_HOST=127.0.0.1
export CCODEX_RXT_PORT=8788
export CCODEX_RXT_DEVICE=cuda
export CCODEX_RXT_DTYPE=bfloat16
```

If you are running CPU-only, set:

```bash
export CCODEX_RXT_DEVICE=cpu
```

#### 4. Start the RxT sidecar

From the repo root:

```bash
source .venv_rxt/bin/activate
python scripts/rxt_memory_brain_server.py
```

The sidecar stays in the foreground and serves requests over HTTP.

#### 5. Point `ccodex` at the sidecar

In the shell where you launch `ccodex`:

```bash
export CCODEX_RXT_BRAIN_URL=http://127.0.0.1:8788/analyze
```

Then start the custom runtime:

```bash
ccodex
```

#### 6. Verify the sidecar code path

You can smoke-test the Python side with:

```bash
source .venv_rxt/bin/activate
python scripts/test_rxt_memory_brain_server.py
```

If `ccodex` behavior does not change after install, verify you restarted the
runtime process after updating the binary or environment variables.

## Tracing / verbose logging

Codex is written in Rust, so it honors the `RUST_LOG` environment variable to configure its logging behavior.

The TUI defaults to `RUST_LOG=codex_core=info,codex_tui=info,codex_rmcp_client=info` and log messages are written to `~/.codex/log/codex-tui.log` by default. For a single run, you can override the log directory with `-c log_dir=...` (for example, `-c log_dir=./.codex-log`).

```bash
tail -F ~/.codex/log/codex-tui.log
```

By comparison, the non-interactive mode (`codex exec`) defaults to `RUST_LOG=error`, but messages are printed inline, so there is no need to monitor a separate file.

See the Rust documentation on [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for more information on the configuration options.
