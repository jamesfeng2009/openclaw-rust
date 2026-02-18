//! OAuth 认证管理器
//!
//! 支持：
//! - Anthropic OAuth (Claude Pro/Max)
//! - OpenAI OAuth (ChatGPT/Codex)
//! - Google OAuth
//! - Azure OAuth
//! - 阿里云 (Qwen/通义千问)
//! - 字节跳动 (Doubao/豆包)
//! - Minimax
//! - 智谱 AI (GLM)
//! - 月之暗面 (Kimi/Moonshot)
//! - Token 刷新和过期处理
//! - Auth Profile 轮换

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info};

use openclaw_core::config::AuthProfile;

pub type Result<T> = std::result::Result<T, OAuthError>;

/// OAuth 提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProvider {
    Anthropic,
    OpenAI,
    Google,
    Azure,
    DeepSeek,
    Qwen,
    Doubao,
    Minimax,
    Glm,
    Kimi,
}

/// OAuth 令牌响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// OAuth 错误
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("认证失败: {0}")]
    AuthFailed(String),

    #[error("Token 刷新失败: {0}")]
    RefreshFailed(String),

    #[error("Token 已过期")]
    TokenExpired,

    #[error("不支持的 OAuth 提供商: {0}")]
    UnsupportedProvider(String),

    #[error("HTTP 错误: {0}")]
    HttpError(String),
}

/// OAuth 认证管理器
pub struct OAuthManager {
    provider: OAuthProvider,
    client_id: String,
    client_secret: String,
    redirect_uri: Option<String>,
    current_token: RwLock<Option<OAuthToken>>,
    custom_authorization_url: Option<String>,
    custom_token_url: Option<String>,
    custom_scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scope: Vec<String>,
}

impl OAuthManager {
    pub fn new(provider: OAuthProvider, client_id: String, client_secret: String) -> Self {
        Self {
            provider,
            client_id,
            client_secret,
            redirect_uri: None,
            current_token: RwLock::new(None),
            custom_authorization_url: None,
            custom_token_url: None,
            custom_scopes: None,
        }
    }

    pub fn with_redirect_uri(mut self, redirect_uri: String) -> Self {
        self.redirect_uri = Some(redirect_uri);
        self
    }

    /// 设置自定义授权 URL
    pub fn with_authorization_url(mut self, url: String) -> Self {
        self.custom_authorization_url = Some(url);
        self
    }

    /// 设置自定义 Token URL
    pub fn with_token_url(mut self, url: String) -> Self {
        self.custom_token_url = Some(url);
        self
    }

    /// 设置自定义 scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.custom_scopes = Some(scopes);
        self
    }

    /// 获取 OAuth 授权 URL
    pub fn get_authorization_url(&self, state: &str) -> Result<String> {
        if let Some(ref url) = self.custom_authorization_url {
            let scope = self
                .custom_scopes
                .as_ref()
                .map(|s| s.join(" "))
                .unwrap_or_else(|| "api:full".to_string());

            let redirect_uri = self
                .redirect_uri
                .as_deref()
                .unwrap_or("http://localhost:18789/callback");

            let url_with_params = format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                url,
                urlencoding::encode(&self.client_id),
                urlencoding::encode(redirect_uri),
                urlencoding::encode(&scope),
                urlencoding::encode(state)
            );
            return Ok(url_with_params);
        }

        match self.provider {
            OAuthProvider::Anthropic => {
                let scope = "api:read api:write";
                Ok(format!(
                    "https://auth.anthropic.com/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::OpenAI => {
                let scope = "model.read organization.read";
                Ok(format!(
                    "https://openai.com/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::Google => {
                let scope = "https://www.googleapis.com/auth/generative-language.retriever";
                Ok(format!(
                    "https://accounts.google.com/o/oauth2/v2/auth?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}&\
                     access_type=offline&\
                     prompt=consent",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::Azure => Err(OAuthError::UnsupportedProvider(
                "Azure AD 使用不同的认证流程".into(),
            )),
            OAuthProvider::Qwen => {
                let scope = "api:full";
                Ok(format!(
                    "https://login.aliyun.com/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::Doubao => {
                let scope = "api:full";
                Ok(format!(
                    "https://login.volcengineapi.com/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::Minimax => {
                let scope = "api:full";
                Ok(format!(
                    "https://platform.minimax.io/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::Glm => {
                let scope = "api:full";
                Ok(format!(
                    "https://open.bigmodel.cn/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::Kimi => {
                let scope = "api:full";
                Ok(format!(
                    "https://platform.moonshot.cn/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
            OAuthProvider::DeepSeek => {
                let scope = "api:full";
                Ok(format!(
                    "https://platform.deepseek.com/oauth/authorize?\
                     client_id={}&\
                     redirect_uri={}&\
                     response_type=code&\
                     scope={}&\
                     state={}",
                    self.client_id,
                    self.redirect_uri
                        .as_deref()
                        .unwrap_or("http://localhost:18789/callback"),
                    urlencoding::encode(scope),
                    urlencoding::encode(state)
                ))
            }
        }
    }

    /// 使用授权码交换令牌
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthToken> {
        let token_url = self.get_token_url();
        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("code", code),
            (
                "redirect_uri",
                self.redirect_uri
                    .as_deref()
                    .unwrap_or("http://localhost:18789/callback"),
            ),
        ];

        let response = client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("OAuth token exchange failed: {}", error_text);
            return Err(OAuthError::AuthFailed(error_text));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let token = OAuthToken {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in),
            scope: token_response
                .scope
                .unwrap_or_default()
                .split_whitespace()
                .map(String::from)
                .collect(),
        };

        *self.current_token.write().await = Some(token.clone());

        Ok(token)
    }

    /// 刷新令牌
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthToken> {
        let token_url = self.get_token_url();
        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("refresh_token", refresh_token),
        ];

        let response = client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("OAuth token refresh failed: {}", error_text);
            return Err(OAuthError::RefreshFailed(error_text));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let token = OAuthToken {
            access_token: token_response.access_token,
            refresh_token: token_response
                .refresh_token
                .or(Some(refresh_token.to_string())),
            expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in),
            scope: token_response
                .scope
                .unwrap_or_default()
                .split_whitespace()
                .map(String::from)
                .collect(),
        };

        *self.current_token.write().await = Some(token.clone());

        Ok(token)
    }

    /// 获取当前有效令牌
    pub async fn get_token(&self) -> Result<OAuthToken> {
        let token = self.current_token.read().await;

        if let Some(token) = token.as_ref() {
            if token.expires_at > Utc::now() + chrono::Duration::seconds(60) {
                return Ok(token.clone());
            }

            if let Some(refresh_token) = &token.refresh_token {
                return self.refresh_token(refresh_token).await;
            }
        }

        Err(OAuthError::TokenExpired)
    }

    fn get_token_url(&self) -> String {
        if let Some(ref url) = self.custom_token_url {
            return url.clone();
        }

        match self.provider {
            OAuthProvider::Anthropic => "https://auth.anthropic.com/oauth/token".to_string(),
            OAuthProvider::OpenAI => "https://openai.com/oauth/token".to_string(),
            OAuthProvider::Google => "https://oauth2.googleapis.com/token".to_string(),
            OAuthProvider::Azure => {
                "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string()
            }
            OAuthProvider::Qwen => "https://api.opencompass.cn/oauth/token".to_string(),
            OAuthProvider::Doubao => {
                "https://ark.cn-beijing.volces.com/api/oauth/token".to_string()
            }
            OAuthProvider::Minimax => "https://api.minimax.chat/oauth/token".to_string(),
            OAuthProvider::Glm => "https://open.bigmodel.cn/oauth/token".to_string(),
            OAuthProvider::Kimi => "https://api.moonshot.cn/oauth/token".to_string(),
            OAuthProvider::DeepSeek => "https://api.deepseek.com/oauth/token".to_string(),
        }
    }
}

/// Auth Profile 轮换管理器
pub struct AuthProfileManager {
    profiles: RwLock<Vec<AuthProfile>>,
    current_index: RwLock<usize>,
}

impl AuthProfileManager {
    pub fn new(profiles: Vec<AuthProfile>) -> Self {
        let mut profiles = profiles;
        profiles.sort_by_key(|p| p.priority);

        Self {
            profiles: RwLock::new(profiles),
            current_index: RwLock::new(0),
        }
    }

    /// 获取当前可用的认证配置
    pub async fn get_active_auth(&self) -> Option<AuthProfile> {
        let profiles = self.profiles.read().await;
        let index = *self.current_index.read().await;

        if profiles.is_empty() {
            return None;
        }

        for i in 0..profiles.len() {
            let idx = (index + i) % profiles.len();
            let profile = &profiles[idx];

            if profile.enabled && !profile.is_expired() {
                return Some(profile.clone());
            }
        }

        None
    }

    /// 标记当前认证失败，切换到下一个
    pub async fn mark_failed(&self) {
        let mut index = self.current_index.write().await;
        let profiles = self.profiles.read().await;

        if !profiles.is_empty() {
            *index = (*index + 1) % profiles.len();
            info!("Auth profile 切换到下一个，索引: {}", *index);
        }
    }

    /// 手动切换到指定 profile
    pub async fn switch_to(&self, profile_id: &str) -> Result<()> {
        let profiles = self.profiles.read().await;

        for (i, profile) in profiles.iter().enumerate() {
            if profile.id == profile_id {
                if !profile.enabled {
                    return Err(OAuthError::AuthFailed("Profile 已禁用".into()));
                }
                *self.current_index.write().await = i;
                return Ok(());
            }
        }

        Err(OAuthError::AuthFailed(format!(
            "未找到 profile: {}",
            profile_id
        )))
    }

    /// 添加新的 auth profile
    pub async fn add_profile(&self, profile: AuthProfile) {
        let mut profiles = self.profiles.write().await;
        profiles.push(profile);
        profiles.sort_by_key(|p| p.priority);
    }

    /// 移除 auth profile
    pub async fn remove_profile(&self, profile_id: &str) -> bool {
        let mut profiles = self.profiles.write().await;
        let len_before = profiles.len();
        profiles.retain(|p| p.id != profile_id);
        profiles.len() < len_before
    }

    /// 列出所有 profiles
    pub async fn list_profiles(&self) -> Vec<AuthProfile> {
        self.profiles.read().await.clone()
    }
}
