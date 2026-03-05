//! 槽位管理模块

use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Slot {
    pub name: String,
    pub value: SlotValue,
    pub slot_type: SlotType,
    pub is_required: bool,
    pub filled: bool,
    pub elicit_on_empty: bool,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SlotValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    List(Vec<String>),
    Entity(EntityValue),
}

#[derive(Debug, Clone)]
pub enum SlotType {
    Text,
    Number,
    Boolean,
    DateTime,
    List,
    Entity(EntityType),
}

#[derive(Debug, Clone)]
pub enum EntityType {
    Person,
    Location,
    Organization,
    Date,
    Time,
    Number,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct EntityValue {
    pub name: String,
    pub value: String,
    pub entity_type: EntityType,
}

impl Slot {
    pub fn new(name: String, slot_type: SlotType) -> Self {
        Self {
            name,
            value: SlotValue::Text(String::new()),
            slot_type,
            is_required: false,
            filled: false,
            elicit_on_empty: false,
            prompt: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.is_required = true;
        self
    }

    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt = Some(prompt.to_string());
        self
    }

    pub fn fill(&mut self, value: SlotValue) -> bool {
        if self.validate_value(&value) {
            self.value = value;
            self.filled = true;
            true
        } else {
            false
        }
    }

    pub fn validate_value(&self, value: &SlotValue) -> bool {
        match (&self.slot_type, value) {
            (SlotType::Text, SlotValue::Text(_)) => true,
            (SlotType::Number, SlotValue::Number(_)) => true,
            (SlotType::Boolean, SlotValue::Boolean(_)) => true,
            (SlotType::DateTime, SlotValue::DateTime(_)) => true,
            (SlotType::List, SlotValue::List(_)) => true,
            (SlotType::Entity(_), SlotValue::Entity(_)) => true,
            _ => false,
        }
    }

    pub fn clear(&mut self) {
        self.value = SlotValue::Text(String::new());
        self.filled = false;
    }

    pub fn get_prompt(&self) -> Option<&str> {
        if !self.filled && self.elicit_on_empty {
            self.prompt.as_deref()
        } else {
            None
        }
    }
}

pub struct SlotManager {
    slots: HashMap<String, Slot>,
}

impl SlotManager {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn add_slot(mut self, slot: Slot) -> Self {
        self.slots.insert(slot.name.clone(), slot);
        self
    }

    pub fn register_slot(&mut self, slot: Slot) {
        self.slots.insert(slot.name.clone(), slot);
    }

    pub fn fill_slot(&mut self, name: &str, value: SlotValue) -> bool {
        if let Some(slot) = self.slots.get_mut(name) {
            slot.fill(value)
        } else {
            false
        }
    }

    pub fn get_slot(&self, name: &str) -> Option<&Slot> {
        self.slots.get(name)
    }

    pub fn get_slot_mut(&mut self, name: &str) -> Option<&mut Slot> {
        self.slots.get_mut(name)
    }

    pub fn is_all_filled(&self) -> bool {
        self.slots.values().all(|s| !s.is_required || s.filled)
    }

    pub fn get_missing_slots(&self) -> Vec<&Slot> {
        self.slots
            .values()
            .filter(|s| s.is_required && !s.filled)
            .collect()
    }

    pub fn get_prompts(&self) -> Vec<String> {
        self.slots
            .values()
            .filter_map(|s| s.get_prompt().map(|p| p.to_string()))
            .collect()
    }

    pub fn clear_slot(&mut self, name: &str) {
        if let Some(slot) = self.slots.get_mut(name) {
            slot.clear();
        }
    }

    pub fn clear_all(&mut self) {
        for slot in self.slots.values_mut() {
            slot.clear();
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn filled_count(&self) -> usize {
        self.slots.values().filter(|s| s.filled).count()
    }

    pub fn get_all_slots(&self) -> &HashMap<String, Slot> {
        &self.slots
    }
}

impl Default for SlotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_creation() {
        let slot = Slot::new("name".to_string(), SlotType::Text);
        assert_eq!(slot.name, "name");
        assert!(!slot.filled);
    }

    #[test]
    fn test_slot_fill() {
        let mut slot = Slot::new("name".to_string(), SlotType::Text);
        assert!(slot.fill(SlotValue::Text("John".to_string())));
        assert!(slot.filled);
    }

    #[test]
    fn test_slot_fill_invalid() {
        let mut slot = Slot::new("age".to_string(), SlotType::Number);
        assert!(!slot.fill(SlotValue::Text("not a number".to_string())));
        assert!(!slot.filled);
    }

    #[test]
    fn test_slot_required() {
        let slot = Slot::new("name".to_string(), SlotType::Text).required();
        assert!(slot.is_required);
    }

    #[test]
    fn test_slot_manager_new() {
        let manager = SlotManager::new();
        assert_eq!(manager.slot_count(), 0);
    }

    #[test]
    fn test_slot_manager_add_slot() {
        let manager = SlotManager::new()
            .add_slot(Slot::new("name".to_string(), SlotType::Text));
        
        assert_eq!(manager.slot_count(), 1);
    }

    #[test]
    fn test_slot_manager_fill_slot() {
        let mut manager = SlotManager::new()
            .add_slot(Slot::new("name".to_string(), SlotType::Text));
        
        assert!(manager.fill_slot("name", SlotValue::Text("John".to_string())));
        
        let slot = manager.get_slot("name").unwrap();
        assert!(slot.filled);
    }

    #[test]
    fn test_slot_manager_is_all_filled() {
        let mut manager = SlotManager::new()
            .add_slot(Slot::new("name".to_string(), SlotType::Text).required());
        
        assert!(!manager.is_all_filled());
        
        manager.fill_slot("name", SlotValue::Text("John".to_string()));
        
        assert!(manager.is_all_filled());
    }

    #[test]
    fn test_slot_manager_get_missing_slots() {
        let mut manager = SlotManager::new()
            .add_slot(Slot::new("name".to_string(), SlotType::Text).required())
            .add_slot(Slot::new("age".to_string(), SlotType::Number).required());
        
        manager.fill_slot("name", SlotValue::Text("John".to_string()));
        
        let missing = manager.get_missing_slots();
        assert_eq!(missing.len(), 1);
    }
}
