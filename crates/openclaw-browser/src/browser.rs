//! 浏览器实例管理

use crate::page::Page;
use crate::types::*;
use chromiumoxide::browser::{Browser as ChromiumBrowser, BrowserConfig as ChromiumConfig};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 浏览器错误
#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("浏览器启动失败: {0}")]
    LaunchFailed(String),

    #[error("页面不存在: {0}")]
    PageNotFound(PageId),

    #[error("导航失败: {0}")]
    NavigationFailed(String),

    #[error("操作超时")]
    Timeout,

    #[error("元素未找到: {0}")]
    ElementNotFound(String),

    #[error("执行失败: {0}")]
    ExecutionFailed(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// 浏览器实例
pub struct Browser {
    pub id: BrowserId,
    inner: ChromiumBrowser,
    config: BrowserConfig,
    pages: Arc<RwLock<HashMap<PageId, Arc<Page>>>>,
}

impl Browser {
    /// 启动浏览器
    pub async fn launch(config: BrowserConfig) -> Result<Self, BrowserError> {
        info!("启动浏览器实例...");

        let mut chrome_config = ChromiumConfig::builder().window_size(config.width, config.height);

        if config.headless {
            chrome_config = chrome_config.no_sandbox();
        }

        if let Some(ref path) = config.executable_path {
            chrome_config = chrome_config.chrome_executable(path);
        }

        for arg in &config.args {
            chrome_config = chrome_config.arg(arg);
        }

        let chrome_config = chrome_config
            .build()
            .map_err(|e| BrowserError::LaunchFailed(format!("配置错误: {}", e)))?;

        let (browser, mut handler) = ChromiumBrowser::launch(chrome_config)
            .await
            .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;

        // 在后台运行 handler
        tokio::spawn(async move {
            use futures::StreamExt;
            while let Some(_) = handler.next().await {}
        });

        let id = Uuid::new_v4().to_string();
        info!("浏览器实例已启动: {}", id);

        Ok(Self {
            id,
            inner: browser,
            config,
            pages: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 创建新页面
    pub async fn new_page(&self) -> Result<Arc<Page>, BrowserError> {
        let page = self
            .inner
            .new_page("about:blank")
            .await
            .map_err(|e| BrowserError::ExecutionFailed(format!("创建页面失败: {}", e)))?;

        let page_id = Uuid::new_v4().to_string();
        let page_wrapper = Arc::new(Page::new(page_id.clone(), page));

        {
            let mut pages = self.pages.write().await;
            pages.insert(page_id.clone(), page_wrapper.clone());
        }

        debug!("创建新页面: {}", page_id);
        Ok(page_wrapper)
    }

    /// 获取页面
    pub async fn get_page(&self, page_id: &PageId) -> Option<Arc<Page>> {
        let pages = self.pages.read().await;
        pages.get(page_id).cloned()
    }

    /// 关闭页面
    pub async fn close_page(&self, page_id: &PageId) -> Result<(), BrowserError> {
        let page = self
            .get_page(page_id)
            .await
            .ok_or_else(|| BrowserError::PageNotFound(page_id.clone()))?;

        page.close().await?;

        {
            let mut pages = self.pages.write().await;
            pages.remove(page_id);
        }

        debug!("关闭页面: {}", page_id);
        Ok(())
    }

    /// 获取所有页面
    pub async fn get_pages(&self) -> Vec<PageId> {
        let pages = self.pages.read().await;
        pages.keys().cloned().collect()
    }

    /// 获取页面数量
    pub async fn page_count(&self) -> usize {
        let pages = self.pages.read().await;
        pages.len()
    }

    /// 关闭浏览器
    pub async fn close(&self) -> Result<(), BrowserError> {
        info!("关闭浏览器实例: {}", self.id);

        // 关闭所有页面
        {
            let mut pages = self.pages.write().await;
            for (_, page) in pages.drain() {
                let _ = page.close().await;
            }
        }

        Ok(())
    }

    /// 获取浏览器版本
    pub async fn version(&self) -> Result<String, BrowserError> {
        // 简化版本获取
        Ok("Chromium".to_string())
    }

    /// 获取浏览器指标
    pub async fn metrics(&self) -> Result<BrowserMetrics, BrowserError> {
        let page_count = self.page_count().await;

        Ok(BrowserMetrics {
            page_count,
            memory_used: 0,
            cpu_usage: 0.0,
        })
    }
}

/// 浏览器池管理器
pub struct BrowserPool {
    browsers: Arc<RwLock<HashMap<BrowserId, Arc<Browser>>>>,
    default_config: BrowserConfig,
}

impl BrowserPool {
    /// 创建新的浏览器池
    pub fn new(default_config: Option<BrowserConfig>) -> Self {
        Self {
            browsers: Arc::new(RwLock::new(HashMap::new())),
            default_config: default_config.unwrap_or_default(),
        }
    }

    /// 创建浏览器实例
    pub async fn create_browser(
        &self,
        config: Option<BrowserConfig>,
    ) -> Result<BrowserId, BrowserError> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let browser = Browser::launch(config).await?;
        let id = browser.id.clone();

        {
            let mut browsers = self.browsers.write().await;
            browsers.insert(id.clone(), Arc::new(browser));
        }

        Ok(id)
    }

    /// 获取浏览器
    pub async fn get_browser(&self, id: &BrowserId) -> Option<Arc<Browser>> {
        let browsers = self.browsers.read().await;
        browsers.get(id).cloned()
    }

    /// 销毁浏览器
    pub async fn destroy_browser(&self, id: &BrowserId) -> Result<(), BrowserError> {
        let browser = {
            let mut browsers = self.browsers.write().await;
            browsers.remove(id)
        };

        if let Some(browser) = browser {
            browser.close().await?;
            info!("浏览器实例已销毁: {}", id);
        }

        Ok(())
    }

    /// 列出所有浏览器
    pub async fn list_browsers(&self) -> Vec<BrowserInfo> {
        let browsers = self.browsers.read().await;
        let mut result = Vec::new();

        for (id, browser) in browsers.iter() {
            result.push(BrowserInfo {
                id: id.clone(),
                page_count: browser.page_count().await,
            });
        }

        result
    }

    /// 清理所有浏览器
    pub async fn cleanup(&self) {
        let ids: Vec<BrowserId> = {
            let browsers = self.browsers.read().await;
            browsers.keys().cloned().collect()
        };

        for id in ids {
            if let Err(e) = self.destroy_browser(&id).await {
                warn!("清理浏览器失败 {}: {}", id, e);
            }
        }

        info!("所有浏览器实例已清理");
    }
}

impl Default for BrowserPool {
    fn default() -> Self {
        Self::new(None)
    }
}

/// 浏览器信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowserInfo {
    pub id: BrowserId,
    pub page_count: usize,
}
