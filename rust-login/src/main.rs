use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::io::AsRawFd;
use std::process::exit;
use std::time::Duration;

// --- โครงสร้างสำหรับ XML Parsing (เทียบเท่า Go structs) ---

/// เทียบเท่า DCallResponse struct
#[derive(Debug, Deserialize)]
#[serde(rename = "dcall")]
struct DCallResponse {
    // ใช้ Option<Agent> เพราะ <agent> อาจจะไม่มีอยู่ (เหมือน pointer ใน Go)
    #[serde(default)]
    agent: Option<Agent>,
}

/// เทียบเท่า Agent struct
#[derive(Debug, Deserialize)]
struct Agent {
    // ใช้ `@[attr_name]` เพื่อระบุว่าเป็น attribute ของ XML
    #[serde(rename = "@agentid", default)]
    agentid: String,
    #[serde(rename = "@name", default)]
    name: String,
}

// --- Error Type ที่กำหนดเอง (สำหรับความสะดวก) ---
#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for AppError {}

// --- ฟังก์ชันหลักในการส่งข้อมูล (เทียบเท่า sendDCall) ---
fn send_dcall(
    host: &str,
    command: &str,
    params: &[String],
) -> Result<String, Box<dyn Error>> {
    let addr = format!("{}:10080", host);
    let stream_result = TcpStream::connect_timeout(
        &addr.parse()?, 
        Duration::from_secs(5)
    );

    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => return Err(format!("dial error: {}", e).into()),
    };

    // สร้าง XML payload
    let payload = match command {
        "login" | "logout" => {
            if params.len() < 3 {
                return Err("login/logout requires 3 parameters".into());
            }
            format!(
                "<dcall command='{}'><data u='{}' p='{}' s='{}' ssid='' v='1.0.0.0' /></dcall>",
                command, params[0], params[1], params[2]
            )
        }
        "pause" | "unpause" => {
            if params.len() < 2 {
                return Err("pause/unpause requires at least 2 parameters".into());
            }
            // ใช้ .get() เพื่อดึงค่า params ที่อาจจะไม่มีอยู่ (เหมือน check len > N ใน Go)
            let id = params.get(2).cloned().unwrap_or_default();
            let txt = params.get(3).cloned().unwrap_or_default();
            format!(
                "<dcall command='{}'><data u='{}' s='{}' id='{}' txt='{}'/></dcall>",
                command, params[0], params[1], id, txt
            )
        }
        _ => return Err(format!("unknown command: {}", command).into()),
    };

    // Server ต้องการ CRLF (\r\n)
    println!("Sending: {}", payload);
    let payload_with_crlf = payload + "\r\n";

    // Set read deadline
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    // Send payload
    if let Err(e) = stream.write_all(payload_with_crlf.as_bytes()) {
        return Err(format!("write error: {}", e).into());
    }

    // Read response until </dcall>
    let mut response_bytes = Vec::new();
    let mut buffer = [0; 1024];
    let closing_tag = b"</dcall>";

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // EOF (Connection closed) - ตรวจสอบว่าได้ response หรือไม่
                if response_bytes.is_empty() {
                    return Err("Connection closed without response (login failed?)".into());
                }
                break;
            }
            Ok(n) => {
                response_bytes.extend_from_slice(&buffer[..n]);
                // ตรวจสอบว่ามี closing tag ใน buffer ที่เรารวบรวมมาหรือยัง
                if response_bytes
                    .windows(closing_tag.len())
                    .any(|window| window == closing_tag)
                {
                    break;
                }
            }
            Err(e) => {
                // ถ้าเป็น read timeout หรือ error อื่นๆ
                // ถ้ามี response บางส่วนแล้ว ให้ return ไป (อาจเป็น connection closed)
                if !response_bytes.is_empty() {
                    break;
                }
                return Err(format!("read error: {}", e).into());
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response_bytes).to_string();
    
    // ตรวจสอบว่า response มี </dcall> tag หรือไม่
    // ถ้าไม่มี แสดงว่า server ปิด connection ก่อน (login failed)
    if !response_str.contains("</dcall>") {
        return Err(format!("Incomplete response (server closed connection): {}", response_str).into());
    }

    Ok(response_str)
}

// --- ฟังก์ชันสำหรับอ่าน AGI environment และตรวจสอบว่าเป็น AGI mode ---
fn read_and_detect_agi() -> Result<bool, Box<dyn Error>> {
    use std::io::{stdin, BufRead};
    use std::os::unix::io::AsRawFd;
    
    // ตรวจสอบว่า stdin มีข้อมูลพร้อมอ่านหรือไม่โดยใช้ poll
    let stdin_fd = stdin().as_raw_fd();
    let mut poll_fds = [libc::pollfd {
        fd: stdin_fd,
        events: libc::POLLIN,
        revents: 0,
    }];
    
    // Poll timeout 100ms - ถ้าไม่มีข้อมูลภายใน 100ms = ไม่ใช่ AGI mode
    let poll_result = unsafe { libc::poll(poll_fds.as_mut_ptr(), 1, 100) };
    
    if poll_result <= 0 {
        // Timeout หรือ error = ไม่ใช่ AGI mode
        return Ok(false);
    }
    
    // มีข้อมูลใน stdin, ลองอ่าน
    let stdin_handle = stdin();
    let mut reader = BufReader::new(stdin_handle);
    let mut first_line = String::new();
    
    reader.read_line(&mut first_line)?;
    
    // ตรวจสอบว่าเป็น AGI environment หรือไม่
    if first_line.starts_with("agi_") {
        // เป็น AGI mode! อ่านบรรทัดที่เหลือจนเจอบรรทัดว่าง
        loop {
            let mut line = String::new();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 || line.trim().is_empty() {
                break;
            }
        }
        return Ok(true);
    }
    
    // ไม่ใช่ AGI format = ไม่ใช่ AGI mode
    Ok(false)
}

// --- ฟังก์ชันส่ง AGI command ---
fn send_agi_command(command: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", command);
    io::stdout().flush()?;
    
    // รอ response จาก Asterisk (อ่านทิ้ง)
    let mut response = String::new();
    let _ = io::stdin().read_line(&mut response);
    
    Ok(())
}
// --- ฟังก์ชันแยกวิเคราะห์ XML (เทียบเท่า parseResponse) ---
fn parse_response(response_xml: &str, agi_mode: bool) -> (String, String, bool) {
    // Debug: แสดง response ที่ได้รับ (เฉพาะโหมดไม่ใช่ AGI)
    if !agi_mode {
        println!("DEBUG: Received XML: {}", response_xml);
    }
    
    // ตรวจสอบว่า response มี <agent> tag หรือไม่
    if !response_xml.contains("<agent") {
        if !agi_mode {
            println!("DEBUG: No <agent> tag found in response");
        }
        return (String::new(), String::new(), false);
    }
    
    match quick_xml::de::from_str::<DCallResponse>(response_xml) {
        Ok(dcall) => {
            // ตรวจสอบว่ามี <agent> และ agentid ไม่ใช่ค่าว่าง
            if let Some(agent) = dcall.agent {
                if !agent.agentid.is_empty() {
                    if !agi_mode {
                        println!("DEBUG: Found agent - ID: {}, Name: {}", agent.agentid, agent.name);
                    }
                    
                    // ตรวจสอบว่า agentid เป็น -1 หรือไม่ (หมายถึง login failed)
                    if agent.agentid == "-1" {
                        if !agi_mode {
                            println!("DEBUG: Login failed - agentid is -1");
                        }
                        return (String::new(), String::new(), false);
                    }
                    
                    // ตรวจสอบว่า agentid เป็นตัวเลขที่มากกว่า 0 หรือไม่
                    if let Ok(id_num) = agent.agentid.parse::<i32>() {
                        if id_num <= 0 {
                            if !agi_mode {
                                println!("DEBUG: Login failed - agentid is {} (not positive)", id_num);
                            }
                            return (String::new(), String::new(), false);
                        }
                    }
                    
                    return (agent.agentid, agent.name, true);
                } else {
                    if !agi_mode {
                        println!("DEBUG: Agent tag exists but agentid is empty");
                    }
                }
            } else {
                if !agi_mode {
                    println!("DEBUG: Agent option is None");
                }
            }
            // ไม่มี <agent> หรือ agentid ว่างเปล่า
            (String::new(), String::new(), false)
        }
        Err(e) => {
            if !agi_mode {
                println!("DEBUG: XML Parse Error: {:?}", e);
            }
            (String::new(), String::new(), false)
        }
    }
}

// --- ฟังก์ชันที่จัดการตรรกะหลักและ exit code (คล้าย main ใน Go) ---
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // ตรวจสอบว่าทำงานใน AGI mode หรือไม่ โดยพยายามอ่าน AGI environment
    let agi_mode = read_and_detect_agi()?;

    if args.len() < 3 {
        if !agi_mode {
            println!("Usage:");
            println!("  Login/Logout: rust-login <host> <command> <extension> <username> <password>");
            println!("  Pause/Unpause: rust-login <host> <command> <extension> <s> [id] [txt]");
            println!("");
            println!("Examples:");
            println!("  rust-login 10.133.1.11 login 4001 agent01 password123");
            println!("  rust-login 10.133.1.11 logout 4001 agent01 password123");
        } else {
            let _ = send_agi_command("SET VARIABLE RET \"FAILED\"");
        }
        exit(1);
    }

    let host = &args[1];
    let command = &args[2];
    // params คือ slice ที่เหลือทั้งหมด
    let params = &args[3..];

    // เรียก send_dcall และใช้ ? เพื่อจัดการ error (ถ้าเกิด Err จะ return ทันที)
    let resp = match send_dcall(host, command, params) {
        Ok(response) => response,
        Err(e) => {
            if agi_mode {
                let _ = send_agi_command("SET VARIABLE RET \"FAILED\"");
            } else {
                println!("Error: {}", e);
            }
            exit(2);
        }
    };

    if !agi_mode {
        println!("Response: {}", resp);
    }

    // Parse XML response
    let (agent_id, name, ok) = parse_response(&resp, agi_mode);
    if ok {
        if agi_mode {
            // ส่ง AGI command เพื่อตั้งค่า RET เป็น OK
            let _ = send_agi_command("SET VARIABLE RET \"OK\"");
        } else {
            println!("SUCCESS: AgentID={}, Name={}", agent_id, name);
        }
        exit(0);
    } else {
        if agi_mode {
            let _ = send_agi_command("SET VARIABLE RET \"FAILED\"");
        } else {
            println!("FAILED: No valid agent response");
        }
        exit(3);
    }
}

// --- main ของ Rust ---
fn main() {
    // `run()` จะ return Result
    // ถ้าเป็น Ok(()) โปรแกรมจบปกติ
    // ถ้าเป็น Err(e) เราจะ print error และ exit(2)
    if let Err(e) = run() {
        println!("Error: {}", e);
        exit(2);
    }
}
