use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use chrono::{Local, NaiveTime, Timelike};
use futures_util::StreamExt;
use notify::{Event as NotifyEvent, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

// ================= Configuration Structures =================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub asterisk: AsteriskConfig,
    pub ivr: IvrConfig,
    /// Optional per-DID overrides, keyed by dialed extension (DID)
    #[serde(default)]
    pub did_overrides: HashMap<String, IvrConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AsteriskConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub app_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IvrConfig {
    pub greetings: GreetingConfig,
    pub menu: MenuConfig,
    pub queues: Vec<QueueConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GreetingConfig {
    pub worktime: WorktimeConfig,
    pub sounds: GreetingSounds,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorktimeConfig {
    pub enabled: bool,
    pub start_time: String,
    pub end_time: String,
    pub days: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GreetingSounds {
    pub worktime: String,
    pub overtime: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenuConfig {
    pub main_menu_sound: String,
    pub invalid_sound: String,
    /// Sound to play when input times out
    #[serde(default = "default_timeout_sound")]
    pub timeout_sound: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

fn default_timeout_sound() -> String { "en/custom/timeout".to_string() }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueueConfig {
    pub dtmf: String,
    pub queue_name: String,
    pub description: String,
}

// ================= ARI Event Models =================

#[derive(Debug, Deserialize)]
struct AriEvent {
    r#type: String,
    #[serde(default)]
    channel: Option<Channel>,
    #[serde(default)]
    digit: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct Channel {
    id: String,
    name: String,
    state: String,
    caller: Option<CallerInfo>,
    #[serde(default)]
    dialplan: Option<DialplanInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct CallerInfo {
    #[serde(default)]
    number: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct DialplanInfo {
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    exten: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
}

// ================= Channel State Management =================

#[derive(Debug)]
struct ChannelState {
    dtmf_buffer: String,
    last_dtmf_time: std::time::Instant,
    retries: u32,
    /// When the system started waiting for input (used for timeout)
    awaiting_since: std::time::Instant,
    /// The dialed DID (called extension) used to select per-DID IVR override
    called_did: Option<String>,
}

impl ChannelState {
    fn new() -> Self {
        Self {
            dtmf_buffer: String::new(),
            last_dtmf_time: std::time::Instant::now(),
            retries: 0,
            awaiting_since: std::time::Instant::now(),
            called_did: None,
        }
    }
}

// ================= ARI Client =================

#[derive(Clone)]
pub struct AriClient {
    base_url: String,
    username: String,
    password: String,
    client: reqwest::Client,
}

impl AriClient {
    pub fn new(host: &str, port: u16, username: &str, password: &str) -> Self {
        Self {
            base_url: format!("http://{}:{}/ari", host, port),
            username: username.to_string(),
            password: password.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn answer_channel(&self, channel_id: &str) -> Result<()> {
        let url = format!("{}/channels/{}/answer", self.base_url, channel_id);
        self.client
            .post(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("Failed to send answer request")?
            .error_for_status()
            .context("Answer request failed")?;
        Ok(())
    }

    pub async fn play_sound(&self, channel_id: &str, sound: &str) -> Result<String> {
        let url = format!("{}/channels/{}/play", self.base_url, channel_id);
        let response = self.client
            .post(&url)
            .basic_auth(&self.username, Some(&self.password))
            .query(&[("media", format!("sound:{}", sound))])
            .send()
            .await
            .context("Failed to send play request")?
            .error_for_status()
            .context("Play request failed")?;
        let playback: serde_json::Value = response.json().await?;
        Ok(playback["id"].as_str().unwrap_or("").to_string())
    }

    pub async fn continue_in_dialplan(
        &self,
        channel_id: &str,
        context: &str,
        extension: &str,
        priority: i32,
    ) -> Result<()> {
        let url = format!("{}/channels/{}/continue", self.base_url, channel_id);
        info!("🔗 Continue request: context={}, extension={}, priority={}", context, extension, priority);
        let response = self.client
            .post(&url)
            .basic_auth(&self.username, Some(&self.password))
            .query(&[
                ("context", context),
                ("extension", extension),
                ("priority", &priority.to_string()),
            ])
            .send()
            .await
            .context("Failed to send continue request")?;
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("❌ Continue failed: {} - {}", status, error_text);
            return Err(anyhow::anyhow!("Continue failed: {} - {}", status, error_text));
        }
        info!("✅ Continue successful");
        Ok(())
    }

    pub async fn hangup_channel(&self, channel_id: &str) -> Result<()> {
        let url = format!("{}/channels/{}", self.base_url, channel_id);
        let _ = self.client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await;
        Ok(())
    }
}

// ================= IVR Application =================

pub struct IvrApp {
    ari_client: AriClient,
    config: Arc<ArcSwap<Config>>,
    channel_states: Arc<Mutex<HashMap<String, ChannelState>>>,
}

impl IvrApp {
    pub fn new(ari_client: AriClient, config: Config) -> Self {
        Self {
            ari_client,
            config: Arc::new(ArcSwap::from_pointee(config)),
            channel_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_config(&self) -> Arc<Config> {
        self.config.load_full()
    }

    pub fn reload_config(&self, new_config: Config) {
        info!("🔄 Reloading configuration...");
        self.config.store(Arc::new(new_config));
        info!("✅ Configuration reloaded successfully");
    }

    pub async fn handle_stasis_start(&self, channel: Channel) -> Result<()> {
        info!(
            "📞 New call from: {} ({})",
            channel
                .caller
                .as_ref()
                .and_then(|c| c.number.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("unknown"),
            channel.id
        );

        {
            let mut states = self.channel_states.lock().await;
            let mut st = ChannelState::new();
            if let Some(d) = channel.dialplan.as_ref().and_then(|d| d.exten.clone()) {
                st.called_did = Some(d);
            }
            states.insert(channel.id.clone(), st);
        }

        if let Err(e) = self.ari_client.answer_channel(&channel.id).await {
            error!("Failed to answer channel: {}", e);
            return Ok(());
        }

        let config = self.get_config();
        // Play time-of-day prompt (either worktime or overtime)
        tokio::time::sleep(Duration::from_millis(300)).await;
        if let Err(e) = self.play_prompt_for_time_of_day(&channel.id, &config).await {
            error!("Failed to play prompt: {}", e);
        }
        // Set awaiting_since to now for timeout tracking
        {
            let mut states = self.channel_states.lock().await;
            if let Some(st) = states.get_mut(&channel.id) {
                st.awaiting_since = std::time::Instant::now();
            }
        }

        // Start watcher for input/timeouts
        self.start_input_watcher(channel.id.clone());

        Ok(())
    }

    pub async fn handle_dtmf(&self, channel_id: String, digit: String) -> Result<()> {
        info!("🔢 DTMF received: {} from channel {}", digit, channel_id);
        // Buffer digits and update timers. Decision is made by watcher task.
        let mut states = self.channel_states.lock().await;
        if let Some(st) = states.get_mut(&channel_id) {
            st.dtmf_buffer.push_str(&digit);
            st.last_dtmf_time = std::time::Instant::now();
        }
        Ok(())
    }

    async fn play_prompt_for_time_of_day(&self, channel_id: &str, config: &Config) -> Result<()> {
        let sound = if self.is_work_hours(&config.ivr.greetings.worktime) {
            &config.ivr.greetings.sounds.worktime
        } else {
            &config.ivr.greetings.sounds.overtime
        };
        self.play_sound(channel_id, sound).await
    }

    fn is_work_hours(&self, worktime: &WorktimeConfig) -> bool {
        if !worktime.enabled {
            return true;
        }
        let now = Local::now();
        let current_day = now.format("%A").to_string();
        if !worktime.days.contains(&current_day) {
            return false;
        }
        let start_time = NaiveTime::parse_from_str(&worktime.start_time, "%H:%M:%S")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        let end_time = NaiveTime::parse_from_str(&worktime.end_time, "%H:%M:%S")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        let current_time = NaiveTime::from_hms_opt(now.hour(), now.minute(), now.second()).unwrap();
        current_time >= start_time && current_time <= end_time
    }

    async fn play_sound(&self, channel_id: &str, sound: &str) -> Result<()> {
        info!("🔊 Playing sound: {} on channel {}", sound, channel_id);
        self.ari_client.play_sound(channel_id, sound).await?;
        Ok(())
    }

    // 🔧 อัปเดตส่วนนี้ให้ส่งกลับไป context [custom-inbound]
    async fn send_to_queue(&self, channel_id: &str, queue_name: &str) -> Result<()> {
        info!("📥 Sending channel {} to queue {}", channel_id, queue_name);
        info!("   Using context: custom-inbound, extension: {}, priority: 1", queue_name);
        match self.ari_client
            .continue_in_dialplan(channel_id, "custom-inbound", queue_name, 1)
            .await {
                Ok(_) => {
                    info!("✅ Channel successfully moved to queue {}", queue_name);
                    let mut states = self.channel_states.lock().await;
                    states.remove(channel_id);
                    Ok(())
                }
                Err(e) => {
                    error!("❌ Failed to continue in dialplan: {}", e);
                    Err(e)
                }
            }
    }

    pub async fn handle_channel_destroyed(&self, channel_id: &str) {
        info!("📴 Channel destroyed: {}", channel_id);
        let mut states = self.channel_states.lock().await;
        states.remove(channel_id);
    }

    async fn get_ivr_for_channel(&self, channel_id: &str) -> IvrConfig {
        let cfg = self.get_config();
        let states = self.channel_states.lock().await;
        let did = states.get(channel_id).and_then(|s| s.called_did.clone());
        drop(states);
        if let Some(d) = did {
            if let Some(ivr) = cfg.did_overrides.get(&d) {
                return ivr.clone();
            }
        }
        cfg.ivr.clone()
    }

    fn start_input_watcher(self: &Self, channel_id: String) {
        let this = self.clone_for_task();
        tokio::spawn(async move {
            loop {
                // Stop when channel state no longer exists
                let (dtmf_buffer, last_dtmf, retries, timeout_sec, max_retries) = {
                    let cfg_full = this.get_config();
                    let mut guard = this.channel_states.lock().await;
                    let st = match guard.get(&channel_id) {
                        Some(s) => s,
                        None => break,
                    };
                    let ivr_cfg = if let Some(d) = st.called_did.as_ref() {
                        cfg_full.did_overrides.get(d).unwrap_or(&cfg_full.ivr)
                    } else {
                        &cfg_full.ivr
                    };
                    (
                        st.dtmf_buffer.clone(),
                        st.last_dtmf_time,
                        st.retries,
                        ivr_cfg.menu.timeout_seconds,
                        ivr_cfg.menu.max_retries,
                    )
                };

                let now = std::time::Instant::now();
                // Decision rules:
                // 1) Extension dialing: >=3 digits and 3s of inactivity
                if dtmf_buffer.len() >= 4 && now.duration_since(last_dtmf) >= Duration::from_secs(3) {
                    info!("📨 Treating '{}' as extension for channel {}", dtmf_buffer, channel_id);
                    let _ = this.send_to_extension(&channel_id, &dtmf_buffer).await;
                    // After routing, stop watcher
                    break;
                }
                // 2) Single-digit menu: exactly 1 digit and 1s of inactivity
                if dtmf_buffer.len() == 1 && now.duration_since(last_dtmf) >= Duration::from_secs(1) {
                    let ivr_cfg = this.get_ivr_for_channel(&channel_id).await;
                    if let Some(q) = ivr_cfg.queues.iter().find(|q| q.dtmf == dtmf_buffer) {
                        info!("✅ Menu selection '{}' -> queue {}", dtmf_buffer, q.queue_name);
                        let _ = this.send_to_queue(&channel_id, &q.queue_name).await;
                        break;
                    } else {
                        // invalid single digit
                        let mut guard = this.channel_states.lock().await;
                        if let Some(st) = guard.get_mut(&channel_id) {
                            st.dtmf_buffer.clear();
                            st.retries += 1;
                            st.awaiting_since = std::time::Instant::now();
                        }
                        let ivr_cfg = this.get_ivr_for_channel(&channel_id).await;
                        let _ = this.play_sound(&channel_id, &ivr_cfg.menu.invalid_sound).await;
                        // If exceeds retries -> fallback
                        if retries + 1 > max_retries {
                            let _ = this.send_to_queue(&channel_id, "10002").await;
                            break;
                        }
                    }
                }

                // 3) Timeout handling: No digits at all for timeout_seconds
                {
                    let mut guard = this.channel_states.lock().await;
                    if let Some(st) = guard.get_mut(&channel_id) {
                        if st.dtmf_buffer.is_empty() && now.duration_since(st.awaiting_since) >= Duration::from_secs(timeout_sec) {
                            st.retries += 1;
                            st.awaiting_since = std::time::Instant::now();
                            let ivr_cfg = this.get_ivr_for_channel(&channel_id).await;
                            let _ = this.play_sound(&channel_id, &ivr_cfg.menu.timeout_sound).await;
                            // Replay prompt according to time-of-day
                            let _ = this.play_prompt_for_time_of_day(&channel_id, &this.get_config()).await;
                            // Exceeded retries -> send to fallback queue 10002
                            if st.retries > ivr_cfg.menu.max_retries {
                                let _ = this.send_to_queue(&channel_id, "10002").await;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }

    fn clone_for_task(&self) -> Self {
        IvrApp {
            ari_client: self.ari_client.clone(),
            config: self.config.clone(),
            channel_states: self.channel_states.clone(),
        }
    }

    async fn send_to_extension(&self, channel_id: &str, extension: &str) -> Result<()> {
        info!("📤 Sending channel {} to extension {}", channel_id, extension);
        self.ari_client
            .continue_in_dialplan(channel_id, "custom-inbound", extension, 1)
            .await
    }
}

// ================= Config Manager =================

pub struct ConfigManager {
    config_path: String,
}

impl ConfigManager {
    pub fn new(config_path: String) -> Self {
        Self { config_path }
    }

    pub fn load_config(&self) -> Result<Config> {
        let content =
            std::fs::read_to_string(&self.config_path).context("Failed to read config file")?;
        let config: Config =
            serde_yaml::from_str(&content).context("Failed to parse config file")?;
        Ok(config)
    }

    pub async fn watch_config<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Config) + Send + 'static,
    {
        let config_path = self.config_path.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let mut watcher =
            notify::recommended_watcher(move |res: Result<NotifyEvent, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        let _ = tx.blocking_send(());
                    }
                }
            })?;
        watcher.watch(Path::new(&self.config_path), RecursiveMode::NonRecursive)?;
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                info!("📝 Config file changed, reloading...");
                tokio::time::sleep(Duration::from_millis(200)).await;
                match std::fs::read_to_string(&config_path) {
                    Ok(content) => match serde_yaml::from_str::<Config>(&content) {
                        Ok(new_config) => {
                            info!("✅ Config parsed successfully");
                            callback(new_config);
                        }
                        Err(e) => error!("❌ Failed to parse config: {}", e),
                    },
                    Err(e) => error!("❌ Failed to read config file: {}", e),
                }
            }
        });
        std::mem::forget(watcher);
        Ok(())
    }
}

// ================= WebSocket Event Handler =================

async fn handle_websocket_events(
    ws_url: String,
    ivr_app: Arc<IvrApp>,
) -> Result<()> {
    loop {
        info!("🔌 Connecting to WebSocket: {}", ws_url);
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                info!("✅ Connected to Asterisk ARI WebSocket");
                let (_, mut read) = ws_stream.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(Message::Text(text)) => {
                            if let Ok(event) = serde_json::from_str::<AriEvent>(&text) {
                                let ivr = ivr_app.clone();
                                tokio::spawn(async move {
                                    match event.r#type.as_str() {
                                        "StasisStart" => {
                                            if let Some(channel) = event.channel {
                                                if let Err(e) = ivr.handle_stasis_start(channel).await {
                                                    error!("Error handling StasisStart: {}", e);
                                                }
                                            }
                                        }
                                        "ChannelDtmfReceived" => {
                                            if let (Some(channel), Some(digit)) = (event.channel, event.digit) {
                                                if let Err(e) = ivr.handle_dtmf(channel.id, digit).await {
                                                    error!("Error handling DTMF: {}", e);
                                                }
                                            }
                                        }
                                        "ChannelDestroyed" => {
                                            if let Some(channel) = event.channel {
                                                ivr.handle_channel_destroyed(&channel.id).await;
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            }
                        }
                        Ok(Message::Close(_)) => {
                            warn!("WebSocket connection closed");
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => error!("Failed to connect to WebSocket: {}", e),
        }
        warn!("Reconnecting in 5 seconds...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// ================= Main =================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    info!("🚀 Starting Asterisk IVR Application");
    let config_manager = ConfigManager::new("config/config.yaml".to_string());
    let config = config_manager.load_config()?;
    info!("📋 Loaded configuration for app: {}", config.asterisk.app_name);
    let ari_client = AriClient::new(
        &config.asterisk.host,
        config.asterisk.port,
        &config.asterisk.username,
        &config.asterisk.password,
    );
    let ivr_app = Arc::new(IvrApp::new(ari_client, config.clone()));
    let ivr_app_clone = ivr_app.clone();
    config_manager.watch_config(move |new_config| {
        ivr_app_clone.reload_config(new_config);
    }).await?;
    let ws_url = format!(
        "ws://{}:{}/ari/events?app={}&api_key={}:{}",
        config.asterisk.host,
        config.asterisk.port,
        config.asterisk.app_name,
        config.asterisk.username,
        config.asterisk.password
    );
    info!("👂 Starting event listener...");
    let ws_handle = tokio::spawn(handle_websocket_events(ws_url, ivr_app.clone()));
    tokio::signal::ctrl_c().await?;
    info!("👋 Shutting down...");
    ws_handle.abort();
    Ok(())
}
