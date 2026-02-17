//! 页面操作模块

use crate::browser::BrowserError;
use crate::types::*;
use chromiumoxide::page::Page as ChromiumPage;
use chromiumoxide_cdp::cdp::browser_protocol::network::CookieParam;
use chromiumoxide_cdp::cdp::browser_protocol::page::CaptureScreenshotFormat;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// 页面实例
pub struct Page {
    pub id: PageId,
    inner: Arc<ChromiumPage>,
}

impl Page {
    pub fn new(id: PageId, page: ChromiumPage) -> Self {
        Self {
            id,
            inner: Arc::new(page),
        }
    }

    /// 导航到 URL
    pub async fn goto(
        &self,
        url: &str,
        options: Option<NavigationOptions>,
    ) -> Result<(), BrowserError> {
        debug!("页面 {} 导航到: {}", self.id, url);

        let timeout = options.and_then(|o| o.timeout_ms).unwrap_or(30000);

        self.inner
            .goto(url)
            .await
            .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;

        // 等待页面加载
        tokio::time::timeout(
            Duration::from_millis(timeout),
            self.inner.wait_for_navigation(),
        )
        .await
        .map_err(|_| BrowserError::Timeout)?
        .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;

        Ok(())
    }

    /// 获取当前 URL
    pub async fn url(&self) -> Result<String, BrowserError> {
        let url = self
            .inner
            .url()
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?
            .unwrap_or_default();
        Ok(url)
    }

    /// 获取页面标题
    pub async fn title(&self) -> Result<String, BrowserError> {
        let title = self
            .inner
            .get_title()
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?
            .unwrap_or_default();
        Ok(title)
    }

    /// 获取页面内容
    pub async fn content(&self) -> Result<String, BrowserError> {
        let content = self
            .inner
            .content()
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;
        Ok(content)
    }

    /// 点击元素
    pub async fn click(
        &self,
        selector: &Selector,
        options: Option<ClickOptions>,
    ) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        debug!("页面 {} 点击元素: {}", self.id, selector_str);

        let options = options.unwrap_or_default();

        let element = self
            .inner
            .find_element(&selector_str)
            .await
            .map_err(|_| BrowserError::ElementNotFound(selector_str.clone()))?;

        if options.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(options.delay_ms)).await;
        }

        for _ in 0..options.click_count {
            element
                .click()
                .await
                .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// 输入文本
    pub async fn type_text(
        &self,
        selector: &Selector,
        text: &str,
        options: Option<TypeOptions>,
    ) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        debug!("页面 {} 输入文本到: {}", self.id, selector_str);

        let element = self
            .inner
            .find_element(&selector_str)
            .await
            .map_err(|_| BrowserError::ElementNotFound(selector_str.clone()))?;

        let options = options.unwrap_or_default();

        // 如果有延迟，逐字符输入
        if options.delay_ms > 0 {
            for c in text.chars() {
                element
                    .type_str(&c.to_string())
                    .await
                    .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;
                tokio::time::sleep(Duration::from_millis(options.delay_ms)).await;
            }
        } else {
            element
                .type_str(text)
                .await
                .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// 清除输入框 (通过 JavaScript 实现)
    pub async fn clear(&self, selector: &Selector) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        // 使用 JavaScript 清除输入框
        let script = format!("document.querySelector('{}').value = ''", selector_str);
        self.evaluate(&script).await?;

        Ok(())
    }

    /// 等待元素出现 (轮询实现)
    pub async fn wait_for_selector(
        &self,
        selector: &Selector,
        timeout_ms: Option<u64>,
    ) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;
        let timeout = timeout_ms.unwrap_or(30000);

        debug!("页面 {} 等待元素: {}", self.id, selector_str);

        let start = std::time::Instant::now();
        let duration = Duration::from_millis(timeout);
        let poll_interval = Duration::from_millis(100);

        loop {
            if self.inner.find_element(&selector_str).await.is_ok() {
                return Ok(());
            }

            if start.elapsed() >= duration {
                return Err(BrowserError::ElementNotFound(selector_str.clone()));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// 等待元素消失 (简化实现)
    pub async fn wait_for_selector_hidden(
        &self,
        selector: &Selector,
        _timeout_ms: Option<u64>,
    ) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;
        debug!("页面 {} 等待元素消失: {}", self.id, selector_str);
        // chromiumoxide 不支持直接等待元素消失，这里简化实现
        Ok(())
    }

    /// 等待导航
    pub async fn wait_for_navigation(&self, timeout_ms: Option<u64>) -> Result<(), BrowserError> {
        let timeout = timeout_ms.unwrap_or(30000);

        tokio::time::timeout(
            Duration::from_millis(timeout),
            self.inner.wait_for_navigation(),
        )
        .await
        .map_err(|_| BrowserError::Timeout)?
        .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;

        Ok(())
    }

    /// 执行 JavaScript
    pub async fn evaluate(&self, script: &str) -> Result<serde_json::Value, BrowserError> {
        debug!("页面 {} 执行脚本", self.id);

        let result = self
            .inner
            .evaluate(script)
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;

        // 从 EvaluationResult 中提取值
        Ok(result.value().cloned().unwrap_or(serde_json::Value::Null))
    }

    /// 查找元素
    pub async fn query_selector(&self, selector: &Selector) -> Result<ElementInfo, BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        let element = self
            .inner
            .find_element(&selector_str)
            .await
            .map_err(|_| BrowserError::ElementNotFound(selector_str.clone()))?;

        self.get_element_info(&element).await
    }

    /// 查找所有元素
    pub async fn query_selector_all(
        &self,
        selector: &Selector,
    ) -> Result<Vec<ElementInfo>, BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        let elements = self
            .inner
            .find_elements(&selector_str)
            .await
            .map_err(|_| BrowserError::ElementNotFound(selector_str.clone()))?;

        let mut result = Vec::new();
        for element in elements {
            result.push(self.get_element_info(&element).await?);
        }

        Ok(result)
    }

    /// 获取元素信息
    async fn get_element_info(
        &self,
        element: &chromiumoxide::element::Element,
    ) -> Result<ElementInfo, BrowserError> {
        // 获取属性
        let attr_vec = element.attributes().await.unwrap_or_default();
        let mut attributes = std::collections::HashMap::new();
        for chunk in attr_vec.chunks(2) {
            if chunk.len() == 2 {
                attributes.insert(chunk[0].clone(), chunk[1].clone());
            }
        }

        // 获取文本
        let text_content = element.inner_text().await.ok().flatten();

        // 获取边界框
        let bounding_box = element.bounding_box().await.ok().map(|bb| BoundingBox {
            x: bb.x,
            y: bb.y,
            width: bb.width,
            height: bb.height,
        });

        Ok(ElementInfo {
            tag_name: attributes.get("tagName").cloned().unwrap_or_default(),
            id: attributes.get("id").cloned(),
            class_name: attributes.get("class").cloned(),
            text_content,
            attributes,
            visible: true,
            clickable: true,
            bounding_box,
        })
    }

    /// 滚动页面
    pub async fn scroll(&self, options: ScrollOptions) -> Result<(), BrowserError> {
        if let Some(distance) = options.distance {
            let script = format!("window.scrollBy({}, {})", distance.x, distance.y);
            self.evaluate(&script).await?;
        } else if let Some(selector) = options.selector {
            let script = format!("document.querySelector('{}').scrollIntoView()", selector);
            self.evaluate(&script).await?;
        }

        Ok(())
    }

    /// 设置视口大小 (通过 JavaScript 实现)
    pub async fn set_viewport(&self, width: u32, height: u32) -> Result<(), BrowserError> {
        let script = format!("window.resizeTo({}, {})", width, height);
        self.evaluate(&script).await?;
        Ok(())
    }

    /// 设置 Cookie
    pub async fn set_cookies(&self, cookies: Vec<Cookie>) -> Result<(), BrowserError> {
        let cookie_params: Vec<CookieParam> = cookies
            .into_iter()
            .filter_map(|c| {
                let mut param = CookieParam::new(c.name, c.value);
                if let Some(domain) = c.domain {
                    param.domain = Some(domain);
                }
                if let Some(path) = c.path {
                    param.path = Some(path);
                }
                Some(param)
            })
            .collect();

        self.inner
            .set_cookies(cookie_params)
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;

        Ok(())
    }

    /// 获取 Cookies
    pub async fn get_cookies(&self) -> Result<Vec<Cookie>, BrowserError> {
        let cookies = self
            .inner
            .get_cookies()
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;

        Ok(cookies
            .into_iter()
            .map(|c| Cookie {
                name: c.name,
                value: c.value,
                domain: Some(c.domain),
                path: Some(c.path),
                expires: Some(c.expires),
                http_only: c.http_only,
                secure: c.secure,
                same_site: c.same_site.map(|s| format!("{:?}", s)),
            })
            .collect())
    }

    /// 上传文件 (需要额外实现)
    pub async fn upload_file(
        &self,
        _selector: &Selector,
        files: Vec<String>,
    ) -> Result<(), BrowserError> {
        debug!("页面 {} 上传文件请求: {:?}", self.id, files);
        Err(BrowserError::ExecutionFailed(
            "文件上传功能需要额外的实现".to_string(),
        ))
    }

    /// 关闭页面
    pub async fn close(&self) -> Result<(), BrowserError> {
        debug!("页面 {} 标记关闭", self.id);
        Ok(())
    }

    /// 刷新页面
    pub async fn reload(&self) -> Result<(), BrowserError> {
        self.inner
            .reload()
            .await
            .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;

        Ok(())
    }

    /// 悬停
    pub async fn hover(&self, selector: &Selector) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        let element = self
            .inner
            .find_element(&selector_str)
            .await
            .map_err(|_| BrowserError::ElementNotFound(selector_str.clone()))?;

        element
            .hover()
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;

        Ok(())
    }

    /// 聚焦
    pub async fn focus(&self, selector: &Selector) -> Result<(), BrowserError> {
        let selector_str = self.selector_to_string(selector)?;

        let element = self
            .inner
            .find_element(&selector_str)
            .await
            .map_err(|_| BrowserError::ElementNotFound(selector_str.clone()))?;

        element
            .focus()
            .await
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;

        Ok(())
    }

    /// 选择器转字符串
    fn selector_to_string(&self, selector: &Selector) -> Result<String, BrowserError> {
        if let Some(ref css) = selector.css {
            Ok(css.clone())
        } else if let Some(ref xpath) = selector.xpath {
            Ok(format!("xpath//{}", xpath))
        } else if let Some(ref text) = selector.text {
            Ok(format!("text={}", text))
        } else {
            Err(BrowserError::ExecutionFailed("无效的选择器".to_string()))
        }
    }

    /// 截图
    pub async fn screenshot(
        &self,
        options: Option<ScreenshotOptions>,
    ) -> Result<Vec<u8>, BrowserError> {
        let options = options.unwrap_or_default();

        debug!("页面 {} 截图", self.id);

        let format = match options.format {
            ScreenshotFormat::Png => CaptureScreenshotFormat::Png,
            ScreenshotFormat::Jpeg => CaptureScreenshotFormat::Jpeg,
            ScreenshotFormat::Webp => CaptureScreenshotFormat::Webp,
        };

        let params = chromiumoxide::page::ScreenshotParams::builder()
            .format(format)
            .full_page(options.full_page)
            .omit_background(options.omit_background)
            .build();

        let data = self
            .inner
            .screenshot(params)
            .await
            .map_err(|e| BrowserError::ExecutionFailed(format!("截图失败: {}", e)))?;

        Ok(data)
    }

    /// 截图并转为 Base64
    pub async fn screenshot_base64(
        &self,
        options: Option<ScreenshotOptions>,
    ) -> Result<String, BrowserError> {
        let data = self.screenshot(options).await?;
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &data,
        ))
    }

    /// 生成 PDF
    pub async fn pdf(&self, options: Option<PdfOptions>) -> Result<Vec<u8>, BrowserError> {
        let options = options.unwrap_or_default();

        // 使用 chromiumoxide 的 pdf 方法
        let data = self
            .inner
            .pdf(
                chromiumoxide_cdp::cdp::browser_protocol::page::PrintToPdfParams {
                    landscape: Some(options.landscape),
                    print_background: Some(options.print_background),
                    scale: Some(options.scale),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| BrowserError::ExecutionFailed(format!("PDF 生成失败: {}", e)))?;

        Ok(data)
    }

    /// 生成 PDF 并转为 Base64
    pub async fn pdf_base64(&self, options: Option<PdfOptions>) -> Result<String, BrowserError> {
        let data = self.pdf(options).await?;
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &data,
        ))
    }
}
