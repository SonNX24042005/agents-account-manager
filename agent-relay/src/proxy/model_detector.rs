use crate::storage::secure_file;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetModelCategory {
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "claude_gpt")]
    ClaudeAndGpt,
}

impl TargetModelCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Gemini => "Gemini Models",
            Self::ClaudeAndGpt => "Claude & GPT Models (Other)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingPreference {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "gemini")]
    GeminiOnly,
    #[serde(rename = "claude_gpt")]
    ClaudeGptOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingState {
    pub preference: RoutingPreference,
    pub detected_category: TargetModelCategory,
    pub last_detected_source: String,
    pub last_updated_at: i64,
}

pub struct ModelDetector {
    state_file: PathBuf,
    history: Arc<Mutex<HashMap<String, (f64, f64, i64)>>>, // account_id -> (prev_gemini, prev_claude, timestamp)
    cached_state: Arc<Mutex<ModelRoutingState>>,
}

impl ModelDetector {
    pub fn new(base_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&base_dir);
        let state_file = base_dir.join("routing_preference.json");

        let default_state = ModelRoutingState {
            preference: RoutingPreference::Auto,
            detected_category: TargetModelCategory::Gemini,
            last_detected_source: "Mặc định (Default)".to_string(),
            last_updated_at: chrono::Utc::now().timestamp(),
        };

        let initial_state = if state_file.exists() {
            fs::read_to_string(&state_file)
                .ok()
                .and_then(|s| serde_json::from_str::<ModelRoutingState>(&s).ok())
                .unwrap_or(default_state)
        } else {
            default_state
        };

        Self {
            state_file,
            history: Arc::new(Mutex::new(HashMap::new())),
            cached_state: Arc::new(Mutex::new(initial_state)),
        }
    }

    pub fn get_state(&self) -> ModelRoutingState {
        let mut state = self.cached_state.lock().unwrap().clone();
        if state.preference == RoutingPreference::Auto {
            // Perform real-time transcript scan if in Auto mode
            if let Some((cat, source)) = Self::scan_recent_transcripts() {
                state.detected_category = cat;
                state.last_detected_source = source;
                state.last_updated_at = chrono::Utc::now().timestamp();
                *self.cached_state.lock().unwrap() = state.clone();
            }
        }
        state
    }

    pub fn get_effective_category(&self) -> TargetModelCategory {
        let state = self.get_state();
        match state.preference {
            RoutingPreference::GeminiOnly => TargetModelCategory::Gemini,
            RoutingPreference::ClaudeGptOnly => TargetModelCategory::ClaudeAndGpt,
            RoutingPreference::Auto => state.detected_category,
        }
    }

    pub fn set_preference(&self, pref: RoutingPreference) -> ModelRoutingState {
        let mut state = self.cached_state.lock().unwrap();
        state.preference = pref;
        state.last_updated_at = chrono::Utc::now().timestamp();

        let to_save = state.clone();
        if let Ok(json_str) = serde_json::to_string_pretty(&to_save) {
            let _ = secure_file::atomic_write(&self.state_file, json_str.as_bytes(), 0o600);
        }
        to_save
    }

    /// Record quota change and detect consumption delta
    pub fn record_quota_delta(&self, account_id: &str, current_gemini: f64, current_claude: f64) {
        let mut hist = self.history.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        if let Some(&(prev_gemini, prev_claude, _ts)) = hist.get(account_id) {
            let gemini_consumed = prev_gemini - current_gemini;
            let claude_consumed = prev_claude - current_claude;

            if claude_consumed > 0.5 && claude_consumed > gemini_consumed {
                self.update_detected_category(
                    TargetModelCategory::ClaudeAndGpt,
                    format!(
                        "Phát hiện tiêu hao hạn ngạch Claude/GPT (-{:.1}%)",
                        claude_consumed
                    ),
                );
            } else if gemini_consumed > 0.5 && gemini_consumed >= claude_consumed {
                self.update_detected_category(
                    TargetModelCategory::Gemini,
                    format!(
                        "Phát hiện tiêu hao hạn ngạch Gemini (-{:.1}%)",
                        gemini_consumed
                    ),
                );
            }
        }

        hist.insert(
            account_id.to_string(),
            (current_gemini, current_claude, now),
        );
    }

    fn update_detected_category(&self, cat: TargetModelCategory, source: String) {
        let mut state = self.cached_state.lock().unwrap();
        state.detected_category = cat;
        state.last_detected_source = source;
        state.last_updated_at = chrono::Utc::now().timestamp();

        let to_save = state.clone();
        if let Ok(json_str) = serde_json::to_string_pretty(&to_save) {
            let _ = secure_file::atomic_write(&self.state_file, json_str.as_bytes(), 0o600);
        }
    }

    /// Scan recent transcript.jsonl files in ~/.gemini/antigravity-cli/brain/
    fn scan_recent_transcripts() -> Option<(TargetModelCategory, String)> {
        let home = dirs::home_dir()?;
        let brain_dir = home.join(".gemini").join("antigravity-cli").join("brain");
        if !brain_dir.exists() {
            return None;
        }

        // Find most recently modified transcript.jsonl
        let mut latest_file = None;
        let mut latest_mtime = std::time::SystemTime::UNIX_EPOCH;

        if let Ok(entries) = fs::read_dir(&brain_dir) {
            for entry in entries.flatten() {
                let log_file = entry
                    .path()
                    .join(".system_generated")
                    .join("logs")
                    .join("transcript.jsonl");
                if log_file.exists() {
                    if let Ok(meta) = log_file.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            if mtime > latest_mtime {
                                latest_mtime = mtime;
                                latest_file = Some(log_file);
                            }
                        }
                    }
                }
            }
        }

        let target_log = latest_file?;
        use std::io::{Read, Seek, SeekFrom};
        let mut file = fs::File::open(target_log).ok()?;
        let meta = file.metadata().ok()?;
        let file_len = meta.len();
        let read_size = file_len.min(32 * 1024); // read up to last 32KB
        let offset = file_len - read_size;
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut buffer = vec![0u8; read_size as usize];
        file.read_exact(&mut buffer).ok()?;
        let content = String::from_utf8_lossy(&buffer);

        // Check the last 100 lines
        let lines: Vec<&str> = content.lines().collect();
        let scan_lines = if lines.len() > 100 {
            &lines[lines.len() - 100..]
        } else {
            &lines[..]
        };

        for line in scan_lines.iter().rev() {
            let lower = line.to_lowercase();
            if lower.contains("claude")
                || lower.contains("sonnet")
                || lower.contains("haiku")
                || lower.contains("opus")
                || lower.contains("gpt-4")
                || lower.contains("gpt-o")
            {
                return Some((
                    TargetModelCategory::ClaudeAndGpt,
                    "Phát hiện qua phiên hội thoại gần nhất (Claude / GPT)".to_string(),
                ));
            }
            if lower.contains("gemini") || lower.contains("flash") || lower.contains("pro") {
                return Some((
                    TargetModelCategory::Gemini,
                    "Phát hiện qua phiên hội thoại gần nhất (Gemini)".to_string(),
                ));
            }
        }

        None
    }
}
