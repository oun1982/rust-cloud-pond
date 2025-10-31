
use mysql::*;
use mysql::prelude::*;
use mysql::{Opts, OptsBuilder};
use chrono::{Local, Timelike, Datelike};
use dotenvy::dotenv;
use std::env;

fn main() {
    // Try loading environment from multiple locations for flexibility
    // 1) current working directory (.env)
    dotenv().ok();
    // 2) explicit file via CHECK_TIME_AGI_ENV_FILE, if provided
    if let Ok(custom_env) = std::env::var("CHECK_TIME_AGI_ENV_FILE") {
        let _ = dotenvy::from_filename(custom_env);
    }
    // 3) .env in the same directory as the binary
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let _ = dotenvy::from_filename(dir.join(".env"));
        }
    }
    // 4) common system path fallback
    let _ = dotenvy::from_filename("/etc/check-time-agi.env");

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        verbose("Usage: check-time-agi I|O <time_group>");
        set_vars(0, 0, 0, 0, 0);
        return;
    }

    let mode_flag = args[1].to_uppercase();
    let time_group = &args[2];

    let (mode, col_active, col_start, col_end, col_wstart, col_wend) = match mode_flag.as_str() {
        "I" => ("inbound", "active_in", "i_start", "i_end", "i_workstart", "i_workend"),
        "O" => ("outbound", "active_out", "o_start", "o_end", "o_workstart", "o_workend"),
        _ => {
            verbose("Invalid mode flag: use I or O");
            set_vars(0, 0, 0, 0, 0);
            return;
        }
    };

    // Collect and validate required environment variables
    let db_user = get_env_required("DB_USER");
    let db_pass = env::var("DB_PASS").unwrap_or_default(); // password may be empty
    let db_host = get_env_required("DB_HOST");
    let db_name = get_env_required("DB_NAME");
    let db_port: u16 = env::var("DB_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(3306);

    if db_user.is_none() || db_host.is_none() || db_name.is_none() {
        let mut missing = Vec::new();
        if db_user.is_none() { missing.push("DB_USER"); }
        if db_host.is_none() { missing.push("DB_HOST"); }
        if db_name.is_none() { missing.push("DB_NAME"); }
        verbose(&format!(
            "Missing required env vars: {}. Set them via environment or .env (DB_PORT defaults to 3306).",
            missing.join(", ")
        ));
        set_vars(0, 0, 0, 0, 0);
        return;
    }

    // Build MySQL options using builder to avoid URL parsing issues
    let mut builder = OptsBuilder::new();
    builder = builder
        .user(db_user.as_deref())
        .pass(Some(db_pass))
        .ip_or_hostname(db_host.as_deref())
        .tcp_port(db_port)
        .db_name(db_name.as_deref());

    let pool = match Pool::new(Opts::from(builder)) {
        Ok(p) => p,
        Err(e) => {
            verbose(&format!("DB connect error: {}", e));
            set_vars(0, 0, 0, 0, 0);
            return;
        }
    };

    let mut conn = pool.get_conn().unwrap();

    let now = Local::now();
    let today_ymd = now.format("%Y-%m-%d").to_string();
    let today_md = now.format("%m-%d").to_string();
    let now_sec = now.hour() * 3600 + now.minute() * 60 + now.second();

    let mut res_holiday = 0;
    let mut res_work = 0;
    let mut res_lunch = 0;
    let mut res_morn = 0;
    let mut res_even = 0;
    let mut status = String::new();

    // === Holiday check ===
    let q_holiday = format!(
        "SELECT {}, {}, {} FROM time_holiday WHERE time_group=? AND (holiday_id=? OR holiday_id=?) LIMIT 1",
        col_active, col_start, col_end
    );

    if let Ok(row_opt) = conn.exec_first::<(Option<i64>, Option<i64>, Option<i64>), _, _>(&q_holiday, (time_group, &today_ymd, &today_md)) {
        if let Some((active, s, e)) = row_opt {
            if active.unwrap_or(0) == 1 {
                if let (Some(start), Some(end)) = (s, e) {
                    if now_sec > start as u32 && now_sec < end as u32 {
                        res_holiday = 1;
                        res_work = 1;
                        status = "HOLIDAY_IN_WORK_TIME".into();
                    } else {
                        status = "HOLIDAY_OUT_OF_WORK_TIME -> Fallback WeekTime".into();
                    }
                }
            } else {
                status = "HOLIDAY_INACTIVE -> Fallback WeekTime".into();
            }
        } else {
            status = "NO_HOLIDAY -> Check WeekTime".into();
        }
    } else {
        status = "NO_HOLIDAY -> Check WeekTime".into();
    }

    // === Weekday check ===
    if res_work == 0 {
        let daynum = now.weekday().num_days_from_sunday() as i32;
        let q_week = format!(
            "SELECT {}, {} FROM time_week WHERE time_group=? AND weekid=? LIMIT 1",
            col_wstart, col_wend
        );
        if let Ok(row_opt) = conn.exec_first::<(Option<i64>, Option<i64>), _, _>(&q_week, (time_group, daynum)) {
            if let Some((s, e)) = row_opt {
                if let (Some(start), Some(end)) = (s, e) {
                    if now_sec > start as u32 && now_sec < end as u32 {
                        res_work = 1;
                        status.push_str(" + WEEKDAY_IN_WORK_TIME");
                    } else {
                        status.push_str(" + WEEKDAY_OUT_OF_WORK_TIME");
                    }
                }
            }
        }
    }

    // === Subwindows check ===
    let base = now.weekday().num_days_from_sunday() as i32;
    let subs = vec![(base * 10, 0), (base * 10 + 1, 1), (base * 10 + 2, 2)];
    for (subid, label) in subs {
        let q_sub = format!(
            "SELECT {}, {} FROM time_week WHERE time_group=? AND weekid=? LIMIT 1",
            col_wstart, col_wend
        );
        if let Ok(row_opt) = conn.exec_first::<(Option<i64>, Option<i64>), _, _>(&q_sub, (time_group, subid)) {
            if let Some((s, e)) = row_opt {
                if let (Some(start), Some(end)) = (s, e) {
                    if now_sec > start as u32 && now_sec < end as u32 {
                        match label {
                            0 => res_lunch = 1,
                            1 => res_morn = 1,
                            2 => res_even = 1,
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    verbose(&format!("DEBUG: Current={} | Mode={} | Group={}", now.format("%H:%M:%S"), mode, time_group));
    set_vars(res_holiday, res_work, res_lunch, res_morn, res_even);
    verbose(&format!(
        "TIME_CHECK: Mode={} Group={} HOL={} WORK={} MORN={} LUNCH={} EVEN={} ({})",
        mode, time_group, res_holiday, res_work, res_morn, res_lunch, res_even, status
    ));
}

fn verbose(msg: &str) {
    println!("VERBOSE \"{}\"", msg);
}

fn set_vars(hol: i32, work: i32, lunch: i32, morn: i32, even: i32) {
    println!("SET VARIABLE RES_HOLIDAY {}", hol);
    println!("SET VARIABLE RES_WORKTIME {}", work);
    println!("SET VARIABLE RES_LUNCH {}", lunch);
    println!("SET VARIABLE RES_MORN {}", morn);
    println!("SET VARIABLE RES_EVEN {}", even);
}

fn get_env_required(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}
