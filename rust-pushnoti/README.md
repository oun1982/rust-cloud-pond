# Rust Push Notification Tool

โปรแกรมนี้ใช้สำหรับส่ง push notifications ไปยัง agents ใน queue ที่กำหนด

## การติดตั้งและใช้งาน

### 1. ติดตั้ง dependencies
```bash
cargo build
```

### 2. ตั้งค่า database connection
คัดลอกไฟล์ `.env.example` เป็น `.env` และแก้ไขค่าการเชื่อมต่อ database:
```bash
cp .env.example .env
```

แก้ไขไฟล์ `.env`:
```
DATABASE_URL=mysql://username:password@hostname:port/dcall
```

### 3. การใช้งาน
```bash
# ส่ง push notification ไปยัง agents ใน queue 10001
cargo run -- --queueid 10001

# หรือใช้ short form
cargo run -- -q 10001
```

### 4. การใช้งาน (binary ที่ build แล้ว)
```bash
# Build binary
cargo build --release

# Run binary
./target/release/rust-pushnoti --queueid 10001
```

## Environment Variables

- `DATABASE_URL`: Connection string สำหรับ MySQL database (ถ้าไม่ได้ตั้งค่าจะใช้ค่า default)

## ตัวอย่างการทำงาน

เมื่อรันคำสั่ง:
```bash
cargo run -- --queueid 10001
```

โปรแกรมจะ:
1. เชื่อมต่อกับ database
2. Query หา agentid ทั้งหมดที่อยู่ใน queueid 10001
3. Loop ส่ง HTTP GET request ไปยัง URL:
   ```
   https://us-central1-softphone-dcallcenter.cloudfunctions.net/sendPush?ext={agentid}&server=pbx-backoffice.osd.co.th
   ```
   สำหรับแต่ละ agentid ที่พบ

## โครงสร้าง Database

โปรแกรมนี้คาดหวังให้มี table `agentqueue` ที่มี columns:
- `queueid`: ID ของ queue
- `agentid`: ID ของ agent

```sql
SELECT agentid FROM agentqueue WHERE queueid = ?
```