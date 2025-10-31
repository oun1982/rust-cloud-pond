#!/bin/bash
# Deploy script สำหรับส่งไฟล์ไปยัง production server

set -e

# Configuration
SERVER="${1:-10.133.1.12}"
USER="${2:-root}"
REMOTE_PATH="/var/lib/asterisk/agi-bin/rust-agi"
LOCAL_BINARY="target/release/rust_agi_example"
LOCAL_CONFIG="config.yaml"
LOCAL_TEST="test-hotreload.sh"

echo "============================================"
echo "  Deploy Rust AGI to Production Server"
echo "============================================"
echo ""
echo "Server: $USER@$SERVER"
echo "Path: $REMOTE_PATH"
echo ""

# ตรวจสอบว่า build แล้วหรือยัง
if [ ! -f "$LOCAL_BINARY" ]; then
    echo "❌ Binary not found: $LOCAL_BINARY"
    echo "   Please build first: cargo build --release"
    exit 1
fi

echo "📦 Step 1: Creating remote directory..."
ssh $USER@$SERVER "mkdir -p $REMOTE_PATH"
echo "✅ Done"
echo ""

echo "📤 Step 2: Uploading files..."

# Upload binary
echo "   → Uploading binary..."
scp $LOCAL_BINARY $USER@$SERVER:$REMOTE_PATH/
echo "   ✓ Binary uploaded"

# Upload config
echo "   → Uploading config..."
scp $LOCAL_CONFIG $USER@$SERVER:$REMOTE_PATH/
echo "   ✓ Config uploaded"

# Upload test script (optional)
if [ -f "$LOCAL_TEST" ]; then
    echo "   → Uploading test script..."
    scp $LOCAL_TEST $USER@$SERVER:$REMOTE_PATH/
    echo "   ✓ Test script uploaded"
fi

echo "✅ All files uploaded"
echo ""

echo "🔧 Step 3: Setting permissions..."
ssh $USER@$SERVER "chmod +x $REMOTE_PATH/rust_agi_example && \
                   chmod 644 $REMOTE_PATH/config.yaml && \
                   chmod +x $REMOTE_PATH/test-hotreload.sh 2>/dev/null || true && \
                   chown -R asterisk:asterisk $REMOTE_PATH/ 2>/dev/null || true"
echo "✅ Permissions set"
echo ""

echo "🔍 Step 4: Verifying installation..."
ssh $USER@$SERVER "ls -lh $REMOTE_PATH/"
echo ""

echo "============================================"
echo "  ✅ Deployment Complete!"
echo "============================================"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Verify files on server:"
echo "   ssh $USER@$SERVER 'ls -lh $REMOTE_PATH/'"
echo ""
echo "2. Check config:"
echo "   ssh $USER@$SERVER 'cat $REMOTE_PATH/config.yaml | head -20'"
echo ""
echo "3. Configure Asterisk dialplan:"
echo "   AGI($REMOTE_PATH/rust_agi_example)"
echo ""
echo "4. Test hot reload:"
echo "   ssh $USER@$SERVER 'cd $REMOTE_PATH && ./test-hotreload.sh config.yaml'"
echo ""
echo "5. View logs:"
echo "   ssh $USER@$SERVER 'tail -f /var/log/asterisk/full | grep Config'"
echo ""
