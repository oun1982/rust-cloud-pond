# Manual Installation Guide

คู่มือติดตั้งแบบ Manual (ไม่ใช้ install.sh)

## 📋 ขั้นตอนการติดตั้ง

### 1. สร้างโฟลเดอร์

```bash
sudo mkdir -p /var/lib/asterisk/agi-bin
```

### 2. คัดลอก Binary

```bash
# คัดลอกไฟล์
sudo cp target/release/rust_agi_example /var/lib/asterisk/agi-bin/

# ตั้งค่าสิทธิ์ให้รันได้
sudo chmod +x /var/lib/asterisk/agi-bin/rust_agi_example
```

### 3. คัดลอก Config File

```bash
# คัดลอกไฟล์
sudo cp config.yaml /var/lib/asterisk/agi-bin/config.yaml

# ตั้งค่าสิทธิ์ให้อ่านได้
sudo chmod 644 /var/lib/asterisk/agi-bin/config.yaml
```

### 4. ตั้งค่า Owner (Optional แต่แนะนำ)

```bash
# ตั้งค่า owner เป็น asterisk user
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/rust_agi_example
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/config.yaml
```

### 5. ตรวจสอบการติดตั้ง

```bash
# ตรวจสอบว่าไฟล์อยู่ที่ถูกต้อง
ls -lh /var/lib/asterisk/agi-bin/

# ควรเห็น:
# -rwxr-xr-x 1 asterisk asterisk 2.6M Oct 24 14:27 rust_agi_example
# -rw-r--r-- 1 asterisk asterisk 2.3K Oct 24 14:21 config.yaml
```

### 6. ทดสอบ Binary

```bash
# ทดสอบว่า binary รันได้
file /var/lib/asterisk/agi-bin/rust_agi_example

# ควรเห็น: ELF 64-bit LSB pie executable, x86-64
```

### 7. แก้ไข Config ตามต้องการ

```bash
sudo nano /var/lib/asterisk/agi-bin/config.yaml
```

**ตัวอย่าง Config:**
```yaml
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

dids:
  "YOUR_DID_HERE":
    welcome_sound: "custom/welcome"
    queues:
      "1": "10001"
      "2": "10002"
    min_extension_digits: 3
    max_extension_digits: 4
```

### 8. ตั้งค่า Asterisk Dialplan

```bash
sudo nano /etc/asterisk/extensions.conf
```

**เพิ่มในไฟล์:**
```
[from-external]
; IVR สำหรับ DID
exten => YOUR_DID,1,NoOp(Incoming IVR call to ${EXTEN})
exten => YOUR_DID,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => YOUR_DID,n,Hangup()

; หรือใช้ pattern matching สำหรับหลาย DID
exten => _02XXXXXXXX,1,NoOp(Incoming call from ${CALLERID(num)} to ${EXTEN})
exten => _02XXXXXXXX,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => _02XXXXXXXX,n,Hangup()
```

### 9. Reload Asterisk

```bash
# Reload dialplan
asterisk -rx "dialplan reload"

# ตรวจสอบว่า reload สำเร็จ
asterisk -rx "dialplan show from-external"
```

### 10. ทดสอบการทำงาน

```bash
# ดู log realtime
tail -f /var/log/asterisk/full | grep -E "(AGI|Config)"

# หรือใน Asterisk CLI
asterisk -rvvv
```

## 🔍 การตรวจสอบหลังติดตั้ง

### ตรวจสอบไฟล์

```bash
# ตรวจสอบ binary
ls -lh /var/lib/asterisk/agi-bin/rust_agi_example
file /var/lib/asterisk/agi-bin/rust_agi_example

# ตรวจสอบ config
ls -lh /var/lib/asterisk/agi-bin/config.yaml
cat /var/lib/asterisk/agi-bin/config.yaml | head -20
```

### ตรวจสอบ Permission

```bash
# Binary ต้องมีสิทธิ์ execute (x)
stat /var/lib/asterisk/agi-bin/rust_agi_example

# Config ต้องอ่านได้
stat /var/lib/asterisk/agi-bin/config.yaml
```

### ตรวจสอบ Asterisk Dialplan

```bash
# ดูว่า AGI ถูก load หรือไม่
asterisk -rx "dialplan show from-external" | grep AGI

# ควรเห็น:
# n. AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
```

## 🧪 ทดสอบการทำงาน

### 1. ทดสอบโทรเข้า

โทรเข้า DID ที่ตั้งค่าไว้ และทดสอบ:
- ✅ ได้ยินเสียง IVR
- ✅ กดตัวเลข 1-9 เข้า queue
- ✅ กดเบอร์ภายใน 3-4 หลัก

### 2. ดู Log

```bash
# Terminal 1: ดู log
tail -f /var/log/asterisk/full

# Terminal 2: โทรเข้า

# ใน Terminal 1 ควรเห็น:
# ✓ Config loaded from: /var/lib/asterisk/agi-bin/config.yaml
# ✓ Config watcher started for: /var/lib/asterisk/agi-bin/config.yaml
# Incoming call - DID: YOUR_DID
# Routing to queue: 10001
```

### 3. ทดสอบ Hot Reload

```bash
# Terminal 1: watch changes
cd /opt/rust-project/rust-dialplan
./test-hotreload.sh /var/lib/asterisk/agi-bin/config.yaml

# Terminal 2: แก้ไข config
sudo nano /var/lib/asterisk/agi-bin/config.yaml
# เปลี่ยนค่า queue หรือ welcome_sound
# บันทึกไฟล์

# Terminal 3: โทรเข้าทดสอบ
# ควรใช้ค่าใหม่ทันที!
```

## 🐛 Troubleshooting

### ปัญหา: Permission denied

```bash
# แก้ไข permission
sudo chmod +x /var/lib/asterisk/agi-bin/rust_agi_example
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/rust_agi_example
```

### ปัญหา: Config ไม่โหลด

```bash
# ตรวจสอบว่าไฟล์มีอยู่
ls -la /var/lib/asterisk/agi-bin/config.yaml

# ตรวจสอบ YAML syntax
cat /var/lib/asterisk/agi-bin/config.yaml

# ตรวจสอบ permission
sudo chmod 644 /var/lib/asterisk/agi-bin/config.yaml
```

### ปัญหา: AGI ไม่ทำงาน

```bash
# ตรวจสอบว่า Asterisk เห็น AGI script
asterisk -rx "agi show commands" | grep -i custom

# ตรวจสอบ dialplan
asterisk -rx "dialplan show from-external"

# ทดสอบรัน binary โดยตรง (จะไม่ทำงานเพราะต้องรับ input จาก Asterisk)
/var/lib/asterisk/agi-bin/rust_agi_example
# กด Ctrl+C เพื่อยกเลิก
```

### ปัญหา: Hot reload ไม่ทำงาน

```bash
# ตรวจสอบ inotify limit
cat /proc/sys/fs/inotify/max_user_watches

# เพิ่ม limit ถ้าน้อยเกินไป
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

## 📦 โครงสร้างไฟล์หลังติดตั้ง

```
/var/lib/asterisk/agi-bin/
├── rust_agi_example          # Binary (2.6 MB)
└── config.yaml                # Config file (2.3 KB)

/etc/asterisk/
└── extensions.conf            # Asterisk dialplan (มี AGI config)

/var/log/asterisk/
└── full                       # Log file
```

## 🔄 Update/Upgrade

### อัพเดท Binary

```bash
# Backup เก่า
sudo cp /var/lib/asterisk/agi-bin/rust_agi_example \
        /var/lib/asterisk/agi-bin/rust_agi_example.backup.$(date +%Y%m%d)

# คัดลอกใหม่
sudo cp target/release/rust_agi_example /var/lib/asterisk/agi-bin/
sudo chmod +x /var/lib/asterisk/agi-bin/rust_agi_example
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/rust_agi_example

# ไม่ต้อง restart Asterisk - สายถัดไปจะใช้ binary ใหม่
```

### อัพเดท Config

```bash
# Backup เก่า
sudo cp /var/lib/asterisk/agi-bin/config.yaml \
        /var/lib/asterisk/agi-bin/config.yaml.backup.$(date +%Y%m%d)

# แก้ไข config
sudo nano /var/lib/asterisk/agi-bin/config.yaml

# Hot reload จะทำงานอัตโนมัติ!
```

## 🗑️ Uninstall

```bash
# ลบไฟล์
sudo rm /var/lib/asterisk/agi-bin/rust_agi_example
sudo rm /var/lib/asterisk/agi-bin/config.yaml

# แก้ไข dialplan (ลบ AGI config)
sudo nano /etc/asterisk/extensions.conf

# Reload
asterisk -rx "dialplan reload"
```

## 📊 Summary

| Step | Command | Description |
|------|---------|-------------|
| 1 | `mkdir -p /var/lib/asterisk/agi-bin` | สร้างโฟลเดอร์ |
| 2 | `cp binary + chmod +x` | คัดลอก binary |
| 3 | `cp config + chmod 644` | คัดลอก config |
| 4 | `chown asterisk:asterisk` | ตั้งค่า owner |
| 5 | `nano extensions.conf` | ตั้งค่า dialplan |
| 6 | `asterisk -rx "dialplan reload"` | Reload Asterisk |
| 7 | ทดสอบโทรเข้า | ทดสอบการทำงาน |

---

**ใช้เวลาติดตั้ง:** ~5-10 นาที  
**ต้องการ restart Asterisk:** ไม่ต้อง (เพียง reload dialplan)  
**Hot reload:** ใช้งานได้ทันทีหลังติดตั้ง
