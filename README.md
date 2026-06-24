# rugit

A from-scratch reimplementation of Git's core version control internals in Rust, built for learning how Git actually works.

`rugit` implements Git's low-level data structures ("plumbing") rather than its user-facing commands ("porcelain"): object storage and hashing, binary index parsing, and the smart HTTP wire protocol.

## Features

* **Object Storage:** Hashes and stores `blob`, `tree`, and `commit` objects using SHA-1, with Zlib compression matching Git's on-disk format.
* **Binary Index Parser:** Reads and writes the `.git/index` file format, including the `DIRC` signature, version header, per-entry stat data, and fan-out padding.
* **Smart HTTP Protocol:** Implements `git-receive-pack` capability negotiation and sideband multiplexing to push to remote repositories over HTTP.
* **Packfile Generation:** Traverses the commit graph to generate delta-compressed packfiles for transmission.
* **Branch Handling:** Implements branch refs and `HEAD` resolution, including branch creation and switching.

---

## Installation

`rugit` is built from source using Cargo.

**Prerequisites:** [Rust and Cargo](https://www.rust-lang.org/tools/install) (stable toolchain)

```bash
git clone https://github.com/PranavDesai-Git/rugit
cd rugit
cargo build --release
```

The compiled binary will be available at `target/release/rugit`. To use it from anywhere, add it to your `PATH` or install it directly:

```bash
cargo install --path .
```

This places `rugit` in `~/.cargo/bin`, which should already be on your `PATH` if Rust was installed via `rustup`.

## CLI Usage

### Repository Lifecycle

Initialize a new `rugit` repository structure:

```bash
rugit init
```

Configure your user credentials (writes directly to `.git/config`):

```bash
rugit config user.name "Your Name"
rugit config user.email "you@example.com"
```

### Staging & Committing

Stage a file into the binary index (`.git/index`):

```bash
rugit add src/main.rs
```

Commit the currently staged index state:

```bash
rugit commit -m "Implement custom index parsing and state tracking"
```

### Branching

Create a new branch pointer from the active `HEAD`:

```bash
rugit branch feature/network-layer
```

Switch `HEAD` to an existing branch:

```bash
rugit switch feature/network-layer
```

### Remote Networking

Add a remote repository:

```bash
rugit remote add origin https://github.com/username/repo.git
```

Push the current branch to a remote. If no remote is given, it defaults to `origin`:

```bash
rugit push
rugit push origin
```

### Help

Print usage information for all subcommands:

```bash
rugit help
```

---

## Built With

* **Rust** — systems language; no garbage collector, manual memory and lifetime management
* `sha1` — SHA-1 hashing for object IDs
* `flate2` — Zlib compression for object storage
* `reqwest` — HTTP client for the Smart HTTP protocol
* `hex` — hex encoding/decoding for object ID strings and storage paths
