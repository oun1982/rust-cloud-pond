#!/bin/bash
# Installation script for Rust AGI IVR System

set -e

echo "============================================"
echo "  Rust AGI IVR System - Installation"
echo "============================================"
echo ""

# ตรวจสอบว่ารันด้วย root หรือไม่
if [ "$EUID" -ne 0 ]; then 
    echo "⚠️  Please run as root or with sudo"
    exit 1
fi

# ตัวแปร
BINARY_SOURCE="target/release/rust_agi_example"
BINARY_DEST="/var/lib/asterisk/agi-bin/rust-agi/rust_agi_example"
CONFIG_SOURCE="config.yaml"
CONFIG_DEST="/var/lib/asterisk/agi-bin/rust-agi/config.yaml"

echo "📦 Step 1: Checking files..."

# ตรวจสอบว่ามี binary หรือไม่
if [ ! -f "$BINARY_SOURCE" ]; then
    echo "❌ Binary not found: $BINARY_SOURCE"
    echo "   Please build first: cargo build --release"
    exit 1
fi

# ตรวจสอบว่ามี config หรือไม่
if [ ! -f "$CONFIG_SOURCE" ]; then
    echo "❌ Config file not found: $CONFIG_SOURCE"
    exit 1
fi

echo "✅ All files found"
echo ""

echo "📋 Step 2: Installing binary..."
# สร้างโฟลเดอร์ถ้ายังไม่มี
mkdir -p "$(dirname "$BINARY_DEST")"
cp "$BINARY_SOURCE" "$BINARY_DEST"
chmod +x "$BINARY_DEST"
echo "✅ Binary installed to: $BINARY_DEST"
echo "   Size: $(du -h $BINARY_DEST | cut -f1)"
echo ""

echo "⚙️  Step 3: Installing config..."
# สร้างโฟลเดอร์ถ้ายังไม่มี
mkdir -p "$(dirname "$CONFIG_DEST")"
if [ -f "$CONFIG_DEST" ]; then
    # สำรองไฟล์เก่า
    BACKUP_FILE="${CONFIG_DEST}.backup.$(date +%Y%m%d_%H%M%S)"
    cp "$CONFIG_DEST" "$BACKUP_FILE"
    echo "📦 Backup old config to: $BACKUP_FILE"
fi

cp "$CONFIG_SOURCE" "$CONFIG_DEST"
chmod 644 "$CONFIG_DEST"
echo "✅ Config installed to: $CONFIG_DEST"
echo ""

echo "🔍 Step 4: Verifying installation..."

# ตรวจสอบ binary
if [ -x "$BINARY_DEST" ]; then
    echo "✅ Binary is executable"
else
    echo "❌ Binary is not executable"
    exit 1
fi

# ตรวจสอบ config
if [ -r "$CONFIG_DEST" ]; then
    echo "✅ Config is readable"
else
    echo "❌ Config is not readable"
    exit 1
fi

echo ""
echo "============================================"
echo "  ✅ Installation Complete!"
echo "============================================"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Edit config file:"
echo "   nano $CONFIG_DEST"
echo ""
echo "2. Configure Asterisk dialplan (/etc/asterisk/extensions.conf):"
echo "   exten => YOUR_DID,1,NoOp(Incoming call)"
echo "   exten => YOUR_DID,n,AGI($BINARY_DEST)"
echo "   exten => YOUR_DID,n,Hangup()"
echo ""
echo "3. Reload Asterisk:"
echo "   asterisk -rx 'dialplan reload'"
echo ""
echo "4. Test your IVR by calling the DID"
echo ""
echo "💡 Tips:"
echo "   - Config changes are applied automatically (hot reload)"
echo "   - Check logs: tail -f /var/log/asterisk/full | grep AGI"
echo "   - Binary location: $BINARY_DEST"
echo "   - Config location: $CONFIG_DEST"
echo ""
