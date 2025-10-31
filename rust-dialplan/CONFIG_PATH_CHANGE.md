# เปลี่ยน Config Path เป็น /var/lib/asterisk/agi-bin

## สิ่งที่เปลี่ยนแปลง

### 1. Config Path Priority
```
/var/lib/asterisk/agi-bin/config.yaml         ← ลำดับ 1 (แนะนำ)
/var/lib/asterisk/agi-bin/ivr-config.yaml     ← ลำดับ 2
/etc/asterisk/ivr-config.yaml                  ← ลำดับ 3
/usr/local/etc/asterisk/ivr-config.yaml        ← ลำดับ 4
./config.yaml                                   ← ลำดับ 5
/opt/rust-project/rust-dialplan/config.yaml    ← ลำดับ 6
```

### 2. Binary Location
```
เดิม: /usr/local/bin/rust_agi_example
ใหม่: /var/lib/asterisk/agi-bin/rust_agi_example
```

### 3. Config Location  
```
เดิม: /etc/asterisk/ivr-config.yaml
ใหม่: /var/lib/asterisk/agi-bin/config.yaml
```

## ทำไมใช้ /var/lib/asterisk/agi-bin?

✅ **เหตุผล:**
1. เป็น path มาตรฐานของ Asterisk AGI scripts
2. วางทั้ง binary และ config ไว้ที่เดียวกัน ง่ายต่อการจัดการ
3. Asterisk มี permission access อยู่แล้ว
4. Backup/restore ทำได้ง่าย (backup ทั้งโฟลเดอร์เดียว)

## วิธีติดตั้ง

### แบบอัตโนมัติ
```bash
sudo ./install.sh
```

### แบบ Manual
```bash
# สร้างโฟลเดอร์
sudo mkdir -p /var/lib/asterisk/agi-bin

# Copy binary
sudo cp target/release/rust_agi_example /var/lib/asterisk/agi-bin/
sudo chmod +x /var/lib/asterisk/agi-bin/rust_agi_example

# Copy config
sudo cp config.yaml /var/lib/asterisk/agi-bin/
sudo chmod 644 /var/lib/asterisk/agi-bin/config.yaml

# ตั้งค่า owner (ถ้าต้องการ)
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/rust_agi_example
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/config.yaml
```

## การใช้งาน

### Asterisk Dialplan (extensions.conf)
```
[from-external]
exten => YOUR_DID,1,NoOp(IVR Call)
exten => YOUR_DID,n,AGI(/var/lib/asterisk/agi-bin/rust_agi_example)
exten => YOUR_DID,n,Hangup()
```

### แก้ไข Config
```bash
nano /var/lib/asterisk/agi-bin/config.yaml
# บันทึก -> Hot reload ทำงานทันที!
```

### ดู Log
```bash
tail -f /var/log/asterisk/full | grep -E "(AGI|Config)"
```

## ทดสอบ Hot Reload
```bash
./test-hotreload.sh /var/lib/asterisk/agi-bin/config.yaml
```

## Backup & Restore

### Backup
```bash
tar -czf agi-ivr-backup-$(date +%Y%m%d).tar.gz /var/lib/asterisk/agi-bin/
```

### Restore
```bash
tar -xzf agi-ivr-backup-YYYYMMDD.tar.gz -C /
```

## Migration จาก Path เก่า

หากติดตั้งไว้ที่ `/usr/local/bin` หรือ `/etc/asterisk` แล้ว:

```bash
# Backup เก่า
sudo cp /etc/asterisk/ivr-config.yaml /etc/asterisk/ivr-config.yaml.backup

# สร้างโฟลเดอร์ใหม่
sudo mkdir -p /var/lib/asterisk/agi-bin

# ย้าย binary
sudo mv /usr/local/bin/rust_agi_example /var/lib/asterisk/agi-bin/

# ย้าย config
sudo mv /etc/asterisk/ivr-config.yaml /var/lib/asterisk/agi-bin/config.yaml

# อัพเดท dialplan
sudo nano /etc/asterisk/extensions.conf
# เปลี่ยน path เป็น /var/lib/asterisk/agi-bin/rust_agi_example

# Reload
asterisk -rx "dialplan reload"
```

---
**Updated**: October 24, 2025  
**Version**: 1.1.0
