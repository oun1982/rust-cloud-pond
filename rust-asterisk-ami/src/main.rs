use asterisk_ami_dep::{AmiConnection, Tag, find_tag};
use chrono::{DateTime, Local};
use std::collections::HashMap;
use tokio::{io::Result, signal};
use tokio::fs::{OpenOptions, create_dir_all};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
struct CallInfo {
    // ----- คอลัมน์ตาม SQL ตัวอย่าง -----
    systemid: i32,
    uniqueid: String,
    seq: i32,
    agentid: String,
    calltype: String,                 // 'O' | 'I'
    channel: String,
    channelstate: String,             // เช่น "4"
    channelstatedesc: String,         // เช่น "Ring"
    connectedline: String,
    context: String,
    exten: String,
    priority: i32,
    destchannel: String,
    destchannelstate: String,         // เก็บเป็น string แล้ว cast เป็นเลขตอนพิมพ์
    destchannelstatedesc: String,
    src: String,
    dst: String,
    destcontext: String,
    destexten: String,
    destpriority: i32,
    destuniqueid: String,
    dialstatus: String,               // ANSWER/BUSY/NOANSWER/...
    hanguprequest: String,
    hangupcause: Option<String>,      // NULL ได้
    transfer: String,
    transferexten: String,
    ringtime: i32,
    talktime: i32,
    holdtime: i32,
    ringdate: String,
    connectdate: String,
    completedate: String,
    updatedate: String,
    star: i32,
    lock1: i32,
    bill: i32,

    // ----- ฟิลด์ช่วยคำนวณ -----
    ring_epoch: i64,
    connect_epoch: Option<i64>,
}

impl CallInfo {
    fn new_from_dialbegin(packet: &Vec<Tag>) -> Self {
        let uniqueid = get(packet, "Uniqueid");
        let src = get(packet, "CallerIDNum");
        let dst = get(packet, "DestCallerIDNum");          // ให้ตรงตัวอย่าง: ปลายทาง
        let channel = get(packet, "Channel");
        let context = get(packet, "Context");
        let exten = get(packet, "Exten");
        let priority = parse_i32(&get(packet, "Priority"));
        let channelstate = get(packet, "ChannelState");     // เช่น "4"
        let channelstatedesc = get(packet, "ChannelStateDesc");

        let destchannel = get(packet, "DestChannel");
        let destchannelstate = get(packet, "DestChannelState");         // เช่น "0" ตอน DialBegin
        let destchannelstatedesc = get(packet, "DestChannelStateDesc"); // เช่น "Down"
        let destcontext = get(packet, "DestContext");
        let destexten = get(packet, "DestExten");
        let destpriority = parse_i32(&get(packet, "DestPriority"));
        let destuniqueid = get(packet, "DestUniqueid");

        let now = Local::now();
        let ringdate = fmt_ts(now);
        let ring_epoch = now.timestamp();

        // ตัดสินใจ calltype: O (outbound) / I (inbound)
        let calltype = infer_calltype(&context, &destchannel);

        // agentid: outbound ใช้ผู้โทร (src), inbound ใช้ปลายทาง (dst) ให้สอดคล้องตัวอย่าง
        let agentid = if calltype == "O" { src.clone() } else { dst.clone() };

        CallInfo {
            systemid: 1,
            uniqueid,
            seq: 1,
            agentid,
            calltype,
            channel,
            channelstate,
            channelstatedesc,
            connectedline: dst.clone(),
            context,
            exten,
            priority,
            destchannel,
            destchannelstate,
            destchannelstatedesc,
            src,
            dst,
            destcontext,
            destexten,
            destpriority,
            destuniqueid,
            dialstatus: String::new(),
            hanguprequest: String::new(),
            hangupcause: None,
            transfer: String::new(),
            transferexten: String::new(),
            ringtime: 0,
            talktime: 0,
            holdtime: 0,
            ringdate: ringdate.clone(),
            connectdate: String::new(),
            completedate: String::new(),
            updatedate: ringdate,
            star: 0,
            lock1: 0,
            bill: 0,
            ring_epoch,
            connect_epoch: None,
        }
    }

    fn apply_dialend(&mut self, packet: &Vec<Tag>) {
        self.dialstatus = get(packet, "DialStatus"); // ANSWER/BUSY/NOANSWER...

        let deststate = get(packet, "DestChannelState");
        let destdesc  = get(packet, "DestChannelStateDesc");

        if self.dialstatus.eq_ignore_ascii_case("ANSWER") {
            // ให้ตรงตัวอย่าง ANSWER ⇒ Up(6)
            self.destchannelstate = "6".to_string();
            self.destchannelstatedesc = "Up".to_string();

            let now = Local::now();
            self.connectdate = fmt_ts(now);
            self.connect_epoch = Some(now.timestamp());

            // ringtime = connect - ring
            self.ringtime = (self.connect_epoch.unwrap_or(self.ring_epoch) - self.ring_epoch)
                .max(0) as i32;
        } else {
            // ไม่ ANSWER → เก็บค่าจาก DialEnd ตามจริง (เช่น Ringing 5)
            if !deststate.is_empty() {
                self.destchannelstate = deststate;
            }
            if !destdesc.is_empty() {
                self.destchannelstatedesc = destdesc;
            }
        }
    }

    fn apply_hangup(&mut self, packet: &Vec<Tag>) {
        // Cause-txt เช่น "User busy" → อาจไม่มี
        let cause_txt = get(packet, "Cause-txt");
        self.hangupcause = if cause_txt.is_empty() { None } else { Some(cause_txt) };

        // ให้สอดคล้องตัวอย่าง: ใช้ปลายทางเป็นผู้วาง (อ่านง่ายและคงที่)
        self.hanguprequest = self.dst.clone();

        // Completed & Updated
        let now = Local::now();
        self.completedate = fmt_ts(now);
        self.updatedate = self.completedate.clone();

        // talktime = completed - connect (ถ้ามีการรับสาย)
        if let Some(conn_epoch) = self.connect_epoch {
            self.talktime = (now.timestamp() - conn_epoch).max(0) as i32;
        } else {
            self.talktime = 0;
        }

        // ถ้าไม่เคย connect ⇒ ringtime = completed - ring
        if self.connect_epoch.is_none() {
            self.ringtime = (now.timestamp() - self.ring_epoch).max(0) as i32;
        }
    }

    fn to_sql(&self) -> String {
        // hangupcause อาจเป็น NULL
        let hangupcause_sql = match &self.hangupcause {
            Some(s) if !s.is_empty() => format!("'{}'", escape_sql(s)),
            _ => "NULL".to_string(),
        };

        // ใช้ named placeholders กันเคลื่อน
        format!(
            "INSERT INTO `dialtraffic` (`systemid`, `uniqueid`, `seq`, `agentid`, `calltype`, \
             `channel`, `channelstate`, `channelstatedesc`, `connectedline`, `context`, \
             `exten`, `priority`, `destchannel`, `destchannelstate`, `destchannelstatedesc`, \
             `src`, `dst`, `destcontext`, `destexten`, `destpriority`, `destuniqueid`, \
             `dialstatus`, `hanguprequest`, `hangupcause`, `transfer`, `transferexten`, \
             `ringtime`, `talktime`, `holdtime`, `ringdate`, `connectdate`, `completedate`, \
             `updatedate`, `star`, `lock1`, `bill`) VALUES \
             ({systemid}, '{uniqueid}', {seq}, '{agentid}', '{calltype}', '{channel}', '{channelstate}', '{channelstatedesc}', \
              '{connectedline}', '{context}', '{exten}', {priority}, '{destchannel}', {destchannelstate}, '{destchannelstatedesc}', \
              '{src}', '{dst}', '{destcontext}', '{destexten}', {destpriority}, '{destuniqueid}', '{dialstatus}', '{hanguprequest}', \
              {hangupcause}, '{transfer}', '{transferexten}', {ringtime}, {talktime}, {holdtime}, '{ringdate}', '{connectdate}', \
              '{completedate}', '{updatedate}', {star}, {lock1}, {bill});",
            systemid = self.systemid,
            uniqueid = escape_sql(&self.uniqueid),
            seq = self.seq,
            agentid = escape_sql(&self.agentid),
            calltype = self.calltype,
            channel = escape_sql(&self.channel),
            channelstate = escape_sql(&self.channelstate),
            channelstatedesc = escape_sql(&self.channelstatedesc),
            connectedline = escape_sql(&self.connectedline),
            context = escape_sql(&self.context),
            exten = escape_sql(&self.exten),
            priority = self.priority,
            destchannel = escape_sql(&self.destchannel),
            destchannelstate = parse_i32(&self.destchannelstate),
            destchannelstatedesc = escape_sql(&self.destchannelstatedesc),
            src = escape_sql(&self.src),
            dst = escape_sql(&self.dst),
            destcontext = escape_sql(&self.destcontext),
            destexten = escape_sql(&self.destexten),
            destpriority = self.destpriority,
            destuniqueid = escape_sql(&self.destuniqueid),
            dialstatus = escape_sql(&self.dialstatus),
            hanguprequest = escape_sql(&self.hanguprequest),
            hangupcause = hangupcause_sql, // (NULL หรือ 'text') — ไม่ใส่ quoteเพิ่ม
            transfer = escape_sql(&self.transfer),
            transferexten = escape_sql(&self.transferexten),
            ringtime = self.ringtime,
            talktime = self.talktime,
            holdtime = self.holdtime,
            ringdate = self.ringdate,
            connectdate = self.connectdate,
            completedate = self.completedate,
            updatedate = self.updatedate,
            star = self.star,
            lock1 = self.lock1,
            bill = self.bill,
        )
    }
}

// ---------- Utilities ----------
fn get(packet: &Vec<Tag>, key: &str) -> String {
    find_tag(packet, key).map(|s| s.to_string()).unwrap_or_default()
}

fn parse_i32(s: &str) -> i32 {
    s.trim().parse::<i32>().unwrap_or(0)
}

fn fmt_ts(dt: DateTime<Local>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn escape_sql(input: &str) -> String {
    input.replace('\'', "''")
}

// แตกไฟล์ตามวัน: logs/calllog_YYYYMMDD.sql
const LOG_DIR: &str = "logs";
fn daily_log_path() -> String {
    let today = Local::now().format("%Y%m%d").to_string();
    format!("{}/calllog_{}.sql", LOG_DIR, today)
}

// เขียนไฟล์แบบ append (สร้างโฟลเดอร์ให้ด้วยถ้ายังไม่มี)
async fn append_line(path: &str, line: &str) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        create_dir_all(parent).await.ok();
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path).await?;
    f.write_all(line.as_bytes()).await?;
    f.write_all(b"\n").await?;
    f.flush().await?;
    Ok(())
}

// ตัดสินใจ I/O จาก context/destchannel
fn infer_calltype(context: &str, destchannel: &str) -> String {
    let c = context.to_lowercase();
    if c.contains("dialout") || destchannel.contains("OUT-") {
        "O".to_string()
    } else if c.contains("from-trunk") || c.contains("macro-dial-one") {
        "I".to_string()
    } else if destchannel.to_lowercase().contains("out-") {
        "O".to_string()
    } else {
        "I".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let host = "192.168.60.1:5038";
    let username = "ami";
    let password = "password";

    let conn = AmiConnection::connect(host).await
        .expect("Failed to connect to AMI");

    conn.send(vec![
        Tag::from("Action", "Login"),
        Tag::from("Username", username),
        Tag::from("Secret", password),
    ]).await.expect("Login failed");

    println!("✅ Connected to Asterisk AMI!");
    println!("⏳ Waiting for events... (Press Ctrl+C to exit)");
    println!("📝 SQL output (daily): {}", daily_log_path());

    let mut rx = conn.events();
    let mut calls: HashMap<String, CallInfo> = HashMap::new();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(Some(packet)) => {
                        if let Some(ev) = find_tag(&packet, "Event") {
                            match ev.as_str() {
                                "DialBegin" => {
                                    let uniqueid = get(&packet, "Uniqueid");
                                    let call = CallInfo::new_from_dialbegin(&packet);
                                    calls.insert(uniqueid, call);
                                }
                                "DialEnd" => {
                                    let uniqueid = get(&packet, "Uniqueid");
                                    if let Some(call) = calls.get_mut(&uniqueid) {
                                        call.apply_dialend(&packet);
                                    }
                                }
                                "Hangup" => {
                                    let uniqueid = get(&packet, "Uniqueid");
                                    if let Some(mut call) = calls.remove(&uniqueid) {
                                        call.apply_hangup(&packet);

                                        let path = daily_log_path();
                                        let line = call.to_sql();
                                        if let Err(e) = append_line(&path, &line).await {
                                            eprintln!("❌ Failed to write SQL: {}", e);
                                        } else {
                                            println!("✅ Appended SQL for call {} → {}", uniqueid, path);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(None) => {
                        eprintln!("⚠️ Disconnected from AMI.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("❌ Error receiving event: {:?}", e);
                        break;
                    }
                }
            }
            _ = signal::ctrl_c() => {
                println!("\n👋 Ctrl+C pressed. Exiting...");
                break;
            }
        }
    }

    Ok(())
}
