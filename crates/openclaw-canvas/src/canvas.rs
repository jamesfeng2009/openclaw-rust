//! 画布核心模块

use crate::types::*;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// 画布错误
#[derive(Debug, Error)]
pub enum CanvasError {
    #[error("画布不存在: {0}")]
    NotFound(CanvasId),

    #[error("元素不存在: {0}")]
    ElementNotFound(String),

    #[error("图层不存在: {0}")]
    LayerNotFound(String),

    #[error("操作被拒绝: {0}")]
    Forbidden(String),

    #[error("无效操作: {0}")]
    InvalidOperation(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Agent 操作类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum AgentAction {
    /// 添加元素
    AddElement {
        element: Element,
    },
    /// 更新元素
    UpdateElement {
        element_id: String,
        updates: ElementUpdate,
    },
    /// 删除元素
    DeleteElement {
        element_id: String,
    },
    /// 移动元素
    MoveElement {
        element_id: String,
        position: Point,
    },
    /// 缩放元素
    ScaleElement {
        element_id: String,
        scale_x: f64,
        scale_y: f64,
    },
    /// 旋转元素
    RotateElement {
        element_id: String,
        angle: f64,
    },
    /// 更改元素颜色
    ChangeColor {
        element_id: String,
        color: Color,
    },
    /// 添加文本
    AddText {
        position: Point,
        text: String,
        font_size: f64,
        color: Color,
    },
    /// 添加图片
    AddImage {
        position: Point,
        url: String,
        width: f64,
        height: f64,
    },
    /// 添加形状
    AddShape {
        shape: Shape,
        position: Point,
    },
    /// 撤销操作
    Undo,
    /// 重做操作
    Redo,
    /// 清空画布
    Clear,
    /// 设置背景色
    SetBackground {
        color: Color,
    },
}

/// 画布管理器
pub struct CanvasManager {
    canvases: Arc<RwLock<HashMap<CanvasId, Arc<RwLock<CanvasState>>>>>,
}

impl CanvasManager {
    /// 创建新的画布管理器
    pub fn new() -> Self {
        Self {
            canvases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新画布
    pub async fn create_canvas(&self, name: String, width: f64, height: f64) -> CanvasId {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let canvas = CanvasState {
            id: id.clone(),
            name,
            width,
            height,
            background_color: Color::white(),
            elements: HashMap::new(),
            layers: vec![Layer::new("Layer 1".to_string(), 0)],
            created_at: now,
            updated_at: now,
        };

        let mut canvases = self.canvases.write().await;
        canvases.insert(id.clone(), Arc::new(RwLock::new(canvas)));

        info!("创建画布: {}", id);
        id
    }

    /// 获取画布
    pub async fn get_canvas(&self, id: &CanvasId) -> Option<Arc<RwLock<CanvasState>>> {
        let canvases = self.canvases.read().await;
        canvases.get(id).cloned()
    }

    /// 获取画布状态
    pub async fn get_canvas_state(&self, id: &CanvasId) -> Result<CanvasState, CanvasError> {
        let canvas = self
            .get_canvas(id)
            .await
            .ok_or_else(|| CanvasError::NotFound(id.clone()))?;
        let state = canvas.read().await;
        Ok(state.clone())
    }

    /// 删除画布
    pub async fn delete_canvas(&self, id: &CanvasId) -> Result<(), CanvasError> {
        let mut canvases = self.canvases.write().await;
        if canvases.remove(id).is_some() {
            info!("删除画布: {}", id);
            Ok(())
        } else {
            Err(CanvasError::NotFound(id.clone()))
        }
    }

    /// 列出所有画布
    pub async fn list_canvases(&self) -> Vec<CanvasInfo> {
        let canvases = self.canvases.read().await;
        canvases
            .values()
            .map(|c| {
                let rt = tokio::runtime::Handle::current();
                let state = rt.block_on(async { c.read().await });
                CanvasInfo {
                    id: state.id.clone(),
                    name: state.name.clone(),
                    element_count: state.elements.len(),
                    created_at: state.created_at,
                    updated_at: state.updated_at,
                }
            })
            .collect()
    }
}

impl Default for CanvasManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 画布信息摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct CanvasInfo {
    pub id: CanvasId,
    pub name: String,
    pub element_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 画布操作
pub struct CanvasOps;

impl CanvasOps {
    /// 添加元素
    pub async fn add_element(
        canvas: &Arc<RwLock<CanvasState>>,
        element: Element,
    ) -> Result<String, CanvasError> {
        let id = element.id.clone();
        let mut state = canvas.write().await;
        state.elements.insert(id.clone(), element);
        state.updated_at = Utc::now();
        debug!("添加元素: {}", id);
        Ok(id)
    }

    /// 更新元素
    pub async fn update_element(
        canvas: &Arc<RwLock<CanvasState>>,
        element_id: &str,
        updates: ElementUpdate,
    ) -> Result<(), CanvasError> {
        let mut state = canvas.write().await;

        let element = state
            .elements
            .get_mut(element_id)
            .ok_or_else(|| CanvasError::ElementNotFound(element_id.to_string()))?;

        if element.locked {
            return Err(CanvasError::Forbidden("元素已锁定".to_string()));
        }

        if let Some(shape) = updates.shape {
            element.shape = shape;
        }
        if let Some(layer) = updates.layer {
            element.layer = layer;
        }
        if let Some(opacity) = updates.opacity {
            element.opacity = opacity;
        }
        if let Some(transform) = updates.transform {
            element.transform = transform;
        }
        if let Some(visible) = updates.visible {
            element.visible = visible;
        }

        element.updated_at = Utc::now();
        state.updated_at = Utc::now();

        debug!("更新元素: {}", element_id);
        Ok(())
    }

    /// 删除元素
    pub async fn delete_element(
        canvas: &Arc<RwLock<CanvasState>>,
        element_id: &str,
    ) -> Result<(), CanvasError> {
        let mut state = canvas.write().await;

        if state.elements.remove(element_id).is_some() {
            state.updated_at = Utc::now();
            debug!("删除元素: {}", element_id);
            Ok(())
        } else {
            Err(CanvasError::ElementNotFound(element_id.to_string()))
        }
    }

    /// 移动元素
    pub async fn move_element(
        canvas: &Arc<RwLock<CanvasState>>,
        element_id: &str,
        dx: f64,
        dy: f64,
    ) -> Result<(), CanvasError> {
        let mut state = canvas.write().await;

        let element = state
            .elements
            .get_mut(element_id)
            .ok_or_else(|| CanvasError::ElementNotFound(element_id.to_string()))?;

        element.transform.translate_x += dx;
        element.transform.translate_y += dy;
        element.updated_at = Utc::now();
        state.updated_at = Utc::now();

        Ok(())
    }

    /// 清空画布
    pub async fn clear_canvas(canvas: &Arc<RwLock<CanvasState>>) -> Result<(), CanvasError> {
        let mut state = canvas.write().await;
        state.elements.clear();
        state.updated_at = Utc::now();
        info!("清空画布: {}", state.id);
        Ok(())
    }

    /// 添加图层
    pub async fn add_layer(
        canvas: &Arc<RwLock<CanvasState>>,
        name: String,
    ) -> Result<String, CanvasError> {
        let mut state = canvas.write().await;
        let order = state.layers.len();
        let layer = Layer::new(name, order);
        let id = layer.id.clone();
        state.layers.push(layer);
        state.updated_at = Utc::now();
        Ok(id)
    }

    /// 删除图层
    pub async fn delete_layer(
        canvas: &Arc<RwLock<CanvasState>>,
        layer_id: &str,
    ) -> Result<(), CanvasError> {
        let mut state = canvas.write().await;

        if state.layers.len() <= 1 {
            return Err(CanvasError::InvalidOperation("至少需要一个图层".to_string()));
        }

        let layer_idx = state
            .layers
            .iter()
            .position(|l| l.id == layer_id)
            .ok_or_else(|| CanvasError::LayerNotFound(layer_id.to_string()))?;

        // 删除该图层上的所有元素
        let layer_id_clone = layer_id.to_string();
        state.elements.retain(|_, e| {
            e.layer != layer_idx
        });

        state.layers.remove(layer_idx);
        state.updated_at = Utc::now();

        Ok(())
    }
}
