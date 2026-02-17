//! 浏览器控制类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 浏览器实例 ID
pub type BrowserId = String;

/// 页面 ID
pub type PageId = String;

/// 浏览器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// 是否无头模式
    pub headless: bool,
    /// 浏览器可执行文件路径
    pub executable_path: Option<String>,
    /// 窗口宽度
    pub width: u32,
    /// 窗口高度
    pub height: u32,
    /// 设备像素比
    pub device_scale_factor: f64,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 启动参数
    pub args: Vec<String>,
    /// 是否忽略 HTTPS 错误
    pub ignore_https_errors: bool,
    /// 是否禁用 GPU
    pub disable_gpu: bool,
    /// 代理服务器
    pub proxy: Option<ProxyConfig>,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            executable_path: None,
            width: 1920,
            height: 1080,
            device_scale_factor: 1.0,
            user_agent: None,
            args: vec![],
            ignore_https_errors: false,
            disable_gpu: true,
            proxy: None,
            timeout_ms: 30000,
        }
    }
}

impl BrowserConfig {
    /// 创建无头浏览器配置
    pub fn headless() -> Self {
        Self::default()
    }

    /// 创建有头浏览器配置
    pub fn headed() -> Self {
        Self {
            headless: false,
            ..Default::default()
        }
    }

    /// 设置窗口大小
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// 设置用户代理
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// 添加启动参数
    pub fn with_arg(mut self, arg: String) -> Self {
        self.args.push(arg);
        self
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// 页面选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageOptions {
    /// 是否在新标签页打开
    pub new_tab: bool,
    /// 是否在后台打开
    pub background: bool,
}

/// 导航选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NavigationOptions {
    /// 超时时间（毫秒）
    pub timeout_ms: Option<u64>,
    /// 等待条件
    pub wait_until: Option<WaitUntil>,
}

/// 等待条件
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WaitUntil {
    Load,
    DomContentLoaded,
    NetworkIdle,
}

/// 截图选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotOptions {
    /// 截图类型
    pub format: ScreenshotFormat,
    /// 图片质量（仅 JPEG，0-100）
    pub quality: Option<u8>,
    /// 是否全页面
    pub full_page: bool,
    /// 裁剪区域
    pub clip: Option<ClipRect>,
    /// 是否省略背景
    pub omit_background: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            format: ScreenshotFormat::Png,
            quality: None,
            full_page: false,
            clip: None,
            omit_background: false,
        }
    }
}

/// 截图格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Webp,
}

/// 裁剪区域
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClipRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 点击选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClickOptions {
    /// 点击次数
    pub click_count: usize,
    /// 按钮类型
    pub button: MouseButton,
    /// 延迟时间（毫秒）
    pub delay_ms: u64,
    /// 偏移位置
    pub offset: Option<Point>,
}

/// 鼠标按钮
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

/// 输入选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypeOptions {
    /// 输入延迟（毫秒）
    pub delay_ms: u64,
}

/// 滚动选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrollOptions {
    /// 滚动距离
    pub distance: Option<ScrollDistance>,
    /// 滚动到元素
    pub selector: Option<String>,
}

/// 滚动距离
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScrollDistance {
    pub x: f64,
    pub y: f64,
}

/// 点坐标
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// 选择器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selector {
    /// CSS 选择器
    pub css: Option<String>,
    /// XPath
    pub xpath: Option<String>,
    /// 文本内容
    pub text: Option<String>,
}

impl Selector {
    pub fn css(css: &str) -> Self {
        Self {
            css: Some(css.to_string()),
            xpath: None,
            text: None,
        }
    }

    pub fn xpath(xpath: &str) -> Self {
        Self {
            css: None,
            xpath: Some(xpath.to_string()),
            text: None,
        }
    }

    pub fn text(text: &str) -> Self {
        Self {
            css: None,
            xpath: None,
            text: Some(text.to_string()),
        }
    }
}

/// 元素信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    /// 标签名
    pub tag_name: String,
    /// ID
    pub id: Option<String>,
    /// 类名
    pub class_name: Option<String>,
    /// 文本内容
    pub text_content: Option<String>,
    /// 属性
    pub attributes: HashMap<String, String>,
    /// 是否可见
    pub visible: bool,
    /// 是否可点击
    pub clickable: bool,
    /// 边界框
    pub bounding_box: Option<BoundingBox>,
}

/// 边界框
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 文件上传选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadOptions {
    /// 文件路径列表
    pub files: Vec<String>,
    /// Base64 编码的文件数据
    pub base64_data: Option<Vec<FileData>>,
}

/// 文件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    pub name: String,
    pub mime_type: String,
    pub data: String, // base64
}

/// 页面状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    pub id: PageId,
    pub url: String,
    pub title: Option<String>,
    pub is_loading: bool,
    pub cookies: Vec<Cookie>,
}

/// Cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<f64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

/// 浏览器指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMetrics {
    /// 页面数量
    pub page_count: usize,
    /// 内存使用（字节）
    pub memory_used: u64,
    /// CPU 使用率
    pub cpu_usage: f64,
}

/// PDF 选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfOptions {
    /// 纸张尺寸
    pub format: Option<PaperFormat>,
    /// 宽度
    pub width: Option<String>,
    /// 高度
    pub height: Option<String>,
    /// 页边距
    pub margin: Option<PdfMargins>,
    /// 横向打印
    pub landscape: bool,
    /// 打印背景
    pub print_background: bool,
    /// 缩放比例
    pub scale: f64,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            format: Some(PaperFormat::A4),
            width: None,
            height: None,
            margin: None,
            landscape: false,
            print_background: false,
            scale: 1.0,
        }
    }
}

/// 纸张格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PaperFormat {
    Letter,
    Legal,
    Tabloid,
    Ledger,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
}

/// PDF 页边距
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMargins {
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

/// 浏览器事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BrowserEvent {
    PageCreated {
        page_id: PageId,
    },
    PageClosed {
        page_id: PageId,
    },
    PageNavigated {
        page_id: PageId,
        url: String,
    },
    PageCrashed {
        page_id: PageId,
    },
    ConsoleMessage {
        page_id: PageId,
        message: String,
        level: String,
    },
    DialogOpened {
        page_id: PageId,
        message: String,
        dialog_type: String,
    },
}

/// JS 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsResult {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsResult>),
    Object(HashMap<String, JsResult>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert!(config.headless);
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_browser_config_headless() {
        let config = BrowserConfig::headless();
        assert!(config.headless);
    }

    #[test]
    fn test_browser_config_headed() {
        let config = BrowserConfig::headed();
        assert!(!config.headless);
    }

    #[test]
    fn test_browser_config_with_size() {
        let config = BrowserConfig::default().with_size(1280, 720);
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
    }

    #[test]
    fn test_browser_config_with_user_agent() {
        let config = BrowserConfig::default().with_user_agent("TestAgent/1.0".to_string());
        assert_eq!(config.user_agent, Some("TestAgent/1.0".to_string()));
    }

    #[test]
    fn test_browser_config_with_args() {
        let config = BrowserConfig::default()
            .with_arg("--disable-extensions".to_string())
            .with_arg("--disable-popup-blocking".to_string());
        assert_eq!(config.args.len(), 2);
    }

    #[test]
    fn test_browser_config_with_timeout() {
        let config = BrowserConfig::default().with_timeout(60000);
        assert_eq!(config.timeout_ms, 60000);
    }

    #[test]
    fn test_selector_css() {
        let selector = Selector::css(".class#id");
        assert_eq!(selector.css, Some(".class#id".to_string()));
        assert!(selector.xpath.is_none());
    }

    #[test]
    fn test_selector_xpath() {
        let selector = Selector::xpath("//div[@class='test']");
        assert_eq!(selector.xpath, Some("//div[@class='test']".to_string()));
        assert!(selector.css.is_none());
    }

    #[test]
    fn test_selector_text() {
        let selector = Selector::text("Click here");
        assert_eq!(selector.text, Some("Click here".to_string()));
    }

    #[test]
    fn test_screenshot_options_default() {
        let options = ScreenshotOptions::default();
        assert!(!options.full_page);
        assert_eq!(options.format, ScreenshotFormat::Png);
    }

    #[test]
    fn test_pdf_options_default() {
        let options = PdfOptions::default();
        assert_eq!(options.format, Some(PaperFormat::A4));
        assert!(!options.print_background);
    }

    #[test]
    fn test_wait_until_values() {
        assert_eq!(WaitUntil::Load, WaitUntil::Load);
        assert_eq!(WaitUntil::DomContentLoaded, WaitUntil::DomContentLoaded);
        assert_eq!(WaitUntil::NetworkIdle, WaitUntil::NetworkIdle);
    }

    #[test]
    fn test_mouse_button_values() {
        assert_eq!(MouseButton::Left, MouseButton::Left);
        assert_eq!(MouseButton::Right, MouseButton::Right);
        assert_eq!(MouseButton::Middle, MouseButton::Middle);
    }
}
