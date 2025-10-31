use asterisk_agi::*;
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher, Event};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// โครงสร้างข้อมูล IVR config สำหรับแต่ละ DID
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IvrConfig {
    welcome_sound: String,
    invalid_sound: String,
    goodbye_sound: String,
    queues: HashMap<String, String>,
    min_extension_digits: usize,
    max_extension_digits: usize,
    extension_timeout_seconds: u64,
    dial_timeout_seconds: u64,
    dial_options: String,
}

/// โครงสร้างข้อมูล config file ทั้งหมด
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigFile {
    default: IvrConfig,
    dids: HashMap<String, IvrConfig>,
}

/// Global config ที่ใช้ร่วมกันทั้ง app (thread-safe)
static GLOBAL_CONFIG: Lazy<Arc<RwLock<ConfigFile>>> = Lazy::new(|| {
    Arc::new(RwLock::new(load_config().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using default.", e);
        create_default_config()
    })))
});

/// อ่าน config file
fn load_config() -> Result<ConfigFile, Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let content = fs::read_to_string(&config_path)?;
    let config: ConfigFile = serde_yaml::from_str(&content)?;
    eprintln!("✓ Config loaded from: {}", config_path);
    Ok(config)
}

/// หา config path (ตรวจสอบหลาย path)
fn get_config_path() -> String {
    let paths = vec![
        "/var/lib/asterisk/agi-bin/rust-agi/config.yaml",
        "/var/lib/asterisk/agi-bin/rust-agi/ivr-config.yaml",
        "/var/lib/asterisk/agi-bin/config.yaml",
        "/var/lib/asterisk/agi-bin/ivr-config.yaml",
        "/etc/asterisk/ivr-config.yaml",
        "/usr/local/etc/asterisk/ivr-config.yaml",
        "./config.yaml",
        "/opt/rust-project/rust-dialplan/config.yaml",
    ];
    
    for path in paths {
        if Path::new(path).exists() {
            return path.to_string();
        }
    }
    
    // ถ้าไม่เจอใช้ default
    "/var/lib/asterisk/agi-bin/rust-agi/config.yaml".to_string()
}

/// สร้าง default config (กรณีไม่พบไฟล์)
fn create_default_config() -> ConfigFile {
    let mut default_queues = HashMap::new();
    for i in 1..=9 {
        default_queues.insert(i.to_string(), format!("1000{}", i));
    }

    ConfigFile {
        default: IvrConfig {
            welcome_sound: "en/custom/new-ivr-osd".to_string(),
            invalid_sound: "invalid".to_string(),
            goodbye_sound: "vm-goodbye".to_string(),
            queues: default_queues,
            min_extension_digits: 3,
            max_extension_digits: 4,
            extension_timeout_seconds: 3,
            dial_timeout_seconds: 60,
            dial_options: "t".to_string(),
        },
        dids: HashMap::new(),
    }
}

/// Reload config (hot reload)
fn reload_config() -> Result<(), Box<dyn std::error::Error>> {
    let new_config = load_config()?;
    let mut config = GLOBAL_CONFIG.write().unwrap();
    *config = new_config;
    eprintln!("✓ Config reloaded successfully!");
    Ok(())
}

/// ดึง IVR config ตาม DID
fn get_ivr_config(did: &str) -> IvrConfig {
    let config = GLOBAL_CONFIG.read().unwrap();
    config.dids.get(did)
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("DID '{}' not found in config, using default", did);
            config.default.clone()
        })
}

/// เริ่มต้น file watcher สำหรับ hot reload
fn start_config_watcher() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    
    if !Path::new(&config_path).exists() {
        eprintln!("Warning: Config file not found: {}", config_path);
        return Ok(());
    }

    // Clone config_path สำหรับใช้ใน thread
    let path_for_watcher = config_path.clone();
    let path_for_display = config_path.clone();

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(_event) = res {
                    let _ = tx.send(());
                }
            },
            NotifyConfig::default(),
        ).expect("Failed to create watcher");

        let watch_path = Path::new(&path_for_watcher);
        watcher.watch(watch_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch config file");

        eprintln!("✓ Config watcher started for: {}", path_for_display);

        loop {
            if rx.recv().is_ok() {
                // รอสักครู่เพื่อให้แน่ใจว่าไฟล์ถูกเขียนเสร็จ
                std::thread::sleep(Duration::from_millis(100));
                
                if let Err(e) = reload_config() {
                    eprintln!("✗ Failed to reload config: {}", e);
                }
            }
        }
    });

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // เริ่ม config watcher สำหรับ hot reload (run once)
    static WATCHER_STARTED: once_cell::sync::OnceCell<()> = once_cell::sync::OnceCell::new();
    WATCHER_STARTED.get_or_init(|| {
        let _ = start_config_watcher();
    });

    // อ่านค่าตัวแปรที่ Asterisk ส่งมา
    let vars = read_variables()?;
    
    // ดึง DID จาก variables (DNID หรือ extension)
    let did = vars.get("agi_dnid")
        .or_else(|| vars.get("agi_extension"))
        .map(|s| s.as_str())
        .unwrap_or("");
    
    eprintln!("Incoming call - DID: {}", did);
    
    // โหลด config สำหรับ DID นี้
    let config = get_ivr_config(did);
    
    // 1. ตอบรับสาย
    answer()?;

    // 2. เล่น IVR และรอรับ input (สามารถกดได้ระหว่างเล่น)
    let first_digit = stream_file(&config.welcome_sound, "0123456789")?;

    // เก็บตัวเลขที่กด
    let mut input = String::new();
    
    if let Some(digit) = first_digit {
        input.push(digit);
        
        // ตรวจสอบว่าตัวเลขที่กดตรงกับ queue mapping หรือไม่
        if let Some(queue_number) = config.queues.get(&digit.to_string()) {
            // ส่งเข้า queue ทันที
            eprintln!("Routing to queue: {}", queue_number);
            set_variable("QUEUE_NUMBER", queue_number)?;
            exec("Queue", &format!("{},{},,,,,,", queue_number, config.dial_options))?;
            return Ok(());
        }
        
        // ถ้าไม่ใช่ queue อาจเป็นเบอร์ภายใน รอรับตัวถัดไป
        let max_additional = config.max_extension_digits.saturating_sub(1);
        for _ in 0..max_additional {
            let timeout_ms = (config.extension_timeout_seconds * 1000) as i32;
            let next_digit = wait_for_digit(timeout_ms)?;
            if let Some(d) = next_digit {
                if d.is_ascii_digit() {
                    input.push(d);
                } else {
                    break;
                }
            } else {
                break; // timeout หรือไม่กดอะไร
            }
        }
    } else {
        // ไม่กดอะไรเลยระหว่างเล่นเสียง รอรับ input อีกครั้ง
        for _ in 0..config.max_extension_digits {
            let timeout_ms = ((config.extension_timeout_seconds + 2) * 1000) as i32;
            let digit = wait_for_digit(timeout_ms)?;
            if let Some(d) = digit {
                if d.is_ascii_digit() {
                    input.push(d);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // ตรวจสอบว่ามี input หรือไม่
    if input.is_empty() {
        // ไม่กดอะไรเลย ให้เล่นเสียงและวางสาย
        stream_file(&config.goodbye_sound, "")?;
        return Ok(());
    }

    // ตรวจสอบว่า input เป็น queue number หรือไม่ (กรณีกดช้าหลังเสียงจบ)
    if let Some(queue_number) = config.queues.get(&input) {
        eprintln!("Routing to queue: {}", queue_number);
        set_variable("QUEUE_NUMBER", queue_number)?;
        exec("Queue", &format!("{},{},,,,,,", queue_number, config.dial_options))?;
        return Ok(());
    }

    // ถ้าเป็นตัวเลขตามจำนวนหลักที่กำหนด ให้โอนสายไปยังเบอร์ภายใน
    if input.len() >= config.min_extension_digits && input.len() <= config.max_extension_digits {
        eprintln!("Dialing extension: {}", input);
        set_variable("EXTENSION_NUMBER", &input)?;
        exec(
            "Dial",
            &format!("PJSIP/{},{},{}", input, config.dial_timeout_seconds, config.dial_options)
        )?;
    } else {
        // input ไม่ถูกต้อง
        eprintln!("Invalid input: {}", input);
        stream_file(&config.invalid_sound, "")?;
    }
    
    Ok(())
}