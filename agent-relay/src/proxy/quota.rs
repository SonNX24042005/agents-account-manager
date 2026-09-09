use crate::models::account::{QuotaBucketInfo, QuotaGroupInfo};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;

#[allow(dead_code)]
pub struct QuotaFetcher;

#[allow(dead_code)]
impl QuotaFetcher {
    pub async fn fetch_account_quota_full(
        client: &Client,
        access_token: &str,
    ) -> (Option<f64>, Vec<QuotaGroupInfo>) {
        let overall_pct = Self::fetch_overall_quota(client, access_token).await;
        let groups = Self::fetch_quota_groups(client, access_token).await;
        (overall_pct, groups)
    }

    async fn fetch_overall_quota(client: &Client, access_token: &str) -> Option<f64> {
        let ep_urls = [
            "https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:fetchAvailableModels",
            "https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels",
            "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels",
        ];

        for url in ep_urls {
            let res = client
                .post(url)
                .bearer_auth(access_token)
                .header("User-Agent", "Antigravity/1.0.0")
                .json(&serde_json::json!({}))
                .send()
                .await;

            if let Ok(resp) = res {
                if resp.status().is_success() {
                    if let Some(json_data) = Self::read_limited_json(resp, 1024 * 1024).await {
                        if let Some(models) = json_data["models"].as_object() {
                            let mut min_percentage: Option<f64> = None;

                            for (_model_id, info) in models {
                                if let Some(fraction) =
                                    info["quotaInfo"]["remainingFraction"].as_f64()
                                {
                                    let pct = (fraction * 100.0).round();
                                    min_percentage =
                                        Some(min_percentage.map_or(pct, |m| m.min(pct)));
                                } else if let Some(pct) =
                                    info["quotaInfo"]["remainingPercentage"].as_f64()
                                {
                                    min_percentage =
                                        Some(min_percentage.map_or(pct, |m| m.min(pct)));
                                }
                            }

                            if let Some(pct) = min_percentage {
                                return Some(pct);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    async fn fetch_quota_groups(client: &Client, access_token: &str) -> Vec<QuotaGroupInfo> {
        let ep_urls = [
            "https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:retrieveUserQuotaSummary",
            "https://daily-cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
            "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
        ];

        for url in ep_urls {
            let res = client
                .post(url)
                .bearer_auth(access_token)
                .header("User-Agent", "Antigravity/1.0.0")
                .json(&serde_json::json!({}))
                .send()
                .await;

            if let Ok(resp) = res {
                if resp.status().is_success() {
                    if let Some(json_data) = Self::read_limited_json(resp, 1024 * 1024).await {
                        if let Some(groups) = json_data["groups"].as_array() {
                            let mut result = Vec::new();

                            for group in groups {
                                let name = group["displayName"]
                                    .as_str()
                                    .unwrap_or("Model Group")
                                    .to_string();

                                let mut buckets = Vec::new();
                                if let Some(b_arr) = group["buckets"].as_array() {
                                    for b in b_arr {
                                        let window =
                                            b["window"].as_str().unwrap_or("WINDOW").to_string();
                                        let frac = b["remainingFraction"].as_f64().unwrap_or(0.0);
                                        let reset_time =
                                            b["resetTime"].as_str().map(|s| s.to_string());

                                        buckets.push(QuotaBucketInfo {
                                            window,
                                            remaining_percentage: (frac * 100.0).round(),
                                            reset_time,
                                        });
                                    }
                                }

                                result.push(QuotaGroupInfo { name, buckets });
                            }

                            if !result.is_empty() {
                                return result;
                            }
                        }
                    }
                }
            }
        }

        Vec::new()
    }

    async fn read_limited_json(response: reqwest::Response, max_bytes: usize) -> Option<Value> {
        if response
            .content_length()
            .is_some_and(|length| length > max_bytes as u64)
        {
            return None;
        }

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.ok()?;
            if body.len().saturating_add(chunk.len()) > max_bytes {
                return None;
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).ok()
    }
}
