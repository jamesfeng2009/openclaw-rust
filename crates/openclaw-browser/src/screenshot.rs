//! 截图和 PDF 工具函数

use crate::types::*;

/// 截图工具函数
pub struct ScreenshotUtils;

impl ScreenshotUtils {
    /// 截取整个页面
    pub fn full_page_png() -> ScreenshotOptions {
        ScreenshotOptions {
            format: ScreenshotFormat::Png,
            full_page: true,
            ..Default::default()
        }
    }

    /// 截取可视区域
    pub fn viewport_png() -> ScreenshotOptions {
        ScreenshotOptions {
            format: ScreenshotFormat::Png,
            full_page: false,
            ..Default::default()
        }
    }

    /// 截取指定区域
    pub fn clip_png(x: f64, y: f64, width: f64, height: f64) -> ScreenshotOptions {
        ScreenshotOptions {
            format: ScreenshotFormat::Png,
            full_page: false,
            clip: Some(ClipRect {
                x,
                y,
                width,
                height,
            }),
            ..Default::default()
        }
    }

    /// 高质量 JPEG 截图
    pub fn high_quality_jpeg(quality: u8) -> ScreenshotOptions {
        ScreenshotOptions {
            format: ScreenshotFormat::Jpeg,
            quality: Some(quality),
            full_page: true,
            ..Default::default()
        }
    }
}

/// PDF 工具函数
pub struct PdfUtils;

impl PdfUtils {
    /// A4 纵向 PDF
    pub fn a4_portrait() -> PdfOptions {
        PdfOptions {
            format: Some(PaperFormat::A4),
            landscape: false,
            ..Default::default()
        }
    }

    /// A4 横向 PDF
    pub fn a4_landscape() -> PdfOptions {
        PdfOptions {
            format: Some(PaperFormat::A4),
            landscape: true,
            ..Default::default()
        }
    }

    /// 自定义尺寸 PDF
    pub fn custom_size(width: &str, height: &str) -> PdfOptions {
        PdfOptions {
            format: None,
            width: Some(width.to_string()),
            height: Some(height.to_string()),
            ..Default::default()
        }
    }

    /// 带边距的 PDF
    pub fn with_margins(
        top: &str,
        bottom: &str,
        left: &str,
        right: &str,
    ) -> PdfOptions {
        PdfOptions {
            margin: Some(PdfMargins {
                top: Some(top.to_string()),
                bottom: Some(bottom.to_string()),
                left: Some(left.to_string()),
                right: Some(right.to_string()),
            }),
            ..Default::default()
        }
    }
}
