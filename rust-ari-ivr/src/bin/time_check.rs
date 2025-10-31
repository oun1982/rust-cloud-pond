use chrono::{Datelike, Local};
use mysql::prelude::*;
use mysql::*;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

// Constants (adapt to your environment)
const DB_HOST: &str = "10.133.1.13";
const DB_USER: &str = "dcall";
const DB_PASS: &str = "dcallpass";
const DB_NAME: &str = "dcall";
const LOG_FILE: &str = "/var/log/asterisk/time_check.log";

fn main() {
    // Parse args
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("VERBOSE \"Usage: time_check [I|O] [time_group]\"");
        println!("SET VARIABLE RES_HOLIDAY 0");
        println!("SET VARIABLE RES_WORKTIME 0");
        println!("SET VARIABLE RES_LUNCH 0");
        println!("SET VARIABLE RES_MORN 0");
        println!("SET VARIABLE RES_EVEN 0");
        println!("SET VARIABLE RES_OVERTIME 0");
        std::process::exit(1);
    }

    let mode_flag = args[1].to_uppercase();
    let time_group = args[2].clone();

    if mode_flag != "I" && mode_flag != "O" {
        println!("VERBOSE \"Invalid mode flag: use I (Inbound) or O (Outbound)\"");
        println!("SET VARIABLE RES_HOLIDAY 0");
        println!("SET VARIABLE RES_WORKTIME 0");
        println!("SET VARIABLE RES_LUNCH 0");
        println!("SET VARIABLE RES_MORN 0");
        println!("SET VARIABLE RES_EVEN 0");
        println!("SET VARIABLE RES_OVERTIME 0");
        std::process::exit(1);
    }

    // Columns by mode
    let (mode, col_hol_active, col_wk_active, col_start, col_end, col_wstart, col_wend) =
        if mode_flag == "I" {
            (
                "inbound",
                "active_in",
                "in_check",
                "i_start",
                "i_end",
                "i_workstart",
                "i_workend",
            )
        } else {
            (
                "outbound",
                "active_out",
                "out_check",
                "o_start",
                "o_end",
                "o_workstart",
                "o_workend",
            )
        };

    // State variables
    let mut res_holiday = 0;
    let mut res_worktime = 0;
    let mut res_lunch = 0;
    let mut res_morn = 0;
    let mut res_even = 0;
    let mut res_overtime = 0;
    let mut status_detail = String::new();

    // Time setup
    let now = Local::now();
    let cur_time_str = now.format("%H:%M:%S").to_string();
    let cur_date_str = now.format("%Y-%m-%d").to_string();
    let cur_month_day_str = now.format("%m-%d").to_string();
    let day_num: u32 = now.weekday().num_days_from_sunday(); // Sunday=0

    // Connect to MySQL
    let opts = OptsBuilder::new()
        .ip_or_hostname(Some(DB_HOST))
        .user(Some(DB_USER))
        .pass(Some(DB_PASS))
        .db_name(Some(DB_NAME));
    let pool = match Pool::new(Opts::from(opts)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("DB Open error: {e}");
            print_defaults_and_exit();
            return;
        }
    };
    let mut conn = match pool.get_conn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("DB Conn error: {e}");
            print_defaults_and_exit();
            return;
        }
    };

    // STEP 1: Holiday check
    let mut hol_start_str = "00:00:00".to_string();
    let mut hol_end_str = "23:59:59".to_string();
    let mut is_holiday_day = false;

    let query_holiday = format!(
        "SELECT h.{col}, SEC_TO_TIME(h.{start}), SEC_TO_TIME(h.{end})\n         FROM time_holiday h\n         WHERE ( (LENGTH(h.holiday_id)=10 AND h.holiday_id = ?) OR (LENGTH(h.holiday_id)=5 AND h.holiday_id = ?) )\n           AND h.time_group = ?\n           AND h.{col} = 1\n         LIMIT 1;",
        col = col_hol_active,
        start = col_start,
        end = col_end
    );

    let holiday_row: Option<(Option<i64>, Option<String>, Option<String>)> = conn
        .exec_first(
            &query_holiday,
            (cur_date_str.clone(), cur_month_day_str.clone(), time_group.clone()),
        )
        .ok()
        .flatten();

    if let Some((hol_active, hol_start, hol_end)) = holiday_row {
        if let Some(s) = hol_start { hol_start_str = s; }
        if let Some(s) = hol_end { hol_end_str = s; }

        if hol_active.unwrap_or(0) == 1 {
            is_holiday_day = true;
            if cur_time_str >= hol_start_str && cur_time_str <= hol_end_str {
                res_holiday = 1;
                status_detail = "HOLIDAY_ACTIVE".to_string();
            } else {
                res_overtime = 1;
                res_worktime = 0;
                status_detail = "HOLIDAY_OVERTIME".to_string();
            }
        }
    }

    // STEP 2: Weektime (if not holiday)
    let mut wk_start_str = String::new();
    let mut wk_end_str = String::new();
    let mut is_in_week_time = false;

    if !is_holiday_day {
        let query_week = format!(
            "SELECT {active}, SEC_TO_TIME({wstart}), SEC_TO_TIME({wend})\n             FROM time_week\n             WHERE time_group = ? AND weekid = ?\n             LIMIT 1;",
            active = col_wk_active,
            wstart = col_wstart,
            wend = col_wend
        );

        let week_row: Option<(Option<i64>, Option<String>, Option<String>)> = conn
            .exec_first(&query_week, (time_group.clone(), day_num as i32))
            .ok()
            .flatten();

        if let Some((wk_active, wk_start, wk_end)) = week_row {
            if wk_active.unwrap_or(0) == 1 {
                if let Some(s) = wk_start.clone() { wk_start_str = s; }
                if let Some(s) = wk_end.clone() { wk_end_str = s; }

                if wk_start.is_some()
                    && wk_end.is_some()
                    && cur_time_str >= wk_start.unwrap()
                    && cur_time_str <= wk_end.unwrap()
                {
                    is_in_week_time = true;
                    status_detail.push_str(" + IN_WEEKTIME");
                } else {
                    status_detail.push_str(" + OUT_OF_WEEKTIME");
                }
            } else {
                status_detail.push_str(" + WEEKDAY_INACTIVE");
            }
        } else {
            status_detail.push_str(" + NO_WEEKDAY_CONFIG");
        }
    }

    // STEP 3: Sub-times (Lunch/Morning/Evening) if not holiday
    if !is_holiday_day {
        let base_id = (day_num * 10) as i32;
        println!(
            "VERBOSE \"DEBUG: Checking sub-times for Day={}, BaseID={}, InWeekTime={}\"",
            day_num, base_id, is_in_week_time
        );

        let sub_times = vec![("LUNCH", base_id), ("MORN", base_id + 1), ("EVEN", base_id + 2)];
        let mut found_sub_time = false;

        for (label, id) in sub_times {
            let query_sub = format!(
                "SELECT {active}, SEC_TO_TIME({wstart}), SEC_TO_TIME({wend})\n                 FROM time_week\n                 WHERE time_group = ? AND weekid = ?\n                 LIMIT 1;",
                active = col_wk_active,
                wstart = col_wstart,
                wend = col_wend
            );

            let sub_row: Option<(Option<i64>, Option<String>, Option<String>)> = conn
                .exec_first(&query_sub, (time_group.clone(), id))
                .ok()
                .flatten();

            if let Some((sub_active, sub_start, sub_end)) = sub_row {
                let sub_start_s = sub_start.clone().unwrap_or_default();
                let sub_end_s = sub_end.clone().unwrap_or_default();
                println!(
                    "VERBOSE \"DEBUG {} (ID:{}): active={:?}, time=[{}-{}], current={}\"",
                    label, id, sub_active, sub_start_s, sub_end_s, cur_time_str
                );

                if sub_active.unwrap_or(0) == 1 {
                    if sub_start.is_some()
                        && sub_end.is_some()
                        && cur_time_str >= sub_start.unwrap()
                        && cur_time_str <= sub_end.unwrap()
                    {
                        found_sub_time = true;
                        match label {
                            "LUNCH" => {
                                res_lunch = 1;
                                status_detail.push_str(" + LUNCH_ACTIVE");
                            }
                            "MORN" => {
                                res_morn = 1;
                                status_detail.push_str(" + MORNING_ACTIVE");
                            }
                            "EVEN" => {
                                res_even = 1;
                                status_detail.push_str(" + EVENING_ACTIVE");
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            } else {
                println!("VERBOSE \"DEBUG {} (ID:{}): No data found\"", label, id);
            }
        }

        if is_in_week_time {
            if found_sub_time {
                res_worktime = 0; // In a specific sub-time, not general worktime
            } else {
                res_worktime = 1;
                status_detail.push_str(" + WORKTIME_ONLY");
            }
        } else {
            if found_sub_time {
                res_worktime = 0;
            } else {
                res_overtime = 1;
                res_worktime = 0;
                status_detail.push_str(" + OVERTIME");
            }
        }
    }

    // STEP 4: Debug
    println!(
        "VERBOSE \"DEBUG: Day={} Current={} | Holiday=[{}-{}] | Week=[{}-{}]\"",
        day_num, cur_time_str, hol_start_str, hol_end_str, wk_start_str, wk_end_str
    );

    // STEP 5: Return variables
    println!("SET VARIABLE RES_HOLIDAY {}", res_holiday);
    println!("SET VARIABLE RES_WORKTIME {}", res_worktime);
    println!("SET VARIABLE RES_LUNCH {}", res_lunch);
    println!("SET VARIABLE RES_MORN {}", res_morn);
    println!("SET VARIABLE RES_EVEN {}", res_even);
    println!("SET VARIABLE RES_OVERTIME {}", res_overtime);
    println!(
        "VERBOSE \"TIME_CHECK: Mode={} Group={} Day={} HOL={} WORK={} MORN={} LUNCH={} EVEN={} OT={} ({})\"",
        mode,
        time_group,
        day_num,
        res_holiday,
        res_worktime,
        res_morn,
        res_lunch,
        res_even,
        res_overtime,
        status_detail
    );

    // STEP 6: Log to file
    if let Err(e) = append_log(
        &now.format("%Y-%m-%d %H:%M:%S").to_string(),
        mode,
        &time_group,
        day_num,
        &cur_time_str,
        res_holiday,
        res_worktime,
        res_morn,
        res_lunch,
        res_even,
        res_overtime,
        &status_detail,
    ) {
        eprintln!("Failed to write log: {e}");
    }
}

fn print_defaults_and_exit() {
    println!("SET VARIABLE RES_HOLIDAY 0");
    println!("SET VARIABLE RES_WORKTIME 0");
    println!("SET VARIABLE RES_LUNCH 0");
    println!("SET VARIABLE RES_MORN 0");
    println!("SET VARIABLE RES_EVEN 0");
    println!("SET VARIABLE RES_OVERTIME 0");
    std::process::exit(1);
}

#[allow(clippy::too_many_arguments)]
fn append_log(
    now_str: &str,
    mode: &str,
    group: &str,
    day_num: u32,
    cur_time: &str,
    hol: i32,
    work: i32,
    morn: i32,
    lunch: i32,
    even: i32,
    ot: i32,
    status: &str,
) -> std::io::Result<()> {
    let mut f = OpenOptions::new()
        .append(true)
        .create(true)
        .write(true)
        .open(LOG_FILE)?;
    let line = format!(
        "{} MODE={} GROUP={} DAY={} CUR={} HOL={} WORK={} MORN={} LUNCH={} EVEN={} OT={} STATUS={}\n",
        now_str, mode, group, day_num, cur_time, hol, work, morn, lunch, even, ot, status
    );
    f.write_all(line.as_bytes())?;
    Ok(())
}
