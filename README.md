# mcp-uutils

`mcp-uutils` is an AI-optimized Model Context Protocol (MCP) server built in Rust, powered by the battle-tested standard UNIX utilities engine (`uutils/coreutils`).

It translates heavy, unpredictable, and raw shell commands into deterministic, JSON-structured, and line-windowed API tools specifically designed for LLMs and AI coding agents (such as Cursor, Cline, Claude Code, and custom autonomous frameworks).

---

## 🚀 Why mcp-uutils?

Traditional AI agents often use raw shell access (`bash`) to execute commands like `ls`, `cat`, or `grep`. This approach introduces significant liabilities for AI performance:

1. **Token Explosion:** Running `cat` on a large file or `ls -R` on a heavy directory can instantly saturate the LLM's context window and skyrocket API costs.
2. **Shell-Escaping Failures:** LLMs frequently mess up quoting and escaping characters when trying to pipe or redirect output (e.g., failing to structure `cat << 'EOF'` correctly).
3. **OS Discrepancies:** Variations between GNU (Linux) and BSD (macOS) command flags confuse the model, leading to hallucinated arguments and broken execution loops.

`mcp-uutils` solves this by introducing an abstract **Agent-Computer Interface (ACI)** layer. It wraps `uutils` core logic into high-efficiency, multi-platform JSON tools.

---

## 🛠️ Provided Core Tools

Once connected, `mcp-uutils` exposes a flat list of highly optimized tools to your LLM:

### 1. `inspect_dir` (Engine: `ls` + `du`)

Lists directory contents as clean, structured JSON. Automatically filters out development noise like `.git/`, `node_modules/`, and Rust's `target/` directories by default.

* **Returns:** Array of `{ "name": string, "type": "file" | "dir", "size_bytes": number }`

### 2. `peek_file` (Engine: `cat` + `head` + `tail`)

Enforces strict **line-windowing** for file inspection. LLMs are prohibited from reading whole files blindly; they must request specific line ranges to protect the context window.

* **Arguments:** `path` (required), `start_line` (optional), `end_line` (optional)

### 3. `structured_grep` (Engine: `grep`)

Performs text pattern matching across the workspace and packages matches into structured blocks, automatically bundling **N-lines of surrounding context** for each match so the LLM understands the surrounding code block in a single tool turn.

### 4. `verify_file` (Engine: `wc` + `md5sum` / `sha256sum`)

Allows agents to self-audit their work by fetching precise line counts, byte sizes, and cryptographic checksums of generated files without reloading the entire code into the context.

---

## 📦 Installation & Setup

### Prerequisites

* Rust 1.75+ and `cargo`

### Build from Source

```bash
git clone https://github.com/yourusername/mcp-uutils.git
cd mcp-uutils
cargo build --release

```

This generates a single, ultra-fast, static binary with zero runtime dependencies under `./target/release/mcp-uutils`.

---

## 🔌 Integration

`mcp-uutils` communicates via standard I/O (`stdio`), making it instantly compatible with any modern MCP host.

### Cline / Cursor Configuration

Add the following snippet to your global MCP settings file (e.g., `mcp_settings.json`):

```json
{
  "mcpServers": {
    "mcp-uutils": {
      "command": "/path/to/mcp-uutils",
      "args": [],
      "env": {
        "WORKSPACE_ROOT": "/path/to/your/projects"
      }
    }
  }
}

```

---

## 🛡️ Security & Guardrails

Unlike raw `bash` access, `mcp-uutils` comes with built-in physical isolation:

* **Path Canonicalization:** All incoming paths are strictly verified and canonicalized. If an agent attempts to use symlinks or `../` to escape the designated `WORKSPACE_ROOT` directory, the request is instantly blocked at the Rust layer before reaching the file system.
* **Read-Only / Read-Write Toggles:** Can be started with restricted flags to prevent accidental file deletion or destructive modification during unsupervised autonomous runs.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](https://www.google.com/search?q=LICENSE) file for details.
