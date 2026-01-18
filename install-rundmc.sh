#!/bin/bash
# install-rundmc.sh - Install rundmc runtime into Docker Desktop VM

set -e

echo "=== Installing rundmc into Docker Desktop VM ==="
echo ""

# Check if rundmc-linux binary exists
if [ ! -f "rundmc-linux" ]; then
    echo "❌ Error: rundmc-linux binary not found"
    echo "Please build it first:"
    echo "  cross build --release --target x86_64-unknown-linux-musl"
    echo "  cp target/x86_64-unknown-linux-musl/release/rundmc ./rundmc-linux"
    exit 1
fi

echo "✅ Found rundmc-linux binary"
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Error: Docker is not running"
    echo "Please start Docker Desktop first"
    exit 1
fi

echo "✅ Docker is running"
echo ""

# Step 1: Create/recreate helper container
echo "📦 Step 1: Setting up helper container..."
if docker ps -a --format '{{.Names}}' | grep -q "^vm-access$"; then
    echo "Removing existing vm-access container..."
    docker rm -f vm-access > /dev/null 2>&1
fi

echo "Creating new vm-access container..."
docker run -d --name vm-access --privileged --pid=host debian sleep infinity > /dev/null
echo "✅ Helper container created"
echo ""

# Step 2: Copy binary to VM
echo "📤 Step 2: Copying rundmc binary to VM..."
docker cp rundmc-linux vm-access:/tmp/rundmc.bin
echo "Binary copied to container"
echo ""

echo "Installing into VM filesystem..."
docker exec vm-access sh -c 'base64 /tmp/rundmc.bin' | \
  docker exec -i vm-access nsenter -t 1 -m -u -n -i sh -c \
  'base64 -d > /usr/local/bin/rundmc && chmod +x /usr/local/bin/rundmc'
echo "✅ Binary installed to /usr/local/bin/rundmc"
echo ""

# Step 3: Verify installation
echo "🔍 Step 3: Verifying installation..."
if docker exec vm-access nsenter -t 1 -m -u -n -i rundmc --help > /dev/null 2>&1; then
    echo "✅ rundmc is working!"
    docker exec vm-access nsenter -t 1 -m -u -n -i rundmc --version 2>&1 | head -1 || echo "rundmc (version info not available)"
else
    echo "❌ Error: rundmc installation failed"
    exit 1
fi
echo ""

# Step 4: Check Docker runtime registration
echo "🔧 Step 4: Checking Docker runtime registration..."
if docker info --format '{{json .Runtimes}}' 2>/dev/null | grep -q '"rundmc"'; then
    echo "✅ rundmc is registered in Docker"
else
    echo "⚠️  Warning: rundmc is NOT registered in Docker"
    echo ""
    echo "Please register it manually:"
    echo "1. Open Docker Desktop → Settings (⚙️)"
    echo "2. Navigate to 'Docker Engine' tab"
    echo "3. Add the following to the JSON config:"
    echo ""
    echo '  "runtimes": {'
    echo '    "rundmc": {'
    echo '      "path": "/usr/local/bin/rundmc"'
    echo '    }'
    echo '  }'
    echo ""
    echo "4. Click 'Apply & Restart'"
    echo ""
fi

# Step 5: Quick test
echo "🧪 Step 5: Running quick test..."
if docker info --format '{{json .Runtimes}}' 2>/dev/null | grep -q '"rundmc"'; then
    echo "Testing rundmc with a simple container..."
    if docker run --rm --runtime=rundmc --network=none alpine echo "Hello from rundmc" 2>&1 | grep -q "Hello from rundmc"; then
        echo "✅ Test passed!"
    else
        echo "⚠️  Test failed (but this might be expected if runtime is not fully functional yet)"
    fi
else
    echo "⏭️  Skipping test (runtime not registered)"
fi
echo ""

echo "=== Installation Complete ==="
echo ""
echo "📋 Summary:"
echo "  - Helper container: vm-access (running)"
echo "  - Binary location: /usr/local/bin/rundmc"
echo "  - Runtime status: $(docker info --format '{{json .Runtimes}}' 2>/dev/null | grep -q '"rundmc"' && echo 'Registered ✅' || echo 'Not registered ⚠️')"
echo ""
echo "Next steps:"
echo "  1. If not registered, follow the registration instructions above"
echo "  2. Run tests: ./test-rundmc.sh"
echo "  3. After code changes, run: ./update-rundmc.sh"
echo ""
