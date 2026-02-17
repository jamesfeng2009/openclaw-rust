//! 浏览器控制 API 路由

use axum::{
    Json, Router,
    extract::State,
    routing::{delete, get, post},
};
use openclaw_browser::{
    BrowserConfig, BrowserId, BrowserInfo, BrowserPool, ClickOptions, Cookie,
    NavigationOptions, PageId, PdfOptions, ScreenshotOptions, Selector,
    TypeOptions,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// 浏览器 API 状态
#[derive(Clone)]
pub struct BrowserApiState {
    pub pool: Arc<BrowserPool>,
}

impl BrowserApiState {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(BrowserPool::new(None)),
        }
    }
}

impl Default for BrowserApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建浏览器 API 路由
pub fn create_browser_router(state: BrowserApiState) -> Router {
    Router::new()
        // 浏览器实例管理
        .route("/browser", post(create_browser))
        .route("/browser", get(list_browsers))
        .route("/browser/{id}", delete(delete_browser))
        .route("/browser/{id}/version", get(get_browser_version))
        // 页面管理
        .route("/browser/{id}/page", post(create_page))
        .route("/browser/{id}/pages", get(list_pages))
        .route("/page/{browser_id}/{page_id}", delete(close_page))
        // 页面操作
        .route("/page/{browser_id}/{page_id}/goto", post(page_goto))
        .route("/page/{browser_id}/{page_id}/url", get(get_page_url))
        .route("/page/{browser_id}/{page_id}/title", get(get_page_title))
        .route(
            "/page/{browser_id}/{page_id}/content",
            get(get_page_content),
        )
        .route("/page/{browser_id}/{page_id}/click", post(page_click))
        .route("/page/{browser_id}/{page_id}/type", post(page_type))
        .route("/page/{browser_id}/{page_id}/clear", post(page_clear))
        .route("/page/{browser_id}/{page_id}/wait", post(page_wait))
        .route("/page/{browser_id}/{page_id}/evaluate", post(page_evaluate))
        .route("/page/{browser_id}/{page_id}/query", post(page_query))
        .route("/page/{browser_id}/{page_id}/scroll", post(page_scroll))
        .route("/page/{browser_id}/{page_id}/cookies", get(get_cookies))
        .route("/page/{browser_id}/{page_id}/cookies", post(set_cookies))
        .route("/page/{browser_id}/{page_id}/upload", post(page_upload))
        .route("/page/{browser_id}/{page_id}/reload", post(page_reload))
        .route("/page/{browser_id}/{page_id}/back", post(page_back))
        .route("/page/{browser_id}/{page_id}/forward", post(page_forward))
        // 截图和 PDF
        .route(
            "/page/{browser_id}/{page_id}/screenshot",
            post(page_screenshot),
        )
        .route("/page/{browser_id}/{page_id}/pdf", post(page_pdf))
        .with_state(state)
}

// ==================== 浏览器实例管理 ====================

/// 创建浏览器请求
#[derive(Debug, Deserialize)]
pub struct CreateBrowserRequest {
    pub headless: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub user_agent: Option<String>,
}

/// 创建浏览器响应
#[derive(Debug, Serialize)]
pub struct CreateBrowserResponse {
    pub id: BrowserId,
}

/// 创建浏览器实例
async fn create_browser(
    State(state): State<BrowserApiState>,
    Json(req): Json<CreateBrowserRequest>,
) -> Result<Json<CreateBrowserResponse>, String> {
    let config = BrowserConfig {
        headless: req.headless.unwrap_or(true),
        width: req.width.unwrap_or(1920),
        height: req.height.unwrap_or(1080),
        user_agent: req.user_agent,
        ..Default::default()
    };

    let id = state
        .pool
        .create_browser(Some(config))
        .await
        .map_err(|e| e.to_string())?;
    info!("创建浏览器实例: {}", id);

    Ok(Json(CreateBrowserResponse { id }))
}

/// 列出浏览器
async fn list_browsers(State(state): State<BrowserApiState>) -> Json<Vec<BrowserInfo>> {
    let browsers = state.pool.list_browsers().await;
    Json(browsers)
}

/// 删除浏览器
async fn delete_browser(
    State(state): State<BrowserApiState>,
    axum::extract::Path(id): axum::extract::Path<BrowserId>,
) -> Result<Json<serde_json::Value>, String> {
    state
        .pool
        .destroy_browser(&id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// 获取浏览器版本
async fn get_browser_version(
    State(state): State<BrowserApiState>,
    axum::extract::Path(id): axum::extract::Path<BrowserId>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state.pool.get_browser(&id).await.ok_or("浏览器不存在")?;
    let version = browser.version().await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "version": version })))
}

// ==================== 页面管理 ====================

/// 创建页面响应
#[derive(Debug, Serialize)]
pub struct CreatePageResponse {
    pub page_id: PageId,
}

/// 创建新页面
async fn create_page(
    State(state): State<BrowserApiState>,
    axum::extract::Path(browser_id): axum::extract::Path<BrowserId>,
) -> Result<Json<CreatePageResponse>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.new_page().await.map_err(|e| e.to_string())?;

    Ok(Json(CreatePageResponse {
        page_id: page.id.clone(),
    }))
}

/// 列出页面
async fn list_pages(
    State(state): State<BrowserApiState>,
    axum::extract::Path(browser_id): axum::extract::Path<BrowserId>,
) -> Result<Json<Vec<PageId>>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let pages = browser.get_pages().await;
    Ok(Json(pages))
}

/// 关闭页面
async fn close_page(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    browser
        .close_page(&page_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

// ==================== 页面操作 ====================

/// 导航请求
#[derive(Debug, Deserialize)]
pub struct GotoRequest {
    pub url: String,
    pub timeout_ms: Option<u64>,
}

/// 页面导航
async fn page_goto(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<GotoRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let options = NavigationOptions {
        timeout_ms: req.timeout_ms,
        ..Default::default()
    };

    page.goto(&req.url, Some(options))
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// 获取页面 URL
async fn get_page_url(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let url = page.url().await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "url": url })))
}

/// 获取页面标题
async fn get_page_title(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let title = page.title().await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "title": title })))
}

/// 获取页面内容
async fn get_page_content(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let content = page.content().await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "content": content })))
}

/// 点击请求
#[derive(Debug, Deserialize)]
pub struct ClickRequest {
    pub selector: String,
    pub click_count: Option<usize>,
    pub delay_ms: Option<u64>,
}

/// 页面点击
async fn page_click(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<ClickRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let options = ClickOptions {
        click_count: req.click_count.unwrap_or(1),
        delay_ms: req.delay_ms.unwrap_or(0),
        ..Default::default()
    };

    page.click(&Selector::css(&req.selector), Some(options))
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// 输入请求
#[derive(Debug, Deserialize)]
pub struct TypeRequest {
    pub selector: String,
    pub text: String,
    pub delay_ms: Option<u64>,
}

/// 页面输入
async fn page_type(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<TypeRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let options = TypeOptions {
        delay_ms: req.delay_ms.unwrap_or(0),
    };

    page.type_text(&Selector::css(&req.selector), &req.text, Some(options))
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// 清除请求
#[derive(Debug, Deserialize)]
pub struct ClearRequest {
    pub selector: String,
}

/// 清除输入框
async fn page_clear(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<ClearRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    page.clear(&Selector::css(&req.selector))
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// 等待请求
#[derive(Debug, Deserialize)]
pub struct WaitRequest {
    pub selector: Option<String>,
    pub timeout_ms: Option<u64>,
    pub hidden: Option<bool>,
}

/// 等待元素
async fn page_wait(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<WaitRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    if let Some(selector) = req.selector {
        if req.hidden.unwrap_or(false) {
            page.wait_for_selector_hidden(&Selector::css(&selector), req.timeout_ms)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            page.wait_for_selector(&Selector::css(&selector), req.timeout_ms)
                .await
                .map_err(|e| e.to_string())?;
        }
    } else {
        page.wait_for_navigation(req.timeout_ms)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// 执行 JS 请求
#[derive(Debug, Deserialize)]
pub struct EvaluateRequest {
    pub script: String,
}

/// 执行 JavaScript
async fn page_evaluate(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<EvaluateRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let result = page
        .evaluate(&req.script)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "result": result })))
}

/// 查询请求
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub selector: String,
    pub all: Option<bool>,
}

/// 查询元素
async fn page_query(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    if req.all.unwrap_or(false) {
        let elements = page
            .query_selector_all(&Selector::css(&req.selector))
            .await
            .map_err(|e| e.to_string())?;
        Ok(Json(serde_json::json!({ "elements": elements })))
    } else {
        let element = page
            .query_selector(&Selector::css(&req.selector))
            .await
            .map_err(|e| e.to_string())?;
        Ok(Json(serde_json::json!({ "element": element })))
    }
}

/// 滚动请求
#[derive(Debug, Deserialize)]
pub struct ScrollRequest {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub selector: Option<String>,
}

/// 滚动页面
async fn page_scroll(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<ScrollRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let options = openclaw_browser::ScrollOptions {
        distance: req
            .x
            .zip(req.y)
            .map(|(x, y)| openclaw_browser::ScrollDistance { x, y }),
        selector: req.selector,
    };

    page.scroll(options).await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// 获取 Cookies
async fn get_cookies(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let cookies = page.get_cookies().await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "cookies": cookies })))
}

/// 设置 Cookies 请求
#[derive(Debug, Deserialize)]
pub struct SetCookiesRequest {
    pub cookies: Vec<Cookie>,
}

/// 设置 Cookies
async fn set_cookies(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<SetCookiesRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    page.set_cookies(req.cookies)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// 上传文件请求
#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub selector: String,
    pub files: Vec<String>,
}

/// 上传文件
async fn page_upload(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<UploadRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    page.upload_file(&Selector::css(&req.selector), req.files)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// 刷新页面
async fn page_reload(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    page.reload().await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// 后退 (通过 JavaScript 实现)
async fn page_back(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    page.evaluate("window.history.back()")
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// 前进 (通过 JavaScript 实现)
async fn page_forward(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
) -> Result<Json<serde_json::Value>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    page.evaluate("window.history.forward()")
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"success": true})))
}

// ==================== 截图和 PDF ====================

/// 截图请求
#[derive(Debug, Deserialize)]
pub struct ScreenshotRequest {
    pub full_page: Option<bool>,
    pub format: Option<String>,
    pub quality: Option<u8>,
}

/// 截图响应
#[derive(Debug, Serialize)]
pub struct ScreenshotResponse {
    pub data: String, // base64
    pub mime_type: String,
}

/// 页面截图
async fn page_screenshot(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<ScreenshotRequest>,
) -> Result<Json<ScreenshotResponse>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let format = match req.format.as_deref() {
        Some("jpeg") | Some("jpg") => openclaw_browser::ScreenshotFormat::Jpeg,
        Some("webp") => openclaw_browser::ScreenshotFormat::Webp,
        _ => openclaw_browser::ScreenshotFormat::Png,
    };

    let options = ScreenshotOptions {
        format,
        quality: req.quality,
        full_page: req.full_page.unwrap_or(false),
        ..Default::default()
    };

    let base64 = page
        .screenshot_base64(Some(options))
        .await
        .map_err(|e| e.to_string())?;

    let mime_type = match format {
        openclaw_browser::ScreenshotFormat::Png => "image/png",
        openclaw_browser::ScreenshotFormat::Jpeg => "image/jpeg",
        openclaw_browser::ScreenshotFormat::Webp => "image/webp",
    };

    Ok(Json(ScreenshotResponse {
        data: base64,
        mime_type: mime_type.to_string(),
    }))
}

/// PDF 请求
#[derive(Debug, Deserialize)]
pub struct PdfRequest {
    pub format: Option<String>,
    pub landscape: Option<bool>,
    pub print_background: Option<bool>,
}

/// PDF 响应
#[derive(Debug, Serialize)]
pub struct PdfResponse {
    pub data: String, // base64
}

/// 生成 PDF
async fn page_pdf(
    State(state): State<BrowserApiState>,
    axum::extract::Path((browser_id, page_id)): axum::extract::Path<(BrowserId, PageId)>,
    Json(req): Json<PdfRequest>,
) -> Result<Json<PdfResponse>, String> {
    let browser = state
        .pool
        .get_browser(&browser_id)
        .await
        .ok_or("浏览器不存在")?;
    let page = browser.get_page(&page_id).await.ok_or("页面不存在")?;

    let format = match req.format.as_deref() {
        Some("a3") => openclaw_browser::PaperFormat::A3,
        Some("a5") => openclaw_browser::PaperFormat::A5,
        _ => openclaw_browser::PaperFormat::A4,
    };

    let options = PdfOptions {
        format: Some(format),
        landscape: req.landscape.unwrap_or(false),
        print_background: req.print_background.unwrap_or(false),
        ..Default::default()
    };

    let base64 = page
        .pdf_base64(Some(options))
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(PdfResponse { data: base64 }))
}
