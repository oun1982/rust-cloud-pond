# Check Supervisor - Asterisk AGI Script

โปรแกรม AGI (Asterisk Gateway Interface) ที่เขียนด้วย Rust สำหรับตรวจสอบสถานะ agent ในฐานข้อมูล

## การติดตั้งและใช้งาน

### 1. Build โปรแกรม
```bash
cd /opt/rust-project/check-supervisor
cargo build --release
```

### 2. คัดลอกไฟล์ไปยัง AGI directory ของ Asterisk
```bash
# คัดลอกไฟล์ executable ไปยัง AGI directory (ปรับ path ตามระบบของคุณ)
sudo cp target/release/check-supervisor /var/lib/asterisk/agi-bin/check-supervisor

# ตั้งค่า permission
sudo chown asterisk:asterisk /var/lib/asterisk/agi-bin/check-supervisor
sudo chmod +x /var/lib/asterisk/agi-bin/check-supervisor
```

### 3. เรียกใช้ใน Dialplan

เพิ่มใน `/etc/asterisk/extensions.conf`:

```
[your-context]
exten => _X.,1,AGI(check-supervisor,${EXTEN})
exten => _X.,n,GotoIf($["${RET}" = "1"]?agent-available:agent-unavailable)
exten => _X.,n(agent-available),Verbose(1,Agent ${EXTEN} is available)
exten => _X.,n,Hangup()
exten => _X.,n(agent-unavailable),Verbose(1,Agent ${EXTEN} is not available)
exten => _X.,n,Hangup()
```

## วิธีการทำงาน

โปรแกรมจะ:

1. **รับ argument แรก** (extension number)
   - ถ้าไม่มี argument จะ set `RET=0` และ exit code 1

2. **ตรวจสอบความยาวของ extension:**
   - **≤ 4 ตัวอักษร**: ตรวจสอบฐานข้อมูล MySQL
   - **> 4 ตัวอักษร**: set `RET=1` ทันที

3. **สำหรับ extension ≤ 4 ตัวอักษร:**
   - เชื่อมต่อฐานข้อมูล MySQL: `10.133.1.13`
   - ใช้ user: `dcall`, password: `dcallpass`, database: `dcall`
   - รัน SQL: `SELECT count(*) FROM agents WHERE id = '<extension>' AND type > '0'`
   - ถ้า count > 0: set `RET=1`
   - ถ้า count = 0: set `RET=0`

## ข้อกำหนดระบบ

- **Rust**: สำหรับ compile (cargo build)
- **MySQL Client**: โปรแกรมใช้ `mysql` command line client ในการเชื่อมต่อฐานข้อมูล
- **Asterisk**: เวอร์ชันที่รองรับ AGI

### ติดตั้ง MySQL Client (Ubuntu/Debian):
```bash
sudo apt-get install mysql-client
```

### ติดตั้ง MySQL Client (CentOS/RHEL):
```bash
sudo yum install mysql
# หรือ
sudo dnf install mysql
```

## ตัวอย่างการใช้งาน

### ใน AGI:
```bash
# ตรวจสอบ agent 1234
/var/lib/asterisk/agi-bin/check-supervisor 1234

# ตัวแปร RET จะถูก set เป็น:
# - "1" = agent พร้อมใช้งาน (type > 0 ในฐานข้อมูล)  
# - "0" = agent ไม่พร้อมใช้งาน หรือไม่พบในฐานข้อมูล
```

### ทดสอบจาก command line:
```bash
# ต้องมี AGI environment - ใช้ echo เปล่าเพื่อจำลอง
echo "" | /var/lib/asterisk/agi-bin/check-supervisor 1234
```

## Troubleshooting

### ถ้า MySQL connection ล้มเหลว:
- ตรวจสอบว่าติดตั้ง `mysql` client แล้ว
- ตรวจสอบ network connectivity ไปยัง `10.133.1.13`
- ตรวจสอบ username/password และ database permissions

### ถ้า AGI script ไม่ทำงาน:
- ตรวจสอบ file permissions (`chmod +x`)
- ตรวจสอบ owner (`chown asterisk:asterisk`)
- ดู Asterisk logs: `/var/log/asterisk/full`

## Security Notes

- โปรแกรมมีการตรวจสอบ input เบื้องต้น (อนุญาตเฉพาะตัวเลขสำหรับ extension ≤ 4 ตัว)
- ใช้ parameterized query เพื่อป้องกัน SQL injection
- Database credentials อยู่ใน source code - ควรพิจารณาใช้ environment variables ใน production