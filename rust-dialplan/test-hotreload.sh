#!/bin/bash
# Hot Reload Test Script

echo "==================================="
echo "  Hot Reload Test"
echo "==================================="
echo ""

CONFIG_FILE="${1:-./config.yaml}"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Config file not found: $CONFIG_FILE"
    echo "Usage: $0 [config_file]"
    exit 1
fi

echo "📁 Config file: $CONFIG_FILE"
echo ""

# แสดง MD5 checksum ปัจจุบัน
echo "Current checksum:"
md5sum "$CONFIG_FILE"
echo ""

# รอให้ผู้ใช้พร้อม
echo "Instructions:"
echo "1. Keep this terminal open"
echo "2. In another terminal, start the AGI program"
echo "3. Then modify the config file: nano $CONFIG_FILE"
echo "4. Save the file and check the AGI program logs"
echo "5. You should see: '✓ Config reloaded successfully!'"
echo ""

echo "Watching for changes..."
echo "Press Ctrl+C to stop"
echo ""

# Watch file changes
while true; do
    inotifywait -e modify,close_write "$CONFIG_FILE" 2>/dev/null
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "🔄 [$(date '+%Y-%m-%d %H:%M:%S')] Config file changed!"
        echo "New checksum:"
        md5sum "$CONFIG_FILE"
        echo ""
        echo "✅ Hot reload should trigger now in the AGI program"
        echo ""
        echo "Watching for more changes..."
    fi
done
