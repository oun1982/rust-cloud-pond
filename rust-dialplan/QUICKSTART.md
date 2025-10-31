# Quick Start Guide - Rust AGI IVR with Hot Reload

## 🚀 สิ่งที่ได้เพิ่มเข้ามา

### ✅ 1. รองรับหลาย DID
- แต่ละ DID สามารถมี config แยกกันได้
- เสียงต้อนรับ, queue mapping, การตั้งค่าต่างๆ แยกกันตาม DID
- มี default config สำหรับ DID ที่ไม่ได้กำหนด

### ✅ 2. ไฟล์ Config แยกออกมา (YAML)
- ไฟล์ config: `config.yaml` หรือ `/etc/asterisk/ivr-config.yaml`
- Format: YAML อ่าน-แก้ไขง่าย
- มีความ comment อธิบายในไฟล์

### ✅ 3. Hot Reload
- **ไม่ต้อง restart AGI หรือ Asterisk**
- แก้ไข config แล้ว save ทำงานทันที
- ใช้ file watcher (inotify) ตรวจจับการเปลี่ยนแปลง
- สายถัดไปจะใช้ config ใหม่ทันที

## 📦 ไฟล์ที่สำคัญ

```
/opt/rust-project/rust-dialplan/
├── target/release/rust_agi_example    # Binary (2.6 MB)
├── config.yaml                        # Config file หลัก
├── config-example.yaml                # ตัวอย่าง config
├── install.sh                         # สคริปต์ติดตั้ง
├── test-hotreload.sh                  # ทดสอบ hot reload
└── README.md                          # คู่มือใช้งานฉบับเต็ม
```

## ⚡ วิธีติดตั้ง (Quick)

```bash
cd /opt/rust-project/rust-dialplan

# 1. ติดตั้งด้วยสคริปต์
sudo ./install.sh

# 2. แก้ไข config ตามต้องการ
sudo nano /var/lib/asterisk/agi-bin/config.yaml

# 3. ตั้งค่า Asterisk (extensions.conf)
sudo nano /etc/asterisk/extensions.conf
```

เพิ่มใน extensions.conf:
```
[from-external]
exten => YOUR_DID,1,NoOp(IVR Call)
exten => YOUR_DID,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => YOUR_DID,n,Hangup()
```

```bash
# 4. Reload Asterisk
asterisk -rx "dialplan reload"
```

## 🧪 ทดสอบ Hot Reload

### Terminal 1: Watch config changes
```bash
./test-hotreload.sh /var/lib/asterisk/agi-bin/config.yaml
```

### Terminal 2: ทดสอบโทร
```bash
# โทรเข้า DID และดู log
tail -f /var/log/asterisk/full | grep -E "(AGI|Config)"
```

### Terminal 3: แก้ไข config
```bash
# แก้ไข config
nano /var/lib/asterisk/agi-bin/config.yaml

# เช่น เปลี่ยน queue mapping
queues:
  "1": "20001"  # เปลี่ยนจาก 10001 เป็น 20001

# บันทึกไฟล์ -> Hot reload จะทำงานทันที!
```

## 📊 Log Messages ที่ควรเห็น

เมื่อโปรแกรมเริ่มทำงาน:
```
✓ Config loaded from: /var/lib/asterisk/agi-bin/config.yaml
✓ Config watcher started for: /var/lib/asterisk/agi-bin/config.yaml
```

เมื่อมีสายเรียกเข้า:
```
Incoming call - DID: 0212345678
```

เมื่อแก้ไข config:
```
✓ Config reloaded successfully!
```

เมื่อผู้โทรกดตัวเลือก:
```
Routing to queue: 10001
# หรือ
Dialing extension: 6789
```

## 🔧 ตัวอย่าง Config Structure

```yaml
# Default config
default:
  welcome_sound: "en/custom/new-ivr-osd"
  queues:
    "1": "10001"
    "2": "10002"
  min_extension_digits: 3
  max_extension_digits: 4

# DID-specific
dids:
  "0212345678":
    welcome_sound: "custom/welcome-hq"
    queues:
      "1": "10001"  # ฝ่ายขาย
      "2": "10002"  # บริการลูกค้า
      "3": "10003"  # เทคนิค
```

## 💡 การใช้งานจริง

### เพิ่ม DID ใหม่
1. เปิดไฟล์: `nano /var/lib/asterisk/agi-bin/config.yaml`
2. เพิ่ม DID ใน section `dids:`
3. บันทึกไฟล์
4. **ไม่ต้องทำอะไรเพิ่ม** - มีผลทันที!

### เปลี่ยน Queue Mapping
1. แก้ไขในไฟล์ config
2. บันทึก
3. สายถัดไปจะใช้ค่าใหม่

### เปลี่ยนไฟล์เสียง
1. แก้ `welcome_sound`, `invalid_sound`, `goodbye_sound`
2. บันทึก
3. มีผลทันที

## 🐛 Troubleshooting

### ถ้า Hot Reload ไม่ทำงาน
```bash
# ตรวจสอบ inotify
cat /proc/sys/fs/inotify/max_user_watches

# เพิ่มค่าถ้าน้อย
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

### ถ้า Config ไม่โหลด
```bash
# ตรวจสอบ syntax
cat /var/lib/asterisk/agi-bin/config.yaml

# ตรวจสอบ permission
ls -la /var/lib/asterisk/agi-bin/config.yaml
```

## 🎯 Features Summary

| Feature | Status | Description |
|---------|--------|-------------|
| Multi-DID Support | ✅ | แต่ละ DID มี config แยกกัน |
| External Config | ✅ | YAML format, อ่านง่าย |
| Hot Reload | ✅ | Auto reload เมื่อแก้ไข config |
| Queue Routing | ✅ | กด 1-9 เข้า queue ได้ทันที |
| Extension Dial | ✅ | รองรับเบอร์ภายใน 3-4 หลัก |
| Interrupt DTMF | ✅ | กดได้ระหว่างเล่นเสียง |
| Logging | ✅ | Log ไปยัง stderr/Asterisk |

## 📦 Binary Info

- **Path**: `target/release/rust_agi_example`
- **Size**: 2.6 MB (stripped)
- **Platform**: x86_64 Linux (Ubuntu 24.04 compatible)
- **Type**: ELF 64-bit LSB executable

## 🚢 Deploy to Server

```bash
# Method 1: SCP
scp target/release/rust_agi_example user@server:/tmp/
scp config.yaml user@server:/tmp/config.yaml
ssh user@server "sudo mkdir -p /var/lib/asterisk/agi-bin && \
                 sudo mv /tmp/rust_agi_example /var/lib/asterisk/agi-bin/ && \
                 sudo chmod +x /var/lib/asterisk/agi-bin/rust_agi_example && \
                 sudo mv /tmp/config.yaml /var/lib/asterisk/agi-bin/"

# Method 2: ใช้ install script
scp -r /opt/rust-project/rust-dialplan user@server:/tmp/
ssh user@server "cd /tmp/rust-dialplan && sudo ./install.sh"
```

---

**Created**: October 24, 2025  
**Version**: 1.0.0  
**Platform**: Rust + Asterisk AGI
