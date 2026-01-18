#!/bin/bash
# update-rundmc.sh - Quick update script for rundmc binary after code changes

set -e

echo "=== Updating rundmc runtime ==="
echo ""

# Step 1: Build
echo "🔨 Step 1: Building rundmc..."
cross build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/rundmc ./rundmc-linux
echo "✅ Build complete"
echo ""

# Step 2: Check if vm-access container exists
if ! docker ps --format '{{.Names}}' | grep -q "^vm-access$"; then
    echo "⚠️  vm-access container not running"
    echo "Running full installation instead..."
    exec ./install-rundmc.sh
fi

# Step 3: Update binary
echo "📤 Step 2: Updating binary in VM..."
docker cp rundmc-linux vm-access:/tmp/rundmc.bin
docker exec vm-access sh -c 'base64 /tmp/rundmc.bin' | \
  docker exec -i vm-access nsenter -t 1 -m -u -n -i sh -c \
  'base64 -d > /usr/local/bin/rundmc && chmod +x /usr/local/bin/rundmc'
echo "✅ Binary updated"
echo ""

# Step 4: Quick verification
echo "🔍 Step 3: Verifying update..."
if docker exec vm-access nsenter -t 1 -m -u -n -i rundmc --help > /dev/null 2>&1; then
    echo "✅ rundmc is working!"
else
    echo "❌ Error: Updated binary is not working"
    exit 1
fi
echo ""

# Step 5: Quick test
echo "🧪 Step 4: Running quick test..."
if docker run --rm --runtime=rundmc --network=none alpine echo "Updated runtime works" 2>&1 | grep -q "Updated runtime works"; then
    echo "✅ Test passed!"
else
    echo "⚠️  Test had issues (check logs above)"
fi
echo ""

echo "=== Update Complete ==="
echo ""
echo "Run full tests with: ./test-rundmc.sh"
echo ""
