//! 画布类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 画布 ID
pub type CanvasId = String;

/// 用户 ID
pub type UserId = String;

/// 画布颜色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self::new(r, g, b, 255))
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn red() -> Self {
        Self::new(255, 0, 0, 255)
    }
    pub fn green() -> Self {
        Self::new(0, 255, 0, 255)
    }
    pub fn blue() -> Self {
        Self::new(0, 0, 255, 255)
    }
    pub fn black() -> Self {
        Self::new(0, 0, 0, 255)
    }
    pub fn white() -> Self {
        Self::new(255, 255, 255, 255)
    }
    pub fn transparent() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

/// 点坐标
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// 矩形区域
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// 线条样式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrokeStyle {
    pub color: Color,
    pub width: f64,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Color::black(),
            width: 1.0,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
        }
    }
}

/// 线端样式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// 线连接样式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// 填充样式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FillStyle {
    pub color: Color,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: Color::transparent(),
        }
    }
}

/// 图形元素类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Shape {
    /// 自由绘制路径
    Path {
        points: Vec<Point>,
        stroke: StrokeStyle,
    },
    /// 直线
    Line {
        start: Point,
        end: Point,
        stroke: StrokeStyle,
    },
    /// 矩形
    Rectangle {
        rect: Rect,
        stroke: Option<StrokeStyle>,
        fill: Option<FillStyle>,
    },
    /// 椭圆/圆
    Ellipse {
        center: Point,
        radius_x: f64,
        radius_y: f64,
        stroke: Option<StrokeStyle>,
        fill: Option<FillStyle>,
    },
    /// 文本
    Text {
        position: Point,
        content: String,
        font_size: f64,
        font_family: String,
        color: Color,
    },
    /// 图片
    Image {
        rect: Rect,
        data: String, // base64 encoded
        mime_type: String,
    },
    /// SVG 形状
    Svg {
        position: Point,
        svg_content: String,
        scale: f64,
    },
}

/// 图形元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    pub shape: Shape,
    pub layer: usize,
    pub locked: bool,
    pub visible: bool,
    pub opacity: f64,
    pub transform: Transform,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<UserId>,
}

impl Element {
    pub fn new(shape: Shape, created_by: Option<UserId>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            shape,
            layer: 0,
            locked: false,
            visible: true,
            opacity: 1.0,
            transform: Transform::default(),
            created_at: now,
            updated_at: now,
            created_by,
        }
    }

    pub fn with_layer(mut self, layer: usize) -> Self {
        self.layer = layer;
        self
    }
}

/// 变换
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    pub translate_x: f64,
    pub translate_y: f64,
    pub rotate: f64,    // radians
    pub scale_x: f64,
    pub scale_y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            rotate: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

/// 用户光标信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCursor {
    pub user_id: UserId,
    pub position: Point,
    pub color: Color,
    pub name: String,
    pub tool: Tool,
}

/// 绘图工具
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Tool {
    Select,
    Pen { stroke: StrokeStyle },
    Eraser { size: f64 },
    Line { stroke: StrokeStyle },
    Rectangle { stroke: Option<StrokeStyle>, fill: Option<FillStyle> },
    Ellipse { stroke: Option<StrokeStyle>, fill: Option<FillStyle> },
    Text { font_size: f64, font_family: String, color: Color },
    Image { data: Option<String> },
    Pan,
    Zoom,
}

impl Default for Tool {
    fn default() -> Self {
        Tool::Select
    }
}

/// 视口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

/// 画布状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasState {
    pub id: CanvasId,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub background_color: Color,
    pub elements: HashMap<String, Element>,
    pub layers: Vec<Layer>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 图层
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
    pub order: usize,
}

impl Layer {
    pub fn new(name: String, order: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            visible: true,
            locked: false,
            opacity: 1.0,
            order,
        }
    }
}
