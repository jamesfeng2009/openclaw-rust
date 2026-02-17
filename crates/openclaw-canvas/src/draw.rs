//! 绘图操作模块

use crate::types::*;
use chrono::Utc;
use std::collections::VecDeque;

/// 绘图历史记录
#[derive(Debug, Clone)]
pub struct DrawHistory {
    undo_stack: VecDeque<DrawAction>,
    redo_stack: VecDeque<DrawAction>,
    max_history: usize,
}

impl DrawHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// 记录操作
    pub fn push(&mut self, action: DrawAction) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(action);
        self.redo_stack.clear();
    }

    /// 撤销
    pub fn undo(&mut self) -> Option<DrawAction> {
        if let Some(action) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(action.clone());
            Some(action)
        } else {
            None
        }
    }

    /// 重做
    pub fn redo(&mut self) -> Option<DrawAction> {
        if let Some(action) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(action.clone());
            Some(action)
        } else {
            None
        }
    }

    /// 是否可以撤销
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// 是否可以重做
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for DrawHistory {
    fn default() -> Self {
        Self::new(100)
    }
}

/// 绘图动作
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum DrawAction {
    /// 添加元素
    AddElement { element: Element },
    /// 更新元素
    UpdateElement {
        element_id: String,
        old_state: Element,
        new_state: Element,
    },
    /// 删除元素
    DeleteElement { element: Element },
    /// 批量操作
    Batch { actions: Vec<DrawAction> },
}

/// 绘图工具处理器
pub struct DrawTool;

impl DrawTool {
    /// 开始绘制路径
    pub fn start_path(start: Point, stroke: StrokeStyle, user_id: Option<UserId>) -> Element {
        Element::new(
            Shape::Path {
                points: vec![start],
                stroke,
            },
            user_id,
        )
    }

    /// 继续绘制路径
    pub fn continue_path(element: &mut Element, point: Point) {
        if let Shape::Path { points, .. } = &mut element.shape {
            points.push(point);
            element.updated_at = Utc::now();
        }
    }

    /// 结束绘制路径
    pub fn end_path(element: &mut Element) {
        // 路径绘制完成，可以进行平滑处理
        if let Shape::Path { points, .. } = &mut element.shape {
            // 简化路径点（可选）
            *points = Self::simplify_path(points, 1.0);
        }
    }

    /// 简化路径点
    pub fn simplify_path(points: &[Point], tolerance: f64) -> Vec<Point> {
        if points.len() < 3 {
            return points.to_vec();
        }

        // Douglas-Peucker 算法
        let mut result = Vec::new();
        let mut stack = vec![(0, points.len() - 1)];

        let mut keep = vec![false; points.len()];
        keep[0] = true;
        keep[points.len() - 1] = true;

        while let Some((start, end)) = stack.pop() {
            if end - start < 2 {
                continue;
            }

            let mut max_dist = 0.0;
            let mut max_idx = start;

            for i in (start + 1)..end {
                let dist = Self::point_to_line_distance(&points[i], &points[start], &points[end]);
                if dist > max_dist {
                    max_dist = dist;
                    max_idx = i;
                }
            }

            if max_dist > tolerance {
                keep[max_idx] = true;
                stack.push((start, max_idx));
                stack.push((max_idx, end));
            }
        }

        for (i, &point) in points.iter().enumerate() {
            if keep[i] {
                result.push(point);
            }
        }

        result
    }

    /// 点到线段的距离
    fn point_to_line_distance(point: &Point, line_start: &Point, line_end: &Point) -> f64 {
        let dx = line_end.x - line_start.x;
        let dy = line_end.y - line_start.y;
        let len_sq = dx * dx + dy * dy;

        if len_sq == 0.0 {
            return point.distance_to(line_start);
        }

        let t = ((point.x - line_start.x) * dx + (point.y - line_start.y) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let proj_x = line_start.x + t * dx;
        let proj_y = line_start.y + t * dy;

        let dist_x = point.x - proj_x;
        let dist_y = point.y - proj_y;

        (dist_x * dist_x + dist_y * dist_y).sqrt()
    }

    /// 创建直线
    pub fn create_line(
        start: Point,
        end: Point,
        stroke: StrokeStyle,
        user_id: Option<UserId>,
    ) -> Element {
        Element::new(Shape::Line { start, end, stroke }, user_id)
    }

    /// 创建矩形
    pub fn create_rectangle(
        rect: Rect,
        stroke: Option<StrokeStyle>,
        fill: Option<FillStyle>,
        user_id: Option<UserId>,
    ) -> Element {
        Element::new(Shape::Rectangle { rect, stroke, fill }, user_id)
    }

    /// 创建椭圆
    pub fn create_ellipse(
        center: Point,
        radius_x: f64,
        radius_y: f64,
        stroke: Option<StrokeStyle>,
        fill: Option<FillStyle>,
        user_id: Option<UserId>,
    ) -> Element {
        Element::new(
            Shape::Ellipse {
                center,
                radius_x,
                radius_y,
                stroke,
                fill,
            },
            user_id,
        )
    }

    /// 创建文本
    pub fn create_text(
        position: Point,
        content: String,
        font_size: f64,
        font_family: String,
        color: Color,
        user_id: Option<UserId>,
    ) -> Element {
        Element::new(
            Shape::Text {
                position,
                content,
                font_size,
                font_family,
                color,
            },
            user_id,
        )
    }

    /// 创建图片
    pub fn create_image(
        rect: Rect,
        data: String,
        mime_type: String,
        user_id: Option<UserId>,
    ) -> Element {
        Element::new(
            Shape::Image {
                rect,
                data,
                mime_type,
            },
            user_id,
        )
    }
}

/// 选择器
pub struct Selector;

impl Selector {
    /// 查找指定位置的元素
    pub fn find_element_at(
        elements: &std::collections::HashMap<String, Element>,
        point: &Point,
        layer_filter: Option<usize>,
    ) -> Option<String> {
        // 从上层到下层查找
        let mut candidates: Vec<_> = elements
            .iter()
            .filter(|(_, e)| e.visible)
            .filter(|(_, e)| layer_filter.map_or(true, |l| e.layer == l))
            .collect();

        candidates.sort_by_key(|(_, e)| std::cmp::Reverse(e.layer));

        for (id, element) in candidates {
            if Self::element_contains(element, point) {
                return Some(id.clone());
            }
        }

        None
    }

    /// 判断元素是否包含点
    fn element_contains(element: &Element, point: &Point) -> bool {
        let p = Point::new(
            point.x - element.transform.translate_x,
            point.y - element.transform.translate_y,
        );

        match &element.shape {
            Shape::Path { points, stroke } => {
                for i in 0..points.len().saturating_sub(1) {
                    if Self::point_near_line(&p, &points[i], &points[i + 1], stroke.width / 2.0) {
                        return true;
                    }
                }
                false
            }
            Shape::Line { start, end, stroke } => {
                Self::point_near_line(&p, start, end, stroke.width / 2.0)
            }
            Shape::Rectangle { rect, .. } => {
                let r = Rect::new(
                    rect.x * element.transform.scale_x,
                    rect.y * element.transform.scale_y,
                    rect.width * element.transform.scale_x,
                    rect.height * element.transform.scale_y,
                );
                r.contains(&p)
            }
            Shape::Ellipse {
                center,
                radius_x,
                radius_y,
                ..
            } => {
                let rx = radius_x * element.transform.scale_x;
                let ry = radius_y * element.transform.scale_y;
                let dx = p.x - center.x;
                let dy = p.y - center.y;
                (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) <= 1.0
            }
            Shape::Text {
                position,
                content,
                font_size,
                ..
            } => {
                // 粗略估算文本宽度
                let width = content.len() as f64 * font_size * 0.6;
                let height = font_size * 1.2;
                let rect = Rect::new(position.x, position.y, width, height);
                rect.contains(&p)
            }
            Shape::Image { rect, .. } => {
                let r = Rect::new(
                    rect.x * element.transform.scale_x,
                    rect.y * element.transform.scale_y,
                    rect.width * element.transform.scale_x,
                    rect.height * element.transform.scale_y,
                );
                r.contains(&p)
            }
            Shape::Svg { position, .. } => {
                // SVG 需要更复杂的命中检测，这里简化处理
                let rect = Rect::new(position.x, position.y, 100.0, 100.0);
                rect.contains(&p)
            }
        }
    }

    /// 点是否在线段附近
    fn point_near_line(point: &Point, start: &Point, end: &Point, threshold: f64) -> bool {
        let dist = DrawTool::point_to_line_distance(point, start, end);
        dist <= threshold
    }
}
