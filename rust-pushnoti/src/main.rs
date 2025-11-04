use mysql::*;
use mysql::prelude::*;
use reqwest;
use clap::Parser;
use std::env;

#[derive(Parser, Debug)]
#[command(name = "rust-pushnoti")]
#[command(about = "Send push notifications to agents in a queue")]
struct Args {
    /// Queue ID to query
    #[arg(short, long)]
    queueid: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // ตรวจสอบ environment variables สำหรับการเชื่อมต่อ database
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "mysql://username:password@localhost:3306/dcall".to_string()
    });
    
    println!("Connecting to database...");
    
    // เชื่อมต่อกับ MySQL database
    let pool = Pool::new(database_url.as_str())?;
    let mut conn = pool.get_conn()?;
    
    println!("Querying agentqueue for queueid: {}", args.queueid);
    
    // Query ข้อมูล agentid จาก table agentqueue ตาม queueid
    let agent_ids: Vec<u32> = conn
        .exec_map(
            "SELECT agentid FROM agentqueue WHERE queueid = ?",
            (args.queueid,),
            |agentid| agentid,
        )?;
    
    if agent_ids.is_empty() {
        println!("No agents found for queueid: {}", args.queueid);
        return Ok(());
    }
    
    println!("Found {} agents for queueid {}", agent_ids.len(), args.queueid);
    
    // สร้าง HTTP client
    let client = reqwest::Client::new();
    
    // Loop ส่ง HTTP request สำหรับแต่ละ agentid
    for agent_id in agent_ids {
        let url = format!(
            "https://us-central1-softphone-dcallcenter.cloudfunctions.net/sendPush?ext={}&server=pbx-backoffice.osd.co.th",
            agent_id
        );
        
        println!("Sending request for agent {}: {}", agent_id, url);
        
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("Agent {}: Response status: {}", agent_id, status);
                
                if status.is_success() {
                    println!("Agent {}: Successfully sent push notification", agent_id);
                } else {
                    println!("Agent {}: Failed to send push notification", agent_id);
                }
            }
            Err(e) => {
                eprintln!("Agent {}: Error sending request: {}", agent_id, e);
            }
        }
    }
    
    println!("Completed sending push notifications to all agents");
    Ok(())
}
