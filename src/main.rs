use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Json, Router, extract::State,
    http::header,
};

// Configuration Structs
#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppConfig {
    #[serde(default = "default_physical_mic")]
    physical_mic: String,
    #[serde(default = "default_target_app")]
    target_app: String,
    #[serde(default = "default_target_volume")]
    target_volume: f32,
    #[serde(default = "default_mic_volume")]
    mic_volume: f32,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_physical_mic() -> String {
    "".to_string()
}
fn default_target_app() -> String {
    "Brave".to_string()
}
fn default_target_volume() -> f32 {
    1.0
}
fn default_mic_volume() -> f32 {
    1.0
}
fn default_enabled() -> bool {
    true
}

// Device entries shown in the UI
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DeviceEntry {
    name: String,
    description: String,
}

// App State shared between background linking loop and Web API
struct AppState {
    config: AppConfig,
    available_mics: Vec<DeviceEntry>,
    available_apps: Vec<String>,
    mic_connected: bool,
    app_connected: bool,
    default_sink_name: String,
}

// Pipewire JSON Parsing Structs
#[derive(Debug, Deserialize, Clone)]
struct PwObject {
    id: u32,
    #[serde(rename = "type")]
    object_type: String,
    info: Option<PwInfo>,
    props: Option<serde_json::Value>,
    metadata: Option<Vec<PwMetadata>>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct PwInfo {
    props: Option<serde_json::Value>,
    direction: Option<String>,
    #[serde(rename = "output-node-id")]
    output_node_id: Option<u32>,
    #[serde(rename = "output-port-id")]
    output_port_id: Option<u32>,
    #[serde(rename = "input-node-id")]
    input_node_id: Option<u32>,
    #[serde(rename = "input-port-id")]
    input_port_id: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
struct PwMetadata {
    key: String,
    value: serde_json::Value,
}

// Internal Representation of PipeWire Objects
#[derive(Debug)]
struct NodeInfo {
    id: u32,
    name: String,
    description: String,
    media_class: String,
    app_name: Option<String>,
}

#[derive(Debug)]
struct PortInfo {
    id: u32,
    node_id: u32,
    name: String,
    direction: String,
}

#[derive(Debug)]
struct LinkInfo {
    out_port: u32,
    in_port: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Channel {
    Left,
    Right,
    Mono,
}

// Configuration helper functions
fn get_config_path() -> PathBuf {
    if let Some(mut path) = dirs::config_dir() {
        path.push("pbs");
        let _ = std::fs::create_dir_all(&path);
        path.push("config.json");
        path
    } else {
        PathBuf::from("config.json")
    }
}

fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
            return config;
        }
    }
    AppConfig {
        physical_mic: "".to_string(),
        target_app: "Brave".to_string(),
        target_volume: 1.0,
        mic_volume: 1.0,
        enabled: true,
    }
}

fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, content);
    }
}

// Property extraction helper functions
fn get_prop_str(obj: &PwObject, key: &str) -> Option<String> {
    let check_val = |val: &serde_json::Value| {
        val.as_str().map(|s| s.to_string()).or_else(|| Some(val.to_string()))
    };
    if let Some(ref info) = obj.info {
        if let Some(ref props) = info.props {
            if let Some(val) = props.get(key) {
                return check_val(val);
            }
        }
    }
    if let Some(ref props) = obj.props {
        if let Some(val) = props.get(key) {
            return check_val(val);
        }
    }
    None
}

fn get_prop_u32(obj: &PwObject, key: &str) -> Option<u32> {
    if let Some(ref info) = obj.info {
        if let Some(ref props) = info.props {
            if let Some(val) = props.get(key) {
                if let Some(num) = val.as_u64() {
                    return Some(num as u32);
                }
                if let Some(s) = val.as_str() {
                    if let Ok(num) = s.parse::<u32>() {
                        return Some(num);
                    }
                }
            }
        }
    }
    if let Some(ref props) = obj.props {
        if let Some(val) = props.get(key) {
            if let Some(num) = val.as_u64() {
                return Some(num as u32);
            }
            if let Some(s) = val.as_str() {
                if let Ok(num) = s.parse::<u32>() {
                    return Some(num);
                }
            }
        }
    }
    None
}

// Channel detection
fn detect_channel(port_name: &str) -> Channel {
    let lower = port_name.to_lowercase();
    
    // Split by non-alphanumeric characters to get tokens
    let tokens: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric()).collect();
    
    for token in tokens {
        if token == "fl" || token == "left" || token == "l" || token == "1" {
            return Channel::Left;
        }
        if token == "fr" || token == "right" || token == "r" || token == "2" {
            return Channel::Right;
        }
    }
    
    // Fallback substring checks for standard suffixes
    if lower.contains("front.left") || lower.contains("front-left") || lower.contains("fl") {
        return Channel::Left;
    }
    if lower.contains("front.right") || lower.contains("front-right") || lower.contains("fr") {
        return Channel::Right;
    }
    if lower.ends_with("_l") || lower.ends_with(".l") {
        return Channel::Left;
    }
    if lower.ends_with("_r") || lower.ends_with(".r") {
        return Channel::Right;
    }
    
    Channel::Mono
}

// Match output ports to input ports
fn get_desired_links(out_ports: &[&PortInfo], in_ports: &[&PortInfo]) -> Vec<(u32, u32)> {
    let mut links = Vec::new();
    let out_left: Vec<&&PortInfo> = out_ports.iter().filter(|p| detect_channel(&p.name) == Channel::Left).collect();
    let out_right: Vec<&&PortInfo> = out_ports.iter().filter(|p| detect_channel(&p.name) == Channel::Right).collect();
    let out_mono: Vec<&&PortInfo> = out_ports.iter().filter(|p| detect_channel(&p.name) == Channel::Mono).collect();

    let in_left: Vec<&&PortInfo> = in_ports.iter().filter(|p| detect_channel(&p.name) == Channel::Left).collect();
    let in_right: Vec<&&PortInfo> = in_ports.iter().filter(|p| detect_channel(&p.name) == Channel::Right).collect();
    let in_mono: Vec<&&PortInfo> = in_ports.iter().filter(|p| detect_channel(&p.name) == Channel::Mono).collect();

    if (!out_left.is_empty() || !out_right.is_empty()) && (!in_left.is_empty() || !in_right.is_empty()) {
        // Link Left to Left
        for ol in &out_left {
            for il in &in_left {
                links.push((ol.id, il.id));
            }
        }
        // Link Right to Right
        for or in &out_right {
            for ir in &in_right {
                links.push((or.id, ir.id));
            }
        }
        // Mono fallbacks if one side is missing Left/Right
        if in_left.is_empty() && in_right.is_empty() {
            for im in &in_mono {
                for ol in &out_left {
                    links.push((ol.id, im.id));
                }
                for or in &out_right {
                    links.push((or.id, im.id));
                }
            }
        }
        if out_left.is_empty() && out_right.is_empty() {
            for om in &out_mono {
                for il in &in_left {
                    links.push((om.id, il.id));
                }
                for ir in &in_right {
                    links.push((om.id, ir.id));
                }
            }
        }
    } else {
        // Fallback: Link every output port to every input port (e.g. mono to stereo)
        for op in out_ports {
            for ip in in_ports {
                links.push((op.id, ip.id));
            }
        }
    }
    links
}

fn is_port_linked(links: &[LinkInfo], out_port: u32, in_port: u32) -> bool {
    for link in links {
        if link.out_port == out_port && link.in_port == in_port {
            return true;
        }
    }
    false
}

// Execute pw-dump to read current state
async fn run_pw_dump() -> Result<Vec<PwObject>, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("pw-dump")
        .output()
        .await?;
    if !output.status.success() {
        return Err("pw-dump command failed".into());
    }
    let objects: Vec<PwObject> = serde_json::from_slice(&output.stdout)?;
    Ok(objects)
}

// Execute pw-link to link two ports
async fn run_pw_link(out_port: u32, in_port: u32) {
    println!("Linking port {} -> {}...", out_port, in_port);
    let status = tokio::process::Command::new("pw-link")
        .arg(out_port.to_string())
        .arg(in_port.to_string())
        .status()
        .await;
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("pw-link exited with status: {}", s),
        Err(e) => eprintln!("Failed to execute pw-link: {}", e),
    }
}

// Process parser
fn parse_dump(objects: &[PwObject]) -> (Vec<NodeInfo>, Vec<PortInfo>, Vec<LinkInfo>, Option<String>) {
    let mut nodes = Vec::new();
    let mut ports = Vec::new();
    let mut links = Vec::new();
    let mut default_sink = None;

    for obj in objects {
        match obj.object_type.as_str() {
            "PipeWire:Interface:Node" => {
                let name = get_prop_str(obj, "node.name").unwrap_or_default();
                let description = get_prop_str(obj, "node.description").unwrap_or_else(|| name.clone());
                let media_class = get_prop_str(obj, "media.class").unwrap_or_default();
                let app_name = get_prop_str(obj, "application.name").or_else(|| get_prop_str(obj, "app.name"));
                nodes.push(NodeInfo {
                    id: obj.id,
                    name,
                    description,
                    media_class,
                    app_name,
                });
            }
            "PipeWire:Interface:Port" => {
                if let Some(node_id) = get_prop_u32(obj, "node.id") {
                    let name = get_prop_str(obj, "port.name").unwrap_or_default();
                    let raw_dir = get_prop_str(obj, "port.direction")
                        .or_else(|| obj.info.as_ref().and_then(|info| info.direction.clone()))
                        .unwrap_or_default()
                        .to_lowercase();
                    let direction = if raw_dir.starts_with("in") {
                        "in".to_string()
                    } else if raw_dir.starts_with("out") {
                        "out".to_string()
                    } else {
                        raw_dir
                    };
                    ports.push(PortInfo {
                        id: obj.id,
                        node_id,
                        name,
                        direction,
                    });
                }
            }
            "PipeWire:Interface:Link" => {
                if let Some(ref info) = obj.info {
                    if let (Some(out_p), Some(in_p)) = (info.output_port_id, info.input_port_id) {
                        links.push(LinkInfo {
                            out_port: out_p,
                            in_port: in_p,
                        });
                    }
                }
            }
            "PipeWire:Interface:Metadata" => {
                if get_prop_str(obj, "metadata.name").as_deref() == Some("default") {
                    if let Some(ref meta_list) = obj.metadata {
                        for meta in meta_list {
                            if meta.key == "default.audio.sink" {
                                if let Some(name_val) = meta.value.get("name") {
                                    if let Some(n) = name_val.as_str() {
                                        default_sink = Some(n.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    (nodes, ports, links, default_sink)
}

// pw-cli virtual microphone supervisor
struct VirtualDeviceManager;

impl VirtualDeviceManager {
    fn new() -> Self {
        Self
    }

    async fn ensure_running(&self) {
        println!("Spawning PBS Virtual Mic node using pw-cli...");
        let status = tokio::process::Command::new("pw-cli")
            .arg("create-node")
            .arg("adapter")
            .arg("{ factory.name=support.null-audio-sink node.name=pbs_virtual_mic node.description=\"PBS Virtual Mic\" media.class=Audio/Source/Virtual object.linger=true audio.position=[FL FR] }")
            .status()
            .await;

        match status {
            Ok(s) if s.success() => {
                println!("Successfully created pbs_virtual_mic");
            }
            Ok(s) => eprintln!("pw-cli exited with status: {}", s),
            Err(e) => eprintln!("Failed to execute pw-cli: {}", e),
        }
    }

    async fn stop(&self) {
        println!("Terminating PBS Virtual Mic node using pw-cli...");
        let status = tokio::process::Command::new("pw-cli")
            .arg("destroy")
            .arg("pbs_virtual_mic")
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        
        match status {
            Ok(s) if s.success() => {
                println!("Successfully destroyed pbs_virtual_mic");
            }
            Ok(_) => {} // Silently ignore if already destroyed
            Err(e) => eprintln!("Failed to execute pw-cli destroy: {}", e),
        }
    }
}

impl Drop for VirtualDeviceManager {
    fn drop(&mut self) {
        println!("Cleaning up PBS Virtual Mic on shutdown...");
        let _ = std::process::Command::new("pw-cli")
            .arg("destroy")
            .arg("pbs_virtual_mic")
            .stderr(std::process::Stdio::null())
            .status();
    }
}

struct TrayIcon;

impl ksni::Tray for TrayIcon {
    fn icon_name(&self) -> String {
        "audio-input-microphone".to_string()
    }

    fn id(&self) -> String {
        "pbs-soundpad".to_string()
    }

    fn title(&self) -> String {
        "PBS Soundpad".to_string()
    }



    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = std::process::Command::new("xdg-open")
            .arg("http://localhost:9876")
            .spawn();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Open Dashboard".to_string(),
                activate: Box::new(|_| {
                    let _ = std::process::Command::new("xdg-open")
                        .arg("http://localhost:9876")
                        .spawn();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|_| {
                    println!("Quit clicked. Cleaning up pbs_virtual_mic...");
                    let _ = std::process::Command::new("pw-cli")
                        .arg("destroy")
                        .arg("pbs_virtual_mic")
                        .status();
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

// Axum API Handlers
async fn serve_index() -> impl IntoResponse {
    Html(include_str!("index.html"))
}


async fn serve_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("app.js"),
    )
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    enabled: bool,
    selected_mic: String,
    selected_app: String,
    selected_volume: f32,
    selected_mic_volume: f32,
    available_mics: Vec<DeviceEntry>,
    available_apps: Vec<String>,
    mic_connected: bool,
    app_connected: bool,
    default_sink_name: String,
}

async fn get_status(State(state): State<Arc<Mutex<AppState>>>) -> Json<StatusResponse> {
    let s = state.lock().await;
    Json(StatusResponse {
        enabled: s.config.enabled,
        selected_mic: s.config.physical_mic.clone(),
        selected_app: s.config.target_app.clone(),
        selected_volume: s.config.target_volume,
        selected_mic_volume: s.config.mic_volume,
        available_mics: s.available_mics.clone(),
        available_apps: s.available_apps.clone(),
        mic_connected: s.mic_connected,
        app_connected: s.app_connected,
        default_sink_name: s.default_sink_name.clone(),
    })
}

#[derive(Debug, Deserialize)]
struct SettingsRequest {
    physical_mic: String,
    target_app: String,
    target_volume: f32,
    mic_volume: f32,
    enabled: bool,
}

async fn post_settings(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(req): Json<SettingsRequest>,
) -> impl IntoResponse {
    let mut s = state.lock().await;
    s.config.physical_mic = req.physical_mic;
    s.config.target_app = req.target_app;
    s.config.target_volume = req.target_volume;
    s.config.mic_volume = req.mic_volume;
    s.config.enabled = req.enabled;
    save_config(&s.config);
    "OK"
}

#[tokio::main]
async fn main() {
    let config = load_config();
    println!("Loaded config: {:?}", config);

    let state = Arc::new(Mutex::new(AppState {
        config,
        available_mics: Vec::new(),
        available_apps: Vec::new(),
        mic_connected: false,
        app_connected: false,
        default_sink_name: "Default Output".to_string(),
    }));

    // Start background linking & monitoring daemon
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let virtual_device_manager = VirtualDeviceManager::new();

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let (config, enabled) = {
                let s = state_clone.lock().await;
                (s.config.clone(), s.config.enabled)
            };

            // Read Pipewire graph
            let objects = match run_pw_dump().await {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("Error querying Pipewire graph: {}", e);
                    continue;
                }
            };

            let (nodes, ports, links, default_sink_node_name) = parse_dump(&objects);

            // Locate our virtual mic in the nodes list
            let mut vinput_node_id = None;
            for node in &nodes {
                if node.name == "pbs_virtual_mic" {
                    vinput_node_id = Some(node.id);
                }
            }

            // Manage the virtual device using VirtualDeviceManager
            if enabled {
                if vinput_node_id.is_none() {
                    virtual_device_manager.ensure_running().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
            } else {
                if vinput_node_id.is_some() {
                    virtual_device_manager.stop().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
            }

            let mut available_mics = Vec::new();
            let mut available_apps = std::collections::HashSet::new();

            let mut mic_node_id = None;
            let mut app_node_ids = Vec::new();
            let mut default_sink_desc = "Default Output".to_string();

            // Locate node targets and build dropdown lists
            for node in &nodes {
                // Collect microphones
                if node.media_class == "Audio/Source" || node.media_class == "Audio/Source/Virtual" {
                    if node.name != "pbs_virtual_mic" {
                        available_mics.push(DeviceEntry {
                            name: node.name.clone(),
                            description: node.description.clone(),
                        });
                    }
                }

                // Collect playing applications
                if node.media_class == "Stream/Output/Audio" {
                    if let Some(ref app_name) = node.app_name {
                        available_apps.insert(app_name.clone());
                    } else {
                        available_apps.insert(node.name.clone());
                    }
                }

                // Check matches for our selected mic
                if !config.physical_mic.is_empty() && node.name == config.physical_mic {
                    mic_node_id = Some(node.id);
                }

                // Check matches for our selected app (case-insensitive substring match)
                if !config.target_app.is_empty() && node.media_class == "Stream/Output/Audio" {
                    let target_lower = config.target_app.to_lowercase();
                    let match_found = node.name.to_lowercase().contains(&target_lower)
                        || node.description.to_lowercase().contains(&target_lower)
                        || node.app_name.as_ref().map(|s| s.to_lowercase().contains(&target_lower)).unwrap_or(false);
                    if match_found {
                        app_node_ids.push(node.id);
                    }
                }

                // Check default sink description
                if let Some(ref def_sink_name) = default_sink_node_name {
                    if node.name == *def_sink_name {
                        default_sink_desc = node.description.clone();
                    }
                }
            }

            let mic_connected = mic_node_id.is_some();
            let app_connected = !app_node_ids.is_empty();

            // Update shared state details
            {
                let mut s = state_clone.lock().await;
                s.available_mics = available_mics;
                s.available_apps = available_apps.into_iter().collect();
                s.mic_connected = mic_connected;
                s.app_connected = app_connected;
                s.default_sink_name = default_sink_desc;
            }

            // Sync links if enabled and pbs_input is present
            if enabled {
                if let Some(vinput_id) = vinput_node_id {
                    let vinput_in_ports: Vec<&PortInfo> = ports.iter()
                        .filter(|p| p.node_id == vinput_id && p.direction == "in")
                        .collect();

                    if !vinput_in_ports.is_empty() {
                        // Link Physical Mic to PBS Input
                        if let Some(mic_id) = mic_node_id {
                            // Set physical mic volume
                            let mic_vol_str = format!("{:.2}", config.mic_volume);
                            let _ = tokio::process::Command::new("wpctl")
                                .arg("set-volume")
                                .arg(mic_id.to_string())
                                .arg(mic_vol_str)
                                .stderr(std::process::Stdio::null())
                                .status()
                                .await;

                            let mic_out_ports: Vec<&PortInfo> = ports.iter()
                                .filter(|p| p.node_id == mic_id && p.direction == "out")
                                .collect();

                            let desired = get_desired_links(&mic_out_ports, &vinput_in_ports);
                            for &(out_port, in_port) in &desired {
                                if !is_port_linked(&links, out_port, in_port) {
                                    run_pw_link(out_port, in_port).await;
                                }
                            }
                        }

                        // Link Target App to PBS Input
                        // NOTE: We do NOT call wpctl set-volume on the app node.
                        // That would change its system volume and alter what the user hears.
                        // We only create the routing link; the app plays normally through speakers too.
                        for app_id in app_node_ids {
                            let app_out_ports: Vec<&PortInfo> = ports.iter()
                                .filter(|p| p.node_id == app_id && p.direction == "out")
                                .collect();

                            let desired = get_desired_links(&app_out_ports, &vinput_in_ports);
                            for &(out_port, in_port) in &desired {
                                if !is_port_linked(&links, out_port, in_port) {
                                    run_pw_link(out_port, in_port).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Configure Web API routes
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/app.js", get(serve_js))
        .route("/api/status", get(get_status))
        .route("/api/settings", post(post_settings))
        .with_state(state);

    // Start system tray icon
    let service = ksni::TrayService::new(TrayIcon {});
    service.spawn();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9876")
        .await
        .unwrap();

    println!("PBS Soundpad dashboard started on http://localhost:9876");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("Received Ctrl+C, initiating shutdown...");
                }
                _ = async {
                    #[cfg(unix)]
                    {
                        if let Ok(mut sigterm) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                            sigterm.recv().await;
                            println!("Received SIGTERM, initiating shutdown...");
                        } else {
                            tokio::time::sleep(tokio::time::Duration::from_secs(999999)).await;
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        tokio::time::sleep(tokio::time::Duration::from_secs(999999)).await;
                    }
                } => {}
            }
        })
        .await
        .unwrap();

    println!("Cleaning up PBS Virtual Mic before exit...");
    let _ = std::process::Command::new("pw-cli")
        .arg("destroy")
        .arg("pbs_virtual_mic")
        .stderr(std::process::Stdio::null())
        .status();
}
