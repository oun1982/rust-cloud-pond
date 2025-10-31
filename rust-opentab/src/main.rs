use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// เทียบเท่ากับ dlgs
use native_dialog::{MessageDialog, MessageType};

// เทียบเท่ากับ robotgo
use enigo::{Enigo, Key, Keyboard, Settings};

fn main() {
    // 1. รับ home directory ของผู้ใช้ปัจจุบัน
    // dirs::home_dir() จะคืนค่าเป็น Option<PathBuf>
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            log_error("ไม่สามารถหา Home Directory ของผู้ใช้ได้");
            return;
        }
    };

    // 2. กำหนดเส้นทางที่เป็นไปได้สำหรับ chrome.exe
    // PathBuf เป็นวิธีที่ปลอดภัยในการสร้าง path (เทียบเท่า filepath.Join)
    // r"..." คือ raw string ใน Rust, ป้องกันปัญหา backslash '\' บน Windows
    let paths_to_check = [
        PathBuf::from(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
        PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
        home_dir
            .join("AppData")
            .join("Local")
            .join("Google")
            .join("Chrome")
            .join("Application")
            .join("chrome.exe"),
    ];

    // 3. ค้นหาไฟล์ chrome.exe ที่ถูกต้อง
    let mut chrome_path: Option<PathBuf> = None;
    for path in &paths_to_check {
        // path.exists() เทียบเท่ากับการตรวจสอบ os.Stat(path) == nil
        if path.exists() {
            chrome_path = Some(path.clone()); // .clone() เพื่อสร้างสำเนา
            break; // หยุด loop เมื่อเจอไฟล์
        }
    }

    // 4. ตรวจสอบว่าเราหา Chrome เจอหรือไม่
    let found_chrome_path = match chrome_path {
        Some(path) => path,
        None => {
            // หากไม่พบ ให้แสดงกล่องข้อความแจ้งเตือนและจบการทำงาน
            MessageDialog::new()
                .set_title("!!!!! Error")
                .set_text("Not Fouund Google Chrome")
                .set_type(MessageType::Error)
                .show_alert()
                .expect("ไม่สามารถแสดงกล่องข้อความ Error ได้");
            return; // จบการทำงาน
        }
    };

    println!("Found Chrome @ : {:?}", found_chrome_path);

    let urls = [
        "https://dialer.maxgroup",
        "https://pbx.maxgroup:8089/ws",
    ];

    // 5. เปิด URL ทั้งหมดในแท็บใหม่
    for (i, url) in urls.iter().enumerate() {
        // ใช้ std::process::Command (เทียบเท่า exec.Command)
        let mut cmd = Command::new(&found_chrome_path);
        cmd.arg("--new-tab")
            .arg("--ignore-certificate-errors")
            .arg(url);

        // .spawn() เทียบเท่ากับ .Start() (รันแบบไม่ blocking)
        // .stderr(Stdio::null()) เพื่อซ่อน error output จาก chrome (ถ้ามี)
        if let Err(e) = cmd.stderr(Stdio::null()).spawn() {
            // ใช้ eprintln! เพื่อพิมพ์ไปยัง stderr
            eprintln!("Can't open URL {}: {:?}", url, e);
        }

        // รอสักครู่ระหว่างการเปิด URL แรกและ URL ที่สอง
        if i == 0 {
            // thread::sleep เทียบเท่า time.Sleep
            thread::sleep(Duration::from_secs(1));
        }
    }

    // 6. รอ 2 วินาทีเพื่อให้แท็บทั้งหมดโหลด
    thread::sleep(Duration::from_secs(2));

    // 7. จำลองการกด Ctrl+W
    println!("Close Tab 2...");
    
    // สร้าง instance ของ Enigo
    let mut enigo = Enigo::new(&Settings::default()).expect("ไม่สามารถสร้าง Enigo instance ได้");

    // การจำลอง KeyTap("w", "ctrl") ใน enigo จะต้องทำทีละขั้นตอน:
    enigo.key(Key::Control, enigo::Direction::Press).unwrap(); // 1. กดปุ่ม Control ค้างไว้
    enigo.key(Key::W, enigo::Direction::Click).unwrap();     // 2. กดปุ่ม W (Click = กดแล้วปล่อย)
    enigo.key(Key::Control, enigo::Direction::Release).unwrap(); // 3. ปล่อยปุ่ม Control

    thread::sleep(Duration::from_secs(1));

    // (ส่วนนี้ถูกคอมเมนต์ไว้ในโค้ด Go)
    // println!("กำลังปิดแท็บที่ 3...");
    // enigo.key(Key::Control, enigo::Direction::Press).unwrap();
    // enigo.key(Key::W, enigo::Direction::Click).unwrap();
    // enigo.key(Key::Control, enigo::Direction::Release).unwrap();

    println!("Finish");
}

// ฟังก์ชันเสริม: ตรวจสอบว่า Chrome กำลังทำงานอยู่หรือไม่ (ไม่ได้ถูกเรียกใช้ใน main)
// เทียบเท่ากับฟังก์ชัน isChromeRunning()
#[allow(dead_code)] // บอก Rust compiler ว่าไม่ต้องเตือนถ้าฟังก์ชันนี้ไม่ได้ถูกใช้
fn is_chrome_running() -> Result<bool, std::io::Error> {
    // .output() จะรันคำสั่งและรอจนจบ (เหมือน .Output() ใน Go)
    let output = Command::new("tasklist")
        .stdout(Stdio::piped()) // เราต้องการจับ stdout
        .stderr(Stdio::null()) // ไม่สนใจ stderr
        .output()?; // '?' คือ error propagation (คล้ายๆ if err != nil)

    if output.status.success() {
        // แปลงผลลัพธ์ (bytes) ให้อยู่ในรูป String
        // .from_utf8_lossy จะแปลงส่วนที่อ่านไม่ออกเป็นสัญลักษณ์แทน
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("chrome.exe"))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to run tasklist",
        ))
    }
}

// ฟังก์ชันเสริมสำหรับแสดง error (ใน Rust ไม่มี log.Fatalf)
fn log_error(message: &str) {
    eprintln!("{}", message); // พิมพ์ไปที่ Standard Error
    MessageDialog::new()
        .set_title("Fatal Error")
        .set_text(message)
        .set_type(MessageType::Error)
        .show_alert()
        .ok(); // .ok() เพื่อละเว้นผลลัพธ์
}