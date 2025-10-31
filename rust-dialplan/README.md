# Rust AGI IVR System with Hot Reload

ระบบ IVR สำหรับ Asterisk เขียนด้วย Rust รองรับการ config หลาย DID และ hot reload

## ✨ Features

- ✅ รองรับหลาย DID (แต่ละ DID มี IVR ต่างกัน)
- ✅ Hot Reload - แก้ไข config แล้วมีผลทันที ไม่ต้อง restart
- ✅ กดตัวเลข 1-9 เข้า Queue ได้ทันที
- ✅ รองรับการกดเบอร์ภายใน 3-4 หลัก
- ✅ สามารถกดระหว่างเล่นเสียงได้ (interrupt)
- ✅ Config file เป็น YAML อ่านง่าย

## 📦 Installation

### 1. คัดลอกไฟล์ไปยัง server

```bash
# คัดลอก binary
scp target/release/rust_agi_example user@server:/var/lib/asterisk/agi-bin/

# คัดลอก config file
scp config.yaml user@server:/var/lib/asterisk/agi-bin/config.yaml

# ตั้งค่า permission
ssh user@server "chmod +x /var/lib/asterisk/agi-bin/rust_agi_example"
```

### 2. ตั้งค่า Asterisk

แก้ไขไฟล์ `/etc/asterisk/extensions.conf`:

```
[from-external]
; DID หลัก
exten => 0212345678,1,NoOp(Incoming call to main office)
exten => 0212345678,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => 0212345678,n,Hangup()

; DID สาขา 1
exten => 0298765432,1,NoOp(Incoming call to branch 1)
exten => 0298765432,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => 0298765432,n,Hangup()

; DID ศูนย์บริการลูกค้า
exten => 0223456789,1,NoOp(Incoming call to support center)
exten => 0223456789,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => 0223456789,n,Hangup()
```

### 3. Reload Asterisk

```bash
asterisk -rx "dialplan reload"
```

## ⚙️ Configuration

ไฟล์ config อยู่ที่: `/var/lib/asterisk/agi-bin/config.yaml`

โปรแกรมจะค้นหาไฟล์ config จาก path ต่อไปนี้ตามลำดับ:
1. `/var/lib/asterisk/agi-bin/config.yaml` (แนะนำ)
2. `/var/lib/asterisk/agi-bin/ivr-config.yaml`
3. `/etc/asterisk/ivr-config.yaml`
4. `/usr/local/etc/asterisk/ivr-config.yaml`
5. `./config.yaml`
6. `/opt/rust-project/rust-dialplan/config.yaml`

### ตัวอย่าง Config

```yaml
# Default config (ใช้เมื่อไม่พบ DID ใน list)
default:
  welcome_sound: "en/custom/new-ivr-osd"
  invalid_sound: "invalid"
  goodbye_sound: "vm-goodbye"
  queues:
    "1": "10001"
    "2": "10002"
    "3": "10003"
  min_extension_digits: 3
  max_extension_digits: 4
  extension_timeout_seconds: 3
  dial_timeout_seconds: 60
  dial_options: "t"

# DID-specific configurations
dids:
  "0212345678":
    welcome_sound: "custom/welcome-hq"
    invalid_sound: "invalid"
    goodbye_sound: "vm-goodbye"
    queues:
      "1": "10001"  # ฝ่ายขาย
      "2": "10002"  # ฝ่ายบริการลูกค้า
      "3": "10003"  # ฝ่ายเทคนิค
    min_extension_digits: 3
    max_extension_digits: 4
    extension_timeout_seconds: 3
    dial_timeout_seconds: 60
    dial_options: "t"
```

### การตั้งค่า

| Parameter | คำอธิบาย |
|-----------|---------|
| `welcome_sound` | ไฟล์เสียงต้อนรับ (ไม่ต้องใส่ .wav) |
| `invalid_sound` | ไฟล์เสียงเมื่อกดผิด |
| `goodbye_sound` | ไฟล์เสียงลาก่อน |
| `queues` | mapping ของตัวเลข 1-9 กับ queue number |
| `min_extension_digits` | จำนวนหลักขั้นต่ำของเบอร์ภายใน |
| `max_extension_digits` | จำนวนหลักสูงสุดของเบอร์ภายใน |
| `extension_timeout_seconds` | timeout ระหว่างการกดตัวเลข |
| `dial_timeout_seconds` | timeout การโทรออก |
| `dial_options` | options สำหรับ Dial command |

## 🔄 Hot Reload

เมื่อแก้ไขไฟล์ config โปรแกรมจะ **reload อัตโนมัติทันที** ไม่ต้อง restart Asterisk หรือ AGI

```bash
# แก้ไข config
nano /var/lib/asterisk/agi-bin/config.yaml

# บันทึกไฟล์ - Hot Reload จะทำงานอัตโนมัติ!
# สายถัดไปจะใช้ config ใหม่ทันที
```

## 📊 Logs

ดู log การทำงาน:

```bash
# แบบ realtime
tail -f /var/log/asterisk/full | grep AGI

# หรือใน Asterisk CLI
asterisk -rvvv
```

Log messages:
- `✓ Config loaded from: /var/lib/asterisk/agi-bin/config.yaml` - โหลด config สำเร็จ
- `✓ Config watcher started for: ...` - เริ่ม hot reload watcher
- `✓ Config reloaded successfully!` - reload config สำเร็จ
- `Incoming call - DID: 0212345678` - มีสายเรียกเข้า
- `Routing to queue: 10001` - ส่งเข้า queue
- `Dialing extension: 6789` - โทรออกไปเบอร์ภายใน

## 🧪 Testing

### ทดสอบ config file

```bash
# ตรวจสอบ syntax
cat /etc/asterisk/ivr-config.yaml | grep -v '^#' | head -20

# ทดสอบ hot reload
echo "# Test reload $(date)" >> /etc/asterisk/ivr-config.yaml
```

### ทดสอบการโทร

1. โทรเข้า DID ที่ตั้งค่าไว้
2. ฟังเสียง IVR
3. กดตัวเลข 1-9 เพื่อเข้า queue
4. หรือกดเบอร์ภายใน 3-4 หลัก

## 🛠️ Troubleshooting

### ปัญหา: ไม่พบ config file

```bash
# ตรวจสอบว่ามีไฟล์หรือไม่
ls -la /var/lib/asterisk/agi-bin/config.yaml

# ถ้าไม่มี ให้สร้างจาก template
cp config.yaml /var/lib/asterisk/agi-bin/config.yaml
```

### ปัญหา: Hot reload ไม่ทำงาน

```bash
# ตรวจสอบว่า inotify ทำงานหรือไม่
cat /proc/sys/fs/inotify/max_user_watches

# เพิ่ม limit ถ้าน้อยเกินไป
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

### ปัญหา: เสียงไม่เล่น

```bash
# ตรวจสอบว่ามีไฟล์เสียงหรือไม่
ls -la /var/lib/asterisk/sounds/en/custom/

# ตรวจสอบ format ของไฟล์
file /var/lib/asterisk/sounds/en/custom/new-ivr-osd.wav
```

## 📝 Build from Source

```bash
# Clone project
cd /opt/rust-project/rust-dialplan

# Build
cargo build --release

# Strip binary
strip target/release/rust_agi_example

# Check size
ls -lh target/release/rust_agi_example
```

## 📄 License

MIT OR Apache-2.0

## 🤝 Support

สำหรับคำถามหรือปัญหา กรุณาติดต่อทีมพัฒนา
