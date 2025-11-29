<h1 align="center">🔥 Blaze API 🔥</h1>
<h3 align="center">Stop waiting for API responses. Start blazing through them.</h3>

<p align="center">
  <strong>
    <em>The ultimate batch API client for your LLM workloads. It load-balances across endpoints, retries intelligently, and processes 10,000+ requests per second on a laptop.</em>
  </strong>
</p>

<p align="center">
  <!-- Package Info -->
  <a href="https://crates.io/crates/blaze-api"><img alt="crates.io" src="https://img.shields.io/crates/v/blaze-api.svg?style=flat-square&color=4D87E6"></a>
  <a href="#"><img alt="rust" src="https://img.shields.io/badge/rust-1.75+-4D87E6.svg?style=flat-square"></a>
  &nbsp;&nbsp;•&nbsp;&nbsp;
  <!-- Features -->
  <a href="https://opensource.org/licenses/MIT"><img alt="license" src="https://img.shields.io/badge/License-MIT-F9A825.svg?style=flat-square"></a>
  <a href="#"><img alt="platform" src="https://img.shields.io/badge/platform-macOS_|_Linux_|_Windows-2ED573.svg?style=flat-square"></a>
</p>

<p align="center">
  <img alt="zero config" src="https://img.shields.io/badge/⚙️_zero_config-works_out_of_the_box-2ED573.svg?style=for-the-badge">
  <img alt="10k rps" src="https://img.shields.io/badge/🚀_10K+_req/sec-on_modest_hardware-2ED573.svg?style=for-the-badge">
</p>

<div align="center">

### 🧭 Quick Navigation

[**⚡ Get Started**](#-get-started-in-60-seconds) •
[**✨ Key Features**](#-feature-breakdown-the-secret-sauce) •
[**🎮 Usage & Examples**](#-usage-fire-and-forget) •
[**⚙️ Configuration**](#%EF%B8%8F-configuration) •
[**🆚 Why Blaze**](#-why-blaze-slaps-other-methods)

</div>

---

**Blaze API** is the batch processor your LLM workloads deserve. Stop writing brittle Python scripts that crash at 100 req/sec. This tool acts like a fleet of pro API consumers, intelligently distributing requests across endpoints, handling failures gracefully, and maxing out your API capacity without breaking a sweat.

<div align="center">
<table>
<tr>
<td align="center">
<h3>⚡</h3>
<b>Blazing Fast</b><br/>
<sub>10K+ req/sec on 8 cores</sub>
</td>
<td align="center">
<h3>🎯</h3>
<b>Smart Load Balancing</b><br/>
<sub>Weighted distribution across endpoints</sub>
</td>
<td align="center">
<h3>🔄</h3>
<b>Auto Retry</b><br/>
<sub>Exponential backoff with jitter</sub>
</td>
<td align="center">
<h3>📊</h3>
<b>Real-time Stats</b><br/>
<sub>Progress, RPS, latency tracking</sub>
</td>
</tr>
</table>
</div>

How it slaps:
- **You:** `blaze -i requests.jsonl -o results.jsonl`
- **Blaze:** Load balances, retries failures, tracks progress, writes results.
- **You:** Go grab a coffee while 100K requests complete. ☕
- **Result:** Perfectly formatted JSONL with every response. Zero babysitting.

---

## 💥 Why Blaze Slaps Other Methods

Manually scripting API requests is a vibe-killer. Blaze makes other methods look ancient.

<table align="center">
<tr>
<td align="center"><b>❌ The Old Way (Pain)</b></td>
<td align="center"><b>✅ The Blaze Way (Glory)</b></td>
</tr>
<tr>
<td>
<ol>
  <li>Write Python script with asyncio.</li>
  <li>Hit GIL limits at 500 req/sec.</li>
  <li>Script crashes, lose progress.</li>
  <li>Add retry logic, still flaky.</li>
  <li>Manually restart, pray it works.</li>
</ol>
</td>
<td>
<ol>
  <li><code>blaze -i data.jsonl -o out.jsonl</code></li>
  <li>Watch the progress bar fly.</li>
  <li>Failures auto-retry with backoff.</li>
  <li>Results stream to disk instantly.</li>
  <li>Go grab a coffee. ☕</li>
</ol>
</td>
</tr>
</table>

We're not just sending requests. We're building a **high-throughput, fault-tolerant pipeline** with weighted load balancing, connection pooling, and intelligent retry logic that actually respects your API provider's limits.

---

## 🚀 Get Started in 60 Seconds

<div align="center">

| Platform | Method | Command |
|:--------:|:------:|:--------|
| 🦀 **All** | Cargo | `cargo install blaze-api` |
| 🍎 **macOS** | Homebrew | `brew install yigitkonur/tap/blaze` |
| 🐧 **Linux** | Binary | See [releases](https://github.com/yigitkonur/blaze-api/releases) |
| 🪟 **Windows** | Binary | See [releases](https://github.com/yigitkonur/blaze-api/releases) |

</div>

### 🦀 From Source (Recommended for Development)

```bash
# Clone and build
git clone https://github.com/yigitkonur/blaze-api.git
cd blaze-api
cargo build --release

# Binary is at ./target/release/blaze
```

### 📦 From crates.io

```bash
cargo install blaze-api
```

> **✨ Zero Config:** After installation, `blaze` is ready to go. Just point it at your JSONL file!

---

## 🎮 Usage: Fire and Forget

The workflow is dead simple.

### Basic Usage

```bash
# Process requests and save results
blaze --input requests.jsonl --output results.jsonl

# Short flags work too
blaze -i requests.jsonl -o results.jsonl

# High-throughput mode (10K req/sec)
blaze -i data.jsonl -o out.jsonl --rate 10000 --workers 200
```

### With Custom Endpoints

```bash
# Use a config file for multiple endpoints
blaze -i requests.jsonl -o results.jsonl --config endpoints.json

# Or set via environment
export BLAZE_ENDPOINT_URL="https://api.openai.com/v1/completions"
export BLAZE_API_KEY="sk-..."
export BLAZE_MODEL="gpt-4"
blaze -i requests.jsonl -o results.jsonl
```

### Input Format

Your `requests.jsonl` file should have one JSON object per line:

```jsonl
{"input": "What is the capital of France?"}
{"input": "Explain quantum computing in simple terms."}
{"input": "Write a haiku about Rust programming."}
```

Or with custom request bodies:

```jsonl
{"body": {"messages": [{"role": "user", "content": "Hello!"}], "model": "gpt-4"}}
{"body": {"messages": [{"role": "system", "content": "You are helpful."}, {"role": "user", "content": "Hi!"}]}}
```

### Output Format

Results are written as JSONL:

```jsonl
{"input": "What is the capital of France?", "response": {"choices": [...]}, "metadata": {"endpoint": "...", "latency_ms": 234, "attempts": 1}}
{"input": "Explain quantum computing...", "response": {"choices": [...]}, "metadata": {"endpoint": "...", "latency_ms": 189, "attempts": 1}}
```

Errors go to `errors.jsonl`:

```jsonl
{"input": "...", "error": "HTTP 429: Rate limit exceeded", "status_code": 429, "attempts": 3}
```

---

## ✨ Feature Breakdown: The Secret Sauce

<div align="center">

| Feature | What It Does | Why You Care |
| :---: | :--- | :--- |
| **⚡ Async Everything**<br/>`Tokio runtime` | Non-blocking I/O with work-stealing scheduler | Saturates your CPU cores efficiently |
| **🎯 Weighted Load Balancing**<br/>`Smart distribution` | Route traffic based on endpoint capacity | Max out multiple API keys simultaneously |
| **🔄 Exponential Backoff**<br/>`With jitter` | Intelligent retry with randomized delays | Respects rate limits, avoids thundering herd |
| **📊 Real-time Progress**<br/>`Live stats` | RPS, success rate, latency, ETA | Know exactly what's happening |
| **🔌 Connection Pooling**<br/>`HTTP/2 keep-alive` | Reuses connections across requests | Eliminates TCP handshake overhead |
| **💾 Streaming Output**<br/>`Immediate writes` | Results written as they complete | Never lose progress on crashes |
| **🏥 Health Tracking**<br/>`Per-endpoint` | Automatic failover on errors | Unhealthy endpoints get cooled off |
| **🔧 Flexible Config**<br/>`CLI + ENV + JSON` | Configure via args, env vars, or files | Fits any workflow |

</div>

---

## ⚙️ Configuration

### CLI Flags

```
USAGE:
    blaze [OPTIONS] --input <FILE>

OPTIONS:
    -i, --input <FILE>        Path to JSONL input file [env: BLAZE_INPUT]
    -o, --output <FILE>       Path for successful responses [env: BLAZE_OUTPUT]
    -e, --errors <FILE>       Path for error responses [default: errors.jsonl]
    -r, --rate <N>            Max requests per second [default: 1000]
    -w, --workers <N>         Concurrent workers [default: 50]
    -t, --timeout <SECS>      Request timeout [default: 30]
    -a, --max-attempts <N>    Max retry attempts [default: 3]
    -c, --config <FILE>       Endpoint config file (JSON)
    -v, --verbose             Enable debug logging
        --json-logs           Output logs as JSON
        --no-progress         Disable progress bar
        --dry-run             Validate config without processing
    -h, --help                Print help
    -V, --version             Print version
```

### Environment Variables

All options can be set via environment variables with `BLAZE_` prefix:

```bash
export BLAZE_INPUT="requests.jsonl"
export BLAZE_OUTPUT="results.jsonl"
export BLAZE_RATE="5000"
export BLAZE_WORKERS="100"
export BLAZE_ENDPOINT_URL="https://api.example.com/v1/completions"
export BLAZE_API_KEY="your-api-key"
export BLAZE_MODEL="gpt-4"
```

### Configuration File

For multiple endpoints, create `endpoints.json`:

```json
{
  "endpoints": [
    {
      "url": "https://api.openai.com/v1/completions",
      "weight": 2,
      "api_key": "sk-key-1",
      "model": "gpt-4",
      "max_concurrent": 100
    },
    {
      "url": "https://api.openai.com/v1/completions",
      "weight": 1,
      "api_key": "sk-key-2",
      "model": "gpt-4",
      "max_concurrent": 50
    }
  ],
  "request": {
    "timeout": "30s",
    "rate_limit": 5000,
    "workers": 100
  },
  "retry": {
    "max_attempts": 3,
    "initial_backoff": "100ms",
    "max_backoff": "10s",
    "multiplier": 2.0
  }
}
```

Then run:

```bash
blaze -i requests.jsonl -o results.jsonl --config endpoints.json
```

---

## 📈 Performance Tips

### Maximize Throughput

```bash
# For maximum speed (adjust based on your API limits)
blaze -i data.jsonl -o out.jsonl \
  --rate 10000 \
  --workers 200 \
  --timeout 60
```

### Balance Load Across Keys

```json
{
  "endpoints": [
    {"url": "...", "api_key": "key-1", "weight": 3, "max_concurrent": 150},
    {"url": "...", "api_key": "key-2", "weight": 2, "max_concurrent": 100},
    {"url": "...", "api_key": "key-3", "weight": 1, "max_concurrent": 50}
  ]
}
```

### Handle Rate Limits Gracefully

```json
{
  "retry": {
    "max_attempts": 5,
    "initial_backoff": "500ms",
    "max_backoff": "30s",
    "multiplier": 2.0
  }
}
```

---

## 🛠️ For Developers & Tinkerers

### Building from Source

```bash
git clone https://github.com/yigitkonur/blaze-api.git
cd blaze-api

# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Using as a Library

```rust
use blaze_api::{Config, EndpointConfig, Processor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config {
        endpoints: vec![EndpointConfig {
            url: "https://api.example.com/v1/completions".to_string(),
            weight: 1,
            api_key: Some("your-key".to_string()),
            model: Some("gpt-4".to_string()),
            max_concurrent: 100,
        }],
        ..Default::default()
    };

    let processor = Processor::new(config)?;
    let result = processor.process_file(
        "requests.jsonl".into(),
        Some("results.jsonl".into()),
        "errors.jsonl".into(),
        true,
    ).await?;

    result.print_summary();
    Ok(())
}
```

### Project Structure

```
src/
├── lib.rs        # Library entry point
├── main.rs       # CLI binary
├── config.rs     # Configuration management
├── client.rs     # HTTP client with retry logic
├── endpoint.rs   # Load balancer implementation
├── processor.rs  # Main processing orchestration
├── request.rs    # Request/response types
├── tracker.rs    # Statistics tracking
└── error.rs      # Error types
```

---

## 🔥 Common Issues & Quick Fixes

<details>
<summary><b>Expand for troubleshooting tips</b></summary>

| Problem | Solution |
| :--- | :--- |
| **"Too many open files"** | Increase ulimit: `ulimit -n 65535` |
| **Connection timeouts** | Increase `--timeout` or reduce `--workers` |
| **Rate limit errors (429)** | Lower `--rate` or add more API keys |
| **Memory usage high** | Reduce `--workers` for large requests |
| **Progress bar not showing** | Don't pipe output, or use `--no-progress --json-logs` |

**Build Issues:**

| Problem | Solution |
| :--- | :--- |
| **OpenSSL errors** | Install OpenSSL dev: `apt install libssl-dev` or use `--features rustls` |
| **Rust version error** | Update Rust: `rustup update stable` (requires 1.75+) |

</details>

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

```bash
# Fork the repo, then:
git clone https://github.com/YOUR_USERNAME/blaze-api.git
cd blaze-api
cargo test
# Make your changes
cargo fmt
cargo clippy
cargo test
# Submit PR
```

---

## 📄 License

MIT © [Yiğit Konur](https://github.com/yigitkonur)

---

<div align="center">

**Built with 🔥 because waiting for API responses is a soul-crushing waste of time.**

[⬆ Back to Top](#-blaze-api-)

</div>
