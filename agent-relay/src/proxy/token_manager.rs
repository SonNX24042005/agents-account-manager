use super::selection::{Agent, SelectionSettings};
use crate::models::Account;
use crate::proxy::model_detector::{ModelDetector, TargetModelCategory};
use crate::storage::{AccountStore, AccountSwitcher};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TokenManager {
    accounts: Arc<RwLock<Vec<Account>>>,
    store: Arc<AccountStore>,
    model_detector: Arc<ModelDetector>,
    pub settings: Arc<SelectionSettings>,
    switch_writer: fn(&Account) -> Result<()>,
    auth_path: Option<std::path::PathBuf>,
}

impl TokenManager {
    const MAX_ACCOUNTS: usize = 100;

    pub fn new(store: AccountStore, data_dir: std::path::PathBuf) -> Result<Self> {
        let store_arc = Arc::new(store);
        let loaded = store_arc.load_all().unwrap_or_default();
        tracing::info!(
            "[TokenManager] Loaded {} accounts into token pool",
            loaded.len()
        );

        Ok(Self {
            settings: Arc::new(SelectionSettings::new(&data_dir)?),
            switch_writer: AccountSwitcher::switch_active_account,
            auth_path: dirs::home_dir().map(|h| h.join(".antigravity/auth.json")),
            accounts: Arc::new(RwLock::new(loaded)),
            store: store_arc,
            model_detector: Arc::new(ModelDetector::new(data_dir)),
        })
    }

    pub async fn auto_select_if_enabled(&self) -> Result<()> {
        let flags = self.settings.flags.lock().await;
        if flags.enabled(Agent::Antigravity) {
            self.select_best_account_for_active_model().await?;
        }
        Ok(())
    }

    pub fn get_model_detector(&self) -> Arc<ModelDetector> {
        self.model_detector.clone()
    }

    pub async fn sync_active_account_from_disk(&self) {
        if self.auth_path.is_none() {
            return;
        }
        let mut disk_email = None;
        if let Some(auth_file) = &self.auth_path {
            if auth_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&auth_file) {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                        disk_email = json_val["email"].as_str().map(|s| s.to_string());
                    }
                }
            }
        }

        let mut list = self.accounts.write().await;
        for acc in list.iter_mut() {
            acc.is_active = disk_email.as_ref() == Some(&acc.email);
        }
    }

    pub async fn list_accounts(&self) -> Vec<Account> {
        self.sync_active_account_from_disk().await;
        self.accounts.read().await.clone()
    }

    pub async fn add_account(&self, mut account: Account) -> Result<()> {
        let mut list = self.accounts.write().await;
        let existing = list
            .iter()
            .find(|item| item.email == account.email)
            .cloned();
        anyhow::ensure!(
            existing.is_some() || list.len() < Self::MAX_ACCOUNTS,
            "Account limit reached ({})",
            Self::MAX_ACCOUNTS
        );

        account.is_active = existing.as_ref().is_some_and(|a| a.is_active);
        self.store.save(&account)?;
        if let Some(previous) = existing {
            if previous.id != account.id {
                self.store.delete(&previous.id)?;
            }
        }
        list.retain(|a| a.email != account.email);
        list.push(account.clone());
        tracing::info!("[TokenManager] Added/Updated account to pool");

        Ok(())
    }

    /// Automatically selects and switches to the account with the highest quota for the currently active model category
    pub async fn select_best_account_for_active_model(
        &self,
    ) -> Result<(Account, TargetModelCategory)> {
        self.sync_active_account_from_disk().await;
        let list = self.accounts.read().await.clone();
        if list.is_empty() {
            return Err(anyhow!("No accounts in pool"));
        }

        let target_category = self.model_detector.get_effective_category();

        let eligible: Vec<Account> = list
            .into_iter()
            .filter(|a| {
                a.has_fresh_quota()
                    && !a.is_rate_limited()
                    && a.has_available_weekly_quota_for_category(target_category)
                    && a.get_effective_quota_for_category(target_category) > 0.0
            })
            .collect();

        if eligible.is_empty() {
            return Err(anyhow!(
                "No eligible accounts available with remaining weekly quota for {}",
                target_category.display_name()
            ));
        }

        let best = eligible
            .into_iter()
            .max_by(|a, b| {
                let score_a = a.get_effective_quota_for_category(target_category);
                let score_b = b.get_effective_quota_for_category(target_category);

                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.is_active.cmp(&b.is_active))
            })
            .ok_or_else(|| anyhow!("Failed to select best account"))?;

        if !best.is_active {
            self.switch_account(&best.id).await?;
        }
        tracing::info!(
            "[TokenManager] Auto-selected account {} for category {:?} (score: {:.1}%)",
            best.email,
            target_category,
            best.get_effective_quota_for_category(target_category)
        );

        Ok((best, target_category))
    }

    #[allow(dead_code)]
    pub async fn select_highest_gemini_account(&self) -> Result<Account> {
        let (acc, _) = self.select_best_account_for_active_model().await?;
        Ok(acc)
    }

    pub async fn switch_account(&self, target_id_or_email: &str) -> Result<Account> {
        let mut list = self.accounts.write().await;
        let target = list
            .iter()
            .find(|a| a.id == target_id_or_email || a.email == target_id_or_email)
            .cloned()
            .ok_or_else(|| anyhow!("Account not found"))?;
        (self.switch_writer)(&target)?;
        for account in list.iter_mut() {
            account.is_active = account.id == target.id;
            self.store.save(account)?;
        }
        Ok(list.iter().find(|a| a.id == target.id).unwrap().clone())
    }

    pub async fn delete_account(&self, target_id_or_email: &str) -> Result<String> {
        let removed_account = {
            let mut list = self.accounts.write().await;
            let pos = list
                .iter()
                .position(|a| a.id == target_id_or_email || a.email == target_id_or_email);
            if let Some(index) = pos {
                let removed = list[index].clone();
                self.store.delete(&removed.id)?;
                list.remove(index);
                tracing::info!("[TokenManager] Deleted account: {}", removed.email);
                Some(removed)
            } else {
                None
            }
        };

        if let Some(removed) = removed_account {
            if removed.is_active {
                let _ = self.auto_select_if_enabled().await;
            }
            Ok(removed.email)
        } else {
            Err(anyhow!("Account not found: {}", target_id_or_email))
        }
    }

    pub async fn select_best_account(&self) -> Result<Account> {
        self.sync_active_account_from_disk().await;
        let flags = self.settings.flags.lock().await;
        let list = self.accounts.read().await;
        let category = self.model_detector.get_effective_category();
        let eligible = |a: &&Account| {
            a.has_fresh_quota()
                && !a.is_rate_limited()
                && a.has_available_weekly_quota_for_category(category)
                && a.get_effective_quota_for_category(category) > 0.0
        };
        if !flags.enabled(Agent::Antigravity) {
            return list
                .iter()
                .filter(eligible)
                .find(|a| a.is_active)
                .cloned()
                .ok_or_else(|| {
                    anyhow!("Tài khoản đã chọn không khả dụng; chế độ tự động đang tắt")
                });
        }
        list.iter()
            .filter(eligible)
            .max_by(|a, b| {
                a.get_effective_quota_for_category(category)
                    .partial_cmp(&b.get_effective_quota_for_category(category))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.is_active.cmp(&b.is_active))
            })
            .cloned()
            .ok_or_else(|| anyhow!("Không có tài khoản còn quota"))
    }

    #[allow(dead_code)]
    pub async fn mark_rate_limited(&self, email: &str, cooldown_seconds: i64) {
        let mut list = self.accounts.write().await;
        if let Some(account) = list.iter_mut().find(|a| a.email == email) {
            account.set_rate_limit(cooldown_seconds);
            account.quota_percentage = 0.0;
            tracing::warn!(
                "[CircuitBreaker] Account {} rate limited (429/403). Cooldown set for {}s",
                email,
                cooldown_seconds
            );
            let _ = self.store.save(account);
        }
    }

    pub async fn reset_cooldowns(&self) {
        let mut list = self.accounts.write().await;
        for acc in list.iter_mut() {
            acc.rate_limit_until = None;
            let _ = self.store.save(acc);
        }
    }

    pub async fn refresh_quotas(&self, client: &reqwest::Client) {
        let list = self.accounts.read().await.clone();
        for mut account in list {
            account.quota_checked_at = None;
            // 1. Auto-refresh OAuth token if expired
            let mut token_refreshed = false;
            if account.is_token_expired() && !account.refresh_token.is_empty() {
                match crate::oauth::GoogleOAuth::refresh_access_token(
                    client,
                    &account.refresh_token,
                )
                .await
                {
                    Ok(token_resp) => {
                        account.access_token = token_resp.access_token;
                        if let Some(new_refresh) = token_resp.refresh_token {
                            account.refresh_token = new_refresh;
                        }
                        let expires_in = token_resp.expires_in.unwrap_or(3600);
                        account.expires_at =
                            chrono::Utc::now() + chrono::Duration::seconds(expires_in);
                        token_refreshed = true;
                        tracing::info!(
                            "[TokenManager] Successfully refreshed token for {}",
                            account.email
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            "[TokenManager] Failed to refresh token for {}: {}",
                            account.email,
                            error
                        );
                    }
                }
            }

            // 2. Fetch latest quota from Google CloudCode PA API
            let (overall_pct, groups) =
                crate::proxy::quota::QuotaFetcher::fetch_account_quota_full(
                    client,
                    &account.access_token,
                )
                .await;

            // If quota fetch failed and token was not already refreshed, attempt a fallback token refresh
            if overall_pct.is_none()
                && groups.is_empty()
                && !token_refreshed
                && !account.refresh_token.is_empty()
            {
                if let Ok(token_resp) =
                    crate::oauth::GoogleOAuth::refresh_access_token(client, &account.refresh_token)
                        .await
                {
                    account.access_token = token_resp.access_token;
                    if let Some(new_refresh) = token_resp.refresh_token {
                        account.refresh_token = new_refresh;
                    }
                    let expires_in = token_resp.expires_in.unwrap_or(3600);
                    account.expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
                    token_refreshed = true;
                    tracing::info!(
                        "[TokenManager] Fallback token refresh succeeded for {}",
                        account.email
                    );
                    let (retry_pct, retry_groups) =
                        crate::proxy::quota::QuotaFetcher::fetch_account_quota_full(
                            client,
                            &account.access_token,
                        )
                        .await;
                    if retry_pct.is_some() || !retry_groups.is_empty() {
                        account.quota_checked_at = Some(chrono::Utc::now());
                    }
                    if let Some(pct) = retry_pct {
                        account.quota_percentage = pct;
                    }
                    if !retry_groups.is_empty() {
                        account.quota_groups = retry_groups;
                    }
                }
            } else {
                if overall_pct.is_some() || !groups.is_empty() {
                    account.quota_checked_at = Some(chrono::Utc::now());
                }
                if let Some(pct) = overall_pct {
                    account.quota_percentage = pct;
                }
                if !groups.is_empty() {
                    account.quota_groups = groups.clone();
                }
            }

            // Record quota delta for intelligent usage detection
            self.model_detector.record_quota_delta(
                &account.id,
                account.get_gemini_5h_quota(),
                account.get_claude_gpt_quota(),
            );

            let mut write_list = self.accounts.write().await;
            if let Some(acc) = write_list.iter_mut().find(|a| a.id == account.id) {
                account.is_active = acc.is_active;
                account.rate_limit_until = acc.rate_limit_until;
                self.store
                    .save(&account)
                    .unwrap_or_else(|e| tracing::warn!("Không lưu được quota: {e}"));
                *acc = account.clone();
                if token_refreshed && acc.is_active {
                    let _ = (self.switch_writer)(acc);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::account::{QuotaBucketInfo, QuotaGroupInfo};

    #[tokio::test]
    async fn test_select_best_account_skips_exhausted_weekly_quota() {
        let dir = std::env::temp_dir().join(format!("aam-tm-test-{}", uuid::Uuid::new_v4()));
        let store = AccountStore::new(dir.clone());
        let mut tm = TokenManager::new(store, dir.clone()).unwrap();
        tm.switch_writer = |_| Ok(());
        tm.auth_path = None;
        tm.model_detector
            .set_preference(crate::proxy::model_detector::RoutingPreference::GeminiOnly);

        // Account 1: 100% 5h quota, but 0% weekly quota
        let mut acc1 = Account::new(
            "acc1@example.com".to_string(),
            "tok1".to_string(),
            "ref1".to_string(),
            3600,
        );
        acc1.quota_groups = vec![QuotaGroupInfo {
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
        }];

        // Account 2: 70% 5h quota, 50% weekly quota
        let mut acc2 = Account::new(
            "acc2@example.com".to_string(),
            "tok2".to_string(),
            "ref2".to_string(),
            3600,
        );
        acc2.quota_groups = vec![QuotaGroupInfo {
            name: "Gemini 2.5 Flash / Pro".to_string(),
            buckets: vec![
                QuotaBucketInfo {
                    window: "FIVE_HOUR".to_string(),
                    remaining_percentage: 70.0,
                    reset_time: None,
                },
                QuotaBucketInfo {
                    window: "WEEKLY".to_string(),
                    remaining_percentage: 50.0,
                    reset_time: None,
                },
            ],
        }];

        acc1.quota_checked_at = Some(chrono::Utc::now());
        acc2.quota_checked_at = Some(chrono::Utc::now());
        tm.add_account(acc1).await.unwrap();
        tm.add_account(acc2).await.unwrap();

        let (best, _category) = tm.select_best_account_for_active_model().await.unwrap();
        assert_eq!(best.email, "acc2@example.com");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn test_select_best_account_fails_when_all_weekly_quotas_exhausted() {
        let dir = std::env::temp_dir().join(format!("aam-tm-test-{}", uuid::Uuid::new_v4()));
        let store = AccountStore::new(dir.clone());
        let mut tm = TokenManager::new(store, dir.clone()).unwrap();
        tm.switch_writer = |_| Ok(());
        tm.auth_path = None;
        tm.model_detector
            .set_preference(crate::proxy::model_detector::RoutingPreference::GeminiOnly);

        let mut acc = Account::new(
            "acc@example.com".to_string(),
            "tok".to_string(),
            "ref".to_string(),
            3600,
        );
        acc.quota_groups = vec![QuotaGroupInfo {
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
        }];

        acc.quota_checked_at = Some(chrono::Utc::now());
        tm.add_account(acc).await.unwrap();

        let res = tm.select_best_account_for_active_model().await;
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(dir);
    }
    #[tokio::test]
    async fn manual_mode_never_falls_back_and_invalid_switch_keeps_active() {
        let dir = std::env::temp_dir().join(format!("aam-manual-{}", uuid::Uuid::new_v4()));
        let mut tm =
            TokenManager::new(AccountStore::new(dir.join("accounts")), dir.clone()).unwrap();
        tm.switch_writer = |_| Ok(());
        tm.auth_path = None;
        let mut low = Account::new("low@example.com".into(), "low".into(), "".into(), 3600);
        low.quota_percentage = 20.0;
        let mut high = Account::new("high@example.com".into(), "high".into(), "".into(), 3600);
        high.quota_percentage = 90.0;
        low.quota_checked_at = Some(chrono::Utc::now());
        high.quota_checked_at = Some(chrono::Utc::now());
        tm.add_account(low.clone()).await.unwrap();
        tm.add_account(high.clone()).await.unwrap();
        tm.settings.set(Agent::Antigravity, false).await.unwrap();
        tm.switch_account(&low.id).await.unwrap();
        tm.auto_select_if_enabled().await.unwrap();
        assert_eq!(tm.select_best_account().await.unwrap().id, low.id);
        assert!(tm.switch_account("missing").await.is_err());
        assert_eq!(tm.select_best_account().await.unwrap().id, low.id);
        tm.mark_rate_limited(&low.email, 60).await;
        assert!(tm.select_best_account().await.is_err());
        tm.settings.set(Agent::Antigravity, true).await.unwrap();
        assert_eq!(tm.select_best_account().await.unwrap().id, high.id);
        tm.settings.set(Agent::Antigravity, false).await.unwrap();
        tm.delete_account(&low.id).await.unwrap();
        assert!(tm.select_best_account().await.is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[tokio::test]
    async fn missing_native_auth_clears_active_state() {
        let dir = std::env::temp_dir().join(format!("aam-auth-sync-{}", uuid::Uuid::new_v4()));
        let mut tm =
            TokenManager::new(AccountStore::new(dir.join("accounts")), dir.clone()).unwrap();
        tm.switch_writer = |_| Ok(());
        tm.auth_path = Some(dir.join("auth.json"));
        let account = Account::new("demo@example.com".into(), "fixture".into(), "".into(), 3600);
        tm.add_account(account.clone()).await.unwrap();
        tm.switch_account(&account.id).await.unwrap();
        assert!(!tm.list_accounts().await[0].is_active);
        std::fs::write(dir.join("auth.json"), r#"{"email":"demo@example.com"}"#).unwrap();
        assert!(tm.list_accounts().await[0].is_active);
        std::fs::remove_file(dir.join("auth.json")).unwrap();
        assert!(!tm.list_accounts().await[0].is_active);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
