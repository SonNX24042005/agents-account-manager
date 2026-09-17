use crate::models::Account;
use crate::proxy::selection::Agent;
use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde_json::json;

pub struct WarmupService;

impl WarmupService {
    /// Kiểm tra tài khoản Antigravity có đủ điều kiện kích hoạt đếm ngược 5 giờ hay không:
    /// - Hạn ngạch 5 giờ đang ở mức 100%
    /// - Hạn ngạch tuần vẫn còn (> 0%)
    /// - Chưa từng được kích hoạt trong vòng 5 giờ qua
    pub fn is_antigravity_eligible(account: &Account, now: DateTime<Utc>) -> bool {
        // Hạn ngạch tuần phải còn (> 0%)
        if !account.has_available_weekly_quota_for_category(
            crate::proxy::model_detector::TargetModelCategory::Gemini,
        ) {
            return false;
        }

        // Hạn ngạch 5 giờ phải đạt 100%
        let five_h_quota = account.get_gemini_5h_quota();
        if five_h_quota < 100.0 {
            return false;
        }

        // Chưa kích hoạt trong vòng 5 giờ qua
        if let Some(last_warmup) = account.last_warmup_at {
            if now - last_warmup < Duration::hours(5) {
                return false;
            }
        }

        true
    }

    /// Gửi câu hỏi đơn giản qua API Google Cloud Code để kích hoạt cửa sổ 5 giờ bắt đầu đếm ngược.
    pub async fn warmup_antigravity(
        client: &Client,
        access_token: &str,
        email: &str,
    ) -> Result<()> {
        let endpoints = [
            "https://daily-cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse",
            "https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse",
        ];

        let payload = json!({
            "project": "aicode-consumers",
            "model": "gemini-3.6-flash-high",
            "request": {
                "contents": [
                    {
                        "role": "user",
                        "parts": [{"text": "ping"}]
                    }
                ]
            }
        });

        for url in endpoints {
            let res = client
                .post(url)
                .bearer_auth(access_token)
                .header("Content-Type", "application/json")
                .header("User-Agent", "Antigravity/1.0.0")
                .timeout(std::time::Duration::from_secs(15))
                .json(&payload)
                .send()
                .await;

            if let Ok(resp) = res {
                if resp.status().is_success() {
                    tracing::info!(
                        "[Warmup] Đã kích hoạt bộ đếm 5 giờ cho tài khoản Antigravity ({}) qua mô hình gemini-3.6-flash-high",
                        email
                    );
                    return Ok(());
                }
            }
        }

        bail!("Không thể gửi yêu cầu kích hoạt 5 giờ cho tài khoản Antigravity: {}", email)
    }

    /// Kiểm tra tài khoản Codex hoặc Claude có đủ điều kiện kích hoạt đếm ngược 5 giờ hay không:
    pub fn is_native_eligible(
        _agent: Agent,
        quota_groups: &[crate::models::account::QuotaGroupInfo],
        last_warmup_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> bool {
        let mut five_h_pct = None;
        let mut weekly_pct = None;

        for group in quota_groups {
            for bucket in &group.buckets {
                if bucket.is_5h() {
                    five_h_pct = Some(bucket.effective_percentage());
                } else if bucket.is_weekly() {
                    weekly_pct = Some(bucket.effective_percentage());
                }
            }
        }

        // Hạn ngạch 5 giờ phải đạt 100%
        let Some(five_h) = five_h_pct else {
            return false;
        };
        if five_h < 100.0 {
            return false;
        }

        // Hạn ngạch tuần phải còn (> 0%)
        let Some(weekly) = weekly_pct else {
            return false;
        };
        if weekly <= 0.0 {
            return false;
        }

        // Chưa kích hoạt trong vòng 5 giờ qua
        if let Some(last_warmup) = last_warmup_at {
            if now - last_warmup < Duration::hours(5) {
                return false;
            }
        }

        true
    }

    /// Gửi câu hỏi đơn giản qua API ChatGPT Codex để kích hoạt cửa sổ 5 giờ bắt đầu đếm ngược.
    pub async fn warmup_codex(client: &Client, access_token: &str, email: &str) -> Result<()> {
        let url = "https://chatgpt.com/backend-api/codex/responses";
        let payload = json!({
            "model": "gpt-5.6-luna",
            "input": [{"role": "user", "content": "ping"}],
            "store": false,
            "stream": true
        });

        let res = client
            .post(url)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("User-Agent", "codex-cli/0.154.0")
            .timeout(std::time::Duration::from_secs(10))
            .json(&payload)
            .send()
            .await?;

        if res.status().is_success() {
            // Đọc trọn vẹn luồng dữ liệu phản hồi để máy chủ OpenAI hoàn tất suy luận và ghi nhận điểm neo hạn ngạch
            let _ = tokio::time::timeout(std::time::Duration::from_secs(15), res.bytes()).await;
            tracing::info!(
                "[Warmup] Đã kích hoạt bộ đếm 5 giờ cho tài khoản Codex ({}) qua mô hình gpt-5.6-luna",
                email
            );
            Ok(())
        } else {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            bail!("Codex warmup failed ({}): {}", status, body)
        }
    }

    /// Gửi câu hỏi đơn giản qua API Anthropic Messages để kích hoạt cửa sổ 5 giờ bắt đầu đếm ngược.
    pub async fn warmup_claude(client: &Client, access_token: &str, email: &str) -> Result<()> {
        let url = "https://api.anthropic.com/v1/messages";
        let payload = json!({
            "model": "claude-3-5-haiku-20241022",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "ping"}]
        });

        let res = client
            .post(url)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "oauth-2025-04-20")
            .timeout(std::time::Duration::from_secs(10))
            .json(&payload)
            .send()
            .await?;

        if res.status().is_success() {
            tracing::info!(
                "[Warmup] Đã kích hoạt bộ đếm 5 giờ cho tài khoản Claude ({}) qua mô hình claude-3-5-haiku",
                email
            );
            Ok(())
        } else {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            bail!("Claude warmup failed ({}): {}", status, body)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::account::{QuotaBucketInfo, QuotaGroupInfo};

    #[test]
    fn test_warmup_eligibility() {
        let now = Utc::now();
        let groups = vec![QuotaGroupInfo {
            name: "Codex".into(),
            buckets: vec![
                QuotaBucketInfo {
                    window: "FIVE_HOUR".into(),
                    remaining_percentage: 100.0,
                    reset_time: None,
                },
                QuotaBucketInfo {
                    window: "WEEKLY".into(),
                    remaining_percentage: 50.0,
                    reset_time: None,
                },
            ],
        }];

        // Eligible when fresh
        assert!(WarmupService::is_native_eligible(Agent::Codex, &groups, None, now));

        // Ineligible when warmed up 1 hour ago
        assert!(!WarmupService::is_native_eligible(
            Agent::Codex,
            &groups,
            Some(now - Duration::hours(1)),
            now
        ));

        // Eligible when warmed up 6 hours ago
        assert!(WarmupService::is_native_eligible(
            Agent::Codex,
            &groups,
            Some(now - Duration::hours(6)),
            now
        ));

        // Ineligible when 5h quota is below 100%
        let mut depleted_5h = groups.clone();
        depleted_5h[0].buckets[0].remaining_percentage = 90.0;
        assert!(!WarmupService::is_native_eligible(Agent::Codex, &depleted_5h, None, now));

        // Ineligible when weekly quota is 0%
        let mut depleted_weekly = groups.clone();
        depleted_weekly[0].buckets[1].remaining_percentage = 0.0;
        assert!(!WarmupService::is_native_eligible(Agent::Codex, &depleted_weekly, None, now));
    }

    #[test]
    fn test_antigravity_warmup_eligibility() {
        let now = Utc::now();
        let mut acc = Account::new(
            "test@example.com".into(),
            "tok".into(),
            "ref".into(),
            3600,
        );
        acc.quota_groups = vec![QuotaGroupInfo {
            name: "Gemini Models".into(),
            buckets: vec![
                QuotaBucketInfo {
                    window: "5h".into(),
                    remaining_percentage: 100.0,
                    reset_time: None,
                },
                QuotaBucketInfo {
                    window: "weekly".into(),
                    remaining_percentage: 80.0,
                    reset_time: None,
                },
            ],
        }];

        // Eligible when fresh
        assert!(WarmupService::is_antigravity_eligible(&acc, now));

        // Ineligible when warmed up 2 hours ago
        acc.last_warmup_at = Some(now - Duration::hours(2));
        assert!(!WarmupService::is_antigravity_eligible(&acc, now));

        // Eligible when warmed up 5.5 hours ago
        acc.last_warmup_at = Some(now - Duration::hours(5) - Duration::minutes(30));
        assert!(WarmupService::is_antigravity_eligible(&acc, now));

        // Ineligible when 5h quota is below 100%
        acc.last_warmup_at = None;
        acc.quota_groups[0].buckets[0].remaining_percentage = 95.0;
        assert!(!WarmupService::is_antigravity_eligible(&acc, now));

        // Ineligible when weekly quota is 0%
        acc.quota_groups[0].buckets[0].remaining_percentage = 100.0;
        acc.quota_groups[0].buckets[1].remaining_percentage = 0.0;
        assert!(!WarmupService::is_antigravity_eligible(&acc, now));
    }
}
