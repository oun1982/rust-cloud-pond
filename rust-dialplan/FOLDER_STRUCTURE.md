# โครงสร้างโฟลเดอร์ rust-agi

## 📁 โครงสร้าง

```
/var/lib/asterisk/agi-bin/
└── rust-agi/
    ├── rust_agi_example        # Binary (2.6 MB)
    ├── config.yaml              # Config file
    └── test-hotreload.sh        # (Optional) ทดสอบ hot reload
```

## ✅ ข้อดีของโครงสร้างนี้

1. **จัดระเบียบดี** - แยก Rust AGI ออกเป็นโฟลเดอร์เฉพาะ
2. **ไม่ปนกับ AGI อื่น** - ถ้ามี AGI script อื่นจะไม่สับสน
3. **Backup ง่าย** - backup ทั้งโฟลเดอร์เดียว
4. **Update ง่าย** - อัพเดทเฉพาะใน folder นี้

## 🚀 วิธีติดตั้ง

### แบบอัตโนมัติ
```bash
cd /opt/rust-project/rust-dialplan
sudo ./install.sh
```

### แบบ Manual

```bash
# 1. สร้างโฟลเดอร์
sudo mkdir -p /var/lib/asterisk/agi-bin/rust-agi

# 2. คัดลอก binary
sudo cp target/release/rust_agi_example /var/lib/asterisk/agi-bin/rust-agi/
sudo chmod +x /var/lib/asterisk/agi-bin/rust-agi/rust_agi_example

# 3. คัดลอก config
sudo cp config.yaml /var/lib/asterisk/agi-bin/rust-agi/
sudo chmod 644 /var/lib/asterisk/agi-bin/rust-agi/config.yaml

# 4. (Optional) คัดลอก test script
sudo cp test-hotreload.sh /var/lib/asterisk/agi-bin/rust-agi/
sudo chmod +x /var/lib/asterisk/agi-bin/rust-agi/test-hotreload.sh

# 5. ตั้งค่า owner
sudo chown -R asterisk:asterisk /var/lib/asterisk/agi-bin/rust-agi/
```

## 📝 Asterisk Dialplan

แก้ไขไฟล์ `/etc/asterisk/extensions.conf`:

```
[from-external]
; IVR สำหรับ DID
exten => YOUR_DID,1,NoOp(Incoming IVR call)
exten => YOUR_DID,n,AGI(/var/lib/asterisk/agi-bin/rust-agi/rust_agi_example)
exten => YOUR_DID,n,Hangup()

; หรือใช้ pattern matching
exten => _02XXXXXXXX,1,NoOp(Call from ${CALLERID(num)} to ${EXTEN})
exten => _02XXXXXXXX,n,AGI(/var/lib/asterisk/agi-bin/rust-agi/rust_agi_example)
exten => _02XXXXXXXX,n,Hangup()
```

## 🔄 Hot Reload

```bash
# แก้ไข config
sudo nano /var/lib/asterisk/agi-bin/rust-agi/config.yaml

# บันทึก -> มีผลทันที!
```

## 🧪 ทดสอบ Hot Reload

```bash
cd /var/lib/asterisk/agi-bin/rust-agi
./test-hotreload.sh config.yaml
```

หรือ

```bash
./test-hotreload.sh /var/lib/asterisk/agi-bin/rust-agi/config.yaml
```

## 📊 Config Path Priority

โปรแกรมจะค้นหา config จาก path ต่อไปนี้ตามลำดับ:

1. `/var/lib/asterisk/agi-bin/rust-agi/config.yaml` ⭐ แนะนำ
2. `/var/lib/asterisk/agi-bin/rust-agi/ivr-config.yaml`
3. `/var/lib/asterisk/agi-bin/config.yaml`
4. `/var/lib/asterisk/agi-bin/ivr-config.yaml`
5. `/etc/asterisk/ivr-config.yaml`
6. `/usr/local/etc/asterisk/ivr-config.yaml`
7. `./config.yaml`
8. `/opt/rust-project/rust-dialplan/config.yaml`

## 📦 Deploy to Server

### จาก Development

```bash
# ส่งทั้ง 3 ไฟล์
scp target/release/rust_agi_example root@10.133.1.12:/var/lib/asterisk/agi-bin/rust-agi/
scp config.yaml root@10.133.1.12:/var/lib/asterisk/agi-bin/rust-agi/
scp test-hotreload.sh root@10.133.1.12:/var/lib/asterisk/agi-bin/rust-agi/

# ตั้งค่า permission
ssh root@10.133.1.12 "chmod +x /var/lib/asterisk/agi-bin/rust-agi/rust_agi_example && \
                       chmod +x /var/lib/asterisk/agi-bin/rust-agi/test-hotreload.sh && \
                       chmod 644 /var/lib/asterisk/agi-bin/rust-agi/config.yaml && \
                       chown -R asterisk:asterisk /var/lib/asterisk/agi-bin/rust-agi/"
```

### Deploy เฉพาะ Binary (Update)

```bash
# Backup เก่า
ssh root@10.133.1.12 "cp /var/lib/asterisk/agi-bin/rust-agi/rust_agi_example \
                          /var/lib/asterisk/agi-bin/rust-agi/rust_agi_example.backup.\$(date +%Y%m%d)"

# Deploy ใหม่
scp target/release/rust_agi_example root@10.133.1.12:/var/lib/asterisk/agi-bin/rust-agi/

# ไม่ต้อง restart - สายถัดไปใช้ version ใหม่
```

### Deploy เฉพาะ Config (Update)

```bash
# Backup เก่า
ssh root@10.133.1.12 "cp /var/lib/asterisk/agi-bin/rust-agi/config.yaml \
                          /var/lib/asterisk/agi-bin/rust-agi/config.yaml.backup.\$(date +%Y%m%d)"

# Deploy ใหม่
scp config.yaml root@10.133.1.12:/var/lib/asterisk/agi-bin/rust-agi/

# Hot reload ทำงานอัตโนมัติ!
```

## 🔍 ตรวจสอบการติดตั้ง

```bash
# SSH เข้า server
ssh root@10.133.1.12

# ตรวจสอบโครงสร้าง
ls -lh /var/lib/asterisk/agi-bin/rust-agi/

# ควรเห็น:
# -rwxr-xr-x 1 asterisk asterisk 2.6M Oct 24 15:31 rust_agi_example
# -rw-r--r-- 1 asterisk asterisk 2.3K Oct 24 14:21 config.yaml
# -rwxr-xr-x 1 asterisk asterisk 1.3K Oct 24 14:24 test-hotreload.sh

# ตรวจสอบว่า binary ใช้งานได้
file /var/lib/asterisk/agi-bin/rust-agi/rust_agi_example

# ตรวจสอบ config
cat /var/lib/asterisk/agi-bin/rust-agi/config.yaml | head -20
```

## 📊 Log & Debug

```bash
# ดู log realtime
tail -f /var/log/asterisk/full | grep -E "(rust_agi|Config)"

# ตรวจสอบว่า config โหลดจาก path ไหน
tail -f /var/log/asterisk/full | grep "Config loaded from"

# ควรเห็น:
# ✓ Config loaded from: /var/lib/asterisk/agi-bin/rust-agi/config.yaml
# ✓ Config watcher started for: /var/lib/asterisk/agi-bin/rust-agi/config.yaml
```

## 🧹 Backup & Restore

### Backup
```bash
# Backup ทั้งโฟลเดอร์
tar -czf rust-agi-backup-$(date +%Y%m%d-%H%M%S).tar.gz \
         /var/lib/asterisk/agi-bin/rust-agi/

# ย้ายไปเก็บ
mv rust-agi-backup-*.tar.gz /backup/
```

### Restore
```bash
# Restore จาก backup
tar -xzf /backup/rust-agi-backup-YYYYMMDD-HHMMSS.tar.gz -C /

# ตั้งค่า permission
chown -R asterisk:asterisk /var/lib/asterisk/agi-bin/rust-agi/
```

## 🗑️ Uninstall

```bash
# ลบทั้งโฟลเดอร์
sudo rm -rf /var/lib/asterisk/agi-bin/rust-agi/

# แก้ไข dialplan (ลบ AGI config)
sudo nano /etc/asterisk/extensions.conf

# Reload
asterisk -rx "dialplan reload"
```

## 💡 Tips

### เปลี่ยนชื่อ Binary (ถ้าต้องการ)

```bash
# เปลี่ยนชื่อให้สั้นกว่า
cd /var/lib/asterisk/agi-bin/rust-agi/
mv rust_agi_example ivr

# อัพเดท dialplan
AGI(/var/lib/asterisk/agi-bin/rust-agi/ivr)
```

### หลาย Version

```bash
# เก็บหลาย version
/var/lib/asterisk/agi-bin/rust-agi/
├── rust_agi_example           # version ปัจจุบัน
├── rust_agi_example.v1.0      # backup v1.0
├── rust_agi_example.v1.1      # backup v1.1
└── config.yaml

# Switch version
mv rust_agi_example rust_agi_example.current
mv rust_agi_example.v1.0 rust_agi_example
```

---

**โครงสร้างนี้เหมาะสำหรับ:**
- ✅ Production server
- ✅ จัดการหลาย AGI scripts
- ✅ Version control
- ✅ Backup & Restore ง่าย
