use crate::device::DeviceProfile;
use crate::proxy::model_detector::TargetModelCategory;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaBucketInfo {
    pub window: String,
    pub remaining_percentage: f64,
    pub reset_time: Option<String>,
}

impl QuotaBucketInfo {
    pub fn effective_percentage(&self) -> f64 {
        if let Some(ref reset_str) = self.reset_time {
            if let Ok(reset_dt) = chrono::DateTime::parse_from_rfc3339(reset_str) {
                if Utc::now() >= reset_dt.with_timezone(&Utc) {
                    return 100.0;
                }
            }
        }
        self.remaining_percentage.clamp(0.0, 100.0)
    }

    pub fn is_weekly(&self) -> bool {
        let w = self.window.to_uppercase();
        w.contains("WEEK") || w.contains("7D") || w.contains("SEVEN") || w.contains("7_DAY")
    }

    pub fn is_5h(&self) -> bool {
        let w = self.window.to_uppercase();
        w.contains("5H") || w.contains("FIVE") || w.contains("5_HOUR")
    }

    pub fn reset_countdown(&self) -> Option<String> {
        let reset_str = self.reset_time.as_ref()?;
        let now = Utc::now();
        let reset_dt = chrono::DateTime::parse_from_rfc3339(reset_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| reset_str.parse::<DateTime<Utc>>())
            .ok()?;

        let diff = reset_dt.signed_duration_since(now);
        let total_seconds = diff.num_seconds();
        if total_seconds <= 0 {
            return Some("đã đến giờ".to_string());
        }

        let total_minutes = ((total_seconds + 30) / 60).max(1);
        let days = total_minutes / (24 * 60);
        let hours = (total_minutes % (24 * 60)) / 60;
        let mins = total_minutes % 60;

        let formatted = if days > 0 {
            format!("~{}d", days)
        } else if hours > 0 {
            if mins > 0 {
                format!("~{}h {}m", hours, mins)
            } else {
                format!("~{}h", hours)
            }
        } else {
            format!("~{}m", mins)
        };
        Some(formatted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaGroupInfo {
    pub name: String,
    pub buckets: Vec<QuotaBucketInfo>,
}

impl QuotaGroupInfo {
    pub fn get_5h_bucket(&self) -> Option<&QuotaBucketInfo> {
        self.buckets
            .iter()
            .find(|b| b.is_5h())
            .or_else(|| self.buckets.iter().find(|b| !b.is_weekly()))
            .or_else(|| self.buckets.first())
    }

    #[allow(dead_code)]
    pub fn get_weekly_bucket(&self) -> Option<&QuotaBucketInfo> {
        self.buckets.iter().find(|b| b.is_weekly())
    }

    pub fn has_available_weekly_quota(&self) -> bool {
        let weekly_buckets: Vec<&QuotaBucketInfo> =
            self.buckets.iter().filter(|b| b.is_weekly()).collect();
        if weekly_buckets.is_empty() {
            true
        } else {
            weekly_buckets
                .iter()
                .all(|b| b.effective_percentage() > 0.0)
        }
    }

    pub fn calculate_score(&self, default_quota: f64) -> f64 {
        if !self.has_available_weekly_quota() {
            return 0.0;
        }

        if let Some(b5h) = self.get_5h_bucket() {
            return b5h.effective_percentage();
        }

        if let Some(first) = self.buckets.first() {
            return first.effective_percentage();
        }

        default_quota.clamp(0.0, 100.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub custom_label: Option<String>,
    pub device_profile: DeviceProfile,
    pub quota_percentage: f64,
    #[serde(default)]
    pub quota_checked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub quota_groups: Vec<QuotaGroupInfo>,
    pub is_active: bool,
    pub rate_limit_until: Option<DateTime<Utc>>,
}

impl Account {
    pub fn new(
        email: String,
        access_token: String,
        refresh_token: String,
        expires_in_seconds: i64,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in_seconds);
        let device_profile = DeviceProfile::generate();

        Self {
            id,
            email,
            access_token,
            refresh_token,
            expires_at,
            custom_label: None,
            device_profile,
            quota_percentage: 0.0,
            quota_checked_at: None,
            quota_groups: Vec::new(),
            is_active: true,
            rate_limit_until: None,
        }
    }

    #[allow(dead_code)]
    pub fn is_token_expired(&self) -> bool {
        Utc::now() >= self.expires_at - chrono::Duration::minutes(5)
    }

    pub fn is_rate_limited(&self) -> bool {
        if let Some(until) = self.rate_limit_until {
            Utc::now() < until
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn set_rate_limit(&mut self, duration_seconds: i64) {
        self.rate_limit_until = Some(Utc::now() + chrono::Duration::seconds(duration_seconds));
    }

    pub fn find_quota_group_for_category(
        &self,
        category: TargetModelCategory,
    ) -> Option<&QuotaGroupInfo> {
        match category {
            TargetModelCategory::Gemini => self
                .quota_groups
                .iter()
                .find(|g| g.name.to_lowercase().contains("gemini")),
            TargetModelCategory::ClaudeAndGpt => self
                .quota_groups
                .iter()
                .find(|g| {
                    let lower = g.name.to_lowercase();
                    lower.contains("claude") || lower.contains("gpt")
                })
                .or_else(|| {
                    self.quota_groups
                        .iter()
                        .find(|g| !g.name.to_lowercase().contains("gemini"))
                }),
        }
    }

    pub fn has_available_weekly_quota_for_category(&self, category: TargetModelCategory) -> bool {
        if let Some(group) = self.find_quota_group_for_category(category) {
            group.has_available_weekly_quota()
        } else if !self.quota_groups.is_empty() {
            true
        } else {
            self.quota_percentage > 0.0
        }
    }

    pub fn get_effective_quota_for_category(&self, category: TargetModelCategory) -> f64 {
        if !self.has_available_weekly_quota_for_category(category) {
            return 0.0;
        }
        match category {
            TargetModelCategory::Gemini => self.get_gemini_5h_quota(),
            TargetModelCategory::ClaudeAndGpt => self.get_claude_gpt_quota(),
        }
    }

    pub fn has_fresh_quota(&self) -> bool {
        self.quota_checked_at.is_some_and(|t| {
            let age = (Utc::now() - t).num_seconds();
            (0..600).contains(&age)
        }) && !self.quota_groups.iter().flat_map(|g| &g.buckets).any(|b| {
            b.reset_time.as_ref().is_some_and(|r| {
                chrono::DateTime::parse_from_rfc3339(r).is_ok_and(|t| t <= Utc::now())
            })
        })
    }

    pub fn get_gemini_5h_quota(&self) -> f64 {
        self.find_quota_group_for_category(TargetModelCategory::Gemini)
            .map(|g| g.calculate_score(self.quota_percentage))
            .unwrap_or(self.quota_percentage)
    }

    pub fn get_claude_gpt_quota(&self) -> f64 {
        self.find_quota_group_for_category(TargetModelCategory::ClaudeAndGpt)
            .map(|g| g.calculate_score(self.quota_percentage))
            .unwrap_or(self.quota_percentage)
    }

    #[allow(dead_code)]
    pub fn get_quota_for_group(&self, group_keyword: &str) -> f64 {
        let kw = group_keyword.to_lowercase();
        for g in &self.quota_groups {
            if g.name.to_lowercase().contains(&kw) {
                return g.calculate_score(self.quota_percentage);
            }
        }
        self.quota_percentage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_bucket_effective_percentage() {
        let bucket_no_reset = QuotaBucketInfo {
            window: "WEEKLY".to_string(),
            remaining_percentage: 45.0,
            reset_time: None,
        };
        assert_eq!(bucket_no_reset.effective_percentage(), 45.0);

        let future_time = (Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let bucket_future_reset = QuotaBucketInfo {
            window: "WEEKLY".to_string(),
            remaining_percentage: 0.0,
            reset_time: Some(future_time),
        };
        assert_eq!(bucket_future_reset.effective_percentage(), 0.0);

        let past_time = (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        let bucket_past_reset = QuotaBucketInfo {
            window: "WEEKLY".to_string(),
            remaining_percentage: 0.0,
            reset_time: Some(past_time),
        };
        assert_eq!(bucket_past_reset.effective_percentage(), 100.0);
    }

    #[test]
    fn test_quota_bucket_reset_countdown() {
        let bucket_none = QuotaBucketInfo {
            window: "FIVE_HOUR".to_string(),
            remaining_percentage: 0.0,
            reset_time: None,
        };
        assert_eq!(bucket_none.reset_countdown(), None);

        let past = (Utc::now() - chrono::Duration::seconds(10)).to_rfc3339();
        let bucket_past = QuotaBucketInfo {
            window: "FIVE_HOUR".to_string(),
            remaining_percentage: 0.0,
            reset_time: Some(past),
        };
        assert_eq!(bucket_past.reset_countdown(), Some("đã đến giờ".to_string()));

        let future_hours = (Utc::now() + chrono::Duration::minutes(135)).to_rfc3339();
        let bucket_hours = QuotaBucketInfo {
            window: "FIVE_HOUR".to_string(),
            remaining_percentage: 0.0,
            reset_time: Some(future_hours),
        };
        assert_eq!(bucket_hours.reset_countdown(), Some("~2h 15m".to_string()));

        let future_days = (Utc::now() + chrono::Duration::hours(150)).to_rfc3339();
        let bucket_days = QuotaBucketInfo {
            window: "WEEKLY".to_string(),
            remaining_percentage: 60.0,
            reset_time: Some(future_days),
        };
        assert_eq!(bucket_days.reset_countdown(), Some("~6d".to_string()));
    }

    #[test]
    fn test_weekly_quota_exhausted_disables_account_for_category() {
        let mut account = Account::new(
            "test@example.com".to_string(),
            "token".to_string(),
            "refresh".to_string(),
            3600,
        );

        account.quota_groups = vec![
            QuotaGroupInfo {
                name: "Gemini 2.5 Flash / Pro".to_string(),
                buckets: vec![
                    QuotaBucketInfo {
                        window: "FIVE_HOUR".to_string(),
                        remaining_percentage: 100.0,
                        reset_time: None,
                    },
                    QuotaBucketInfo {
                        window: "WEEKLY".to_string(),
                        remaining_percentage: 0.0,
                        reset_time: None,
                    },
                ],
            },
            QuotaGroupInfo {
                name: "Claude & GPT Models".to_string(),
                buckets: vec![
                    QuotaBucketInfo {
                        window: "FIVE_HOUR".to_string(),
                        remaining_percentage: 80.0,
                        reset_time: None,
                    },
                    QuotaBucketInfo {
                        window: "WEEKLY".to_string(),
                        remaining_percentage: 50.0,
                        reset_time: None,
                    },
                ],
            },
        ];

        // Gemini weekly quota is 0 -> exhausted
        assert!(!account.has_available_weekly_quota_for_category(TargetModelCategory::Gemini));
        assert_eq!(
            account.get_effective_quota_for_category(TargetModelCategory::Gemini),
            0.0
        );
        assert_eq!(account.get_gemini_5h_quota(), 0.0);

        // Claude weekly quota is 50% -> available with 80% 5h quota
        assert!(account.has_available_weekly_quota_for_category(TargetModelCategory::ClaudeAndGpt));
        assert_eq!(
            account.get_effective_quota_for_category(TargetModelCategory::ClaudeAndGpt),
            80.0
        );
        assert_eq!(account.get_claude_gpt_quota(), 80.0);
    }
}
