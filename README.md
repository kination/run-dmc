# run_dmc

__Until now, this project is for research, not for production__

This project __run_dmc__(=> 'Runtime' for 'Data Management Container') is another "Open Container Initiative (OCI) Runtime" written in Rust, specifically optimized for **Data Engineering** workloads.

## Goal of research
- **Performance First**: Engineered for ultra-low latency (<50ms startup) and minimal memory footprint.
- **I/O Optimized**: Leverages **`io_uring`** with polling mode for high-throughput ETL and database operations.
- **Hardware Accelerated**: Native support for **GPU passthrough** and HugePages configuration for AI/ML pipelines.
- **Modern Linux**: Built on **cgroups v2** and modern kernel namespaces.

## Understanding Containers

**Image** = Read-only template (blueprint)
**Container** = Running instance of an image (isolated process)
**Runtime** = Software that creates/manages containers

### Where rundmc Fits

```
Docker CLI → containerd (image management)
                ↓
            rundmc (OCI runtime) ⭐
                ↓
            Linux Kernel (namespaces, cgroups)
```

**rundmc** is a low-level OCI runtime (like runc) optimized for data engineering:
- io_uring for high-throughput I/O
- GPU/TPU passthrough for AI/ML
- Block I/O isolation for multi-tenant workloads

## Status
**🚧 Under Development (Phase 1)**

Currently implementing the foundational OCI specification compliance (CLI parsing, Basic Isolation).

## Quick Start (macOS with Docker Desktop)

This guide shows how to test `rundmc` on macOS using Docker Desktop.

### Prerequisites

- Docker Desktop for Mac (running)
- Rust toolchain with `cargo`
- `cross` for cross-compilation

### Step 1: Build the Runtime

```bash
# Install cross-compilation tool (if not already installed)
cargo install cross

# Build static Linux binary
cross build --release --target x86_64-unknown-linux-musl

# Copy binary for easier access
cp target/x86_64-unknown-linux-musl/release/rundmc ./rundmc-linux
```

### Step 2: Install into Docker Desktop VM

**Automated Installation (Recommended):**

Use the installation script that handles everything automatically:

```bash
# Run the installation script
./install-rundmc.sh
```

The script will:
- Create the helper container (`vm-access`)
- Copy and install the binary into the VM
- Verify the installation
- Check runtime registration status

**Expected output:**
```
=== Installing rundmc into Docker Desktop VM ===

✅ Found rundmc-linux binary
✅ Docker is running
📦 Step 1: Setting up helper container...
✅ Helper container created
📤 Step 2: Copying rundmc binary to VM...
✅ Binary installed to /usr/local/bin/rundmc
🔍 Step 3: Verifying installation...
✅ rundmc is working!
```

### Step 3: Register Runtime with Docker

1. Open **Docker Desktop** → **Settings** (⚙️ icon)
2. Navigate to **Docker Engine** tab
3. Add `rundmc` to the runtimes configuration:

```json
{
  "runtimes": {
    "rundmc": {
      "path": "/usr/local/bin/rundmc"
    }
  }
}
```

4. Click **Apply & Restart**

### Step 4: Verify Runtime Registration

```bash
# Check if rundmc appears in available runtimes
docker info --format '{{json .Runtimes}}' | python3 -m json.tool | grep -A2 rundmc
```

**Expected output:**
```json
"rundmc": {
    "path": "/usr/local/bin/rundmc",
    ...
}
```

### Step 5: Test the Runtime

**Quick Test:**
```bash
# Run a simple container (note: networking is not yet implemented, so use --network=none)
docker run --rm --runtime=rundmc --network=none alpine echo "Hello from rundmc"
```

**Detailed Test with Logging:**

To observe the full container lifecycle with detailed logs, use this test script:

```bash
# Run tests
./test-rundmc.sh
```

**Expected Output:**

```
=== Testing rundmc with detailed logging ===

📝 Test 1: Simple echo command with DEBUG logging
[INFO  rundmc] rundmc invoked: command="create", root=Some("/var/run/docker/runtime-rundmc/moby")
[INFO  rundmc::container] Creating container: id="abc123", command=["echo", "Hello from rundmc"]
[INFO  rundmc::container] Container created: id="abc123", pid=12345, status=created
[INFO  rundmc::container] Container started: id="abc123", pid=Some(12345), status=running
Hello from rundmc
[INFO  rundmc::container] Container deleted: id="abc123"

📝 Test 2: Multi-step script (sleep + multiple commands)
[INFO  rundmc] rundmc invoked: command="create"...
[INFO  rundmc::container] Creating container...
Step 1: Container started
Step 2: After 1 second
Step 3: After 2 seconds
Step 4: Exiting...
[INFO  rundmc::container] Container deleted...

📝 Test 3: State inspection test
Starting container in background...
Container ID: def456
Checking container state...
    "Runtime": "rundmc",
    "Pid": 12346,
Cleaning up...

📝 Test 4: Full lifecycle with timestamps
Start time: 10:30:45.123
[10:30:45.125] [INFO  rundmc] rundmc invoked: command="create"
[10:30:45.127] [INFO  rundmc::container] Creating container: id="ghi789"
[10:30:45.130] [INFO  rundmc::container] Container created: id="ghi789", pid=12347
[10:30:45.132] [INFO  rundmc::container] Container started: id="ghi789"
[10:30:45.135] Running at: Sat Jan 18 10:30:45 UTC 2026
[10:30:47.140] Done
[10:30:47.145] [INFO  rundmc::container] Container deleted: id="ghi789"
End time: 10:30:47.150

=== All tests completed ===
```

### Cleanup

```bash
# Remove the helper container
docker rm -f vm-access
```
