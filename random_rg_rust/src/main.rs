use rand::seq::SliceRandom;
use rand::thread_rng;
use std::env;
use std::io::{self, BufRead, Write};

fn read_agi_env() {
    // ต้องอ่าน AGI env ให้หมดจนเจอบรรทัดว่าง
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let l = match line {
            Ok(v) => v,
            Err(_) => break,
        };
        if l.trim().is_empty() {
            break;
        }
    }
}

fn agi_cmd(cmd: &str) -> String {
    let mut stdout = io::stdout();
    writeln!(stdout, "{cmd}").ok();
    stdout.flush().ok();

    let mut resp = String::new();
    io::stdin().read_line(&mut resp).ok();
    resp.trim().to_string()
}

fn agi_verbose(msg: &str) {
    let safe = msg.replace('"', "'"); // กัน quote แตก
    let _ = agi_cmd(&format!("VERBOSE \"{}\" 1", safe));
}

fn agi_exec(app: &str, args: &str) {
    let _ = agi_cmd(&format!("EXEC {} {}", app, args));
}

fn agi_get_var(var: &str) -> String {
    let res = agi_cmd(&format!("GET VARIABLE {}", var));
    // ตัวอย่าง: 200 result=1 (ANSWER)
    if let (Some(s), Some(e)) = (res.find('('), res.find(')')) {
        if e > s + 1 {
            return res[s + 1..e].to_string();
        }
    }
    String::new()
}

fn main() {
    // 1) อ่าน AGI environment ก่อน (ห้ามข้าม)
    read_agi_env();

    // 2) รับ ringgroups จาก argv: random_rg_rust,10421,10422,...
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        agi_verbose("No Ring Group arguments from dialplan (RG_LIST empty?)");
        return;
    }

    // args[0] = program name, args[1..] = ring groups
    let mut ring_groups: Vec<String> = args[1..]
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // กันซ้ำ (ถ้ามีใส่มาซ้ำใน dialplan)
    ring_groups.sort();
    ring_groups.dedup();

    if ring_groups.is_empty() {
        agi_verbose("RG_LIST parsed empty after cleanup");
        return;
    }

    agi_verbose(&format!("Ring Groups: {}", ring_groups.join(",")));

    // 3) shuffle เพื่อ random แบบไม่ซ้ำ
    let mut rng = thread_rng();
    ring_groups.shuffle(&mut rng);

    // 4) วน dial ทีละ RG
    for rg in ring_groups {
        agi_verbose(&format!("Dial Ring Group {}", rg));

        // สำคัญ: ผ่าน from-internal เพื่อให้ FreePBX recording/CDR ทำงานปกติ
        agi_exec("Dial", &format!("Local/{}@from-internal,10", rg));

        let status = agi_get_var("DIALSTATUS");
        agi_verbose(&format!("DIALSTATUS={}", status));

        if status == "ANSWER" {
            agi_verbose(&format!("Answered by RG {}", rg));
            return;
        }
        // ไม่ ANSWER => ไปตัวถัดไป
    }

    agi_verbose("No agent answered (all RG tried)");
}
