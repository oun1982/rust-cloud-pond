use std::env;
use std::io::{self, BufRead, Write};
use std::process::{Command, exit};

fn send_agi(cmd: &str) {
    // Send a command to Asterisk AGI on stdout and flush immediately.
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", cmd).ok();
    stdout.flush().ok();
}

fn read_agi_env() {
    // AGI sends environment lines on stdin terminated by a blank line.
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                if l.trim().is_empty() {
                    break;
                }
                // we could parse variables if needed; ignore for now
            }
            Err(_) => break,
        }
    }
}

fn main() {
    // Read AGI environment (required for proper AGI behavior).
    read_agi_env();

    let arg = env::args().nth(1);

    if arg.is_none() {
        // missing extension
        send_agi("VERBOSE Missing Extension 1");
        send_agi("SET VARIABLE RET 0");
        // match PHP script behavior which exited with code 1 on missing arg
        exit(1);
    }

    let ext = arg.unwrap();

    if ext.len() <= 4 {
        // For safety only allow simple id values (digits); if not digits, treat as zero result.
        let is_digits = ext.chars().all(|c| c.is_ascii_digit());

        if !is_digits {
            send_agi("SET VARIABLE RET 0");
            exit(0);
        }

        // Build SQL query
        let sql = format!("select count(*) as cnt from agents where id = '{}' and type <= '3'", ext);

        // Call system `mysql` client to avoid adding a new crate dependency.
        // This keeps the binary easy to build. If the mysql client is not present,
        // we'll treat it as no rows found (RET=0) but also emit a verbose message.
        let output = Command::new("mysql")
            .args(&["-h", "10.133.1.13", "-udcall", "-pdcallpass", "-D", "dcall", "-N", "-s", "-e", &sql])
            .output();

        match output {
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    let msg = format!("VERBOSE mysql client failed: {} 1", err.trim());
                    send_agi(&msg);
                    send_agi("SET VARIABLE RET 0");
                    exit(0);
                }

                let stdout = String::from_utf8_lossy(&out.stdout);
                let val = stdout.trim();
                // parse integer result
                if let Ok(cnt) = val.parse::<i64>() {
                    if cnt > 0 {
                        send_agi("SET VARIABLE RET 1");
                    } else {
                        send_agi("SET VARIABLE RET 0");
                    }
                } else {
                    // Unexpected output — treat as zero
                    send_agi("VERBOSE unexpected mysql output 1");
                    send_agi("SET VARIABLE RET 0");
                }
            }
            Err(e) => {
                let msg = format!("VERBOSE failed to spawn mysql: {} 1", e);
                send_agi(&msg);
                send_agi("SET VARIABLE RET 0");
            }
        }
    } else {
        // If length > 4, script sets RET = 1
        send_agi("SET VARIABLE RET 1");
    }
}
