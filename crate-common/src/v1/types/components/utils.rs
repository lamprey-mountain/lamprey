use crate::v1::types::{
    components::{ComponentId, MAX_COMPONENTS, MAX_DEPTH, MAX_TOTAL_TEXT_LENGTH},
    error::{ErrorField, ErrorFieldType},
};

/// Tracks validation state across the component tree.
pub struct ValidationState {
    path: Vec<String>,
    component_count: usize,
    depth: usize,
    total_text_length: usize,
    pub errors: Vec<ErrorField>,
}

impl ValidationState {
    /// Create a new validation state.
    pub fn new() -> ValidationState {
        ValidationState {
            path: vec![],
            component_count: 0,
            depth: 1,
            total_text_length: 0,
            errors: vec![],
        }
    }

    pub fn enter<F>(&mut self, segment: impl Into<String>, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.depth += 1;
        self.path.push(segment.into());
        f(self);
        self.path.pop();
        self.depth -= 1;
    }

    pub fn enter_index<F>(&mut self, index: usize, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.enter(index.to_string(), f)
    }

    pub fn push_error(&mut self, message: String, ty: ErrorFieldType) {
        self.errors.push(ErrorField {
            key: self.path.clone(),
            message,
            ty,
        });
    }

    pub fn check_component(&mut self) {
        self.component_count += 1;
        if self.component_count > MAX_COMPONENTS {
            self.push_error(
                format!("component tree exceeds maximum of {MAX_COMPONENTS} components"),
                ErrorFieldType::length(1, MAX_COMPONENTS as u64),
            );
        }
        if self.depth > MAX_DEPTH {
            self.push_error(
                format!("component tree exceeds maximum depth of {MAX_DEPTH}"),
                ErrorFieldType::length(1, MAX_DEPTH as u64),
            );
        }
    }

    pub fn add_text(&mut self, len: usize) {
        self.total_text_length += len;
        if self.total_text_length > MAX_TOTAL_TEXT_LENGTH {
            self.push_error(
                format!("total text length exceeds maximum of {MAX_TOTAL_TEXT_LENGTH} bytes"),
                ErrorFieldType::length(1, MAX_TOTAL_TEXT_LENGTH as u64),
            );
        }
    }

    pub fn validate_count(&mut self, count: usize, min: usize, max: usize, field: &str) {
        if count < min {
            self.push_error(
                format!("{field} cannot be empty"),
                ErrorFieldType::length(min as u64, max as u64),
            );
        } else if count > max {
            self.push_error(
                format!("{field} can have up to {max}"),
                ErrorFieldType::length(min as u64, max as u64),
            );
        }
    }

    pub fn validate_length(&mut self, value: &str, min: usize, max: usize, field: &str) {
        if value.is_empty() {
            self.push_error(
                format!("{field} cannot be empty"),
                ErrorFieldType::length(min as u64, max as u64),
            );
        } else if value.len() > max {
            self.push_error(
                format!("{field} can have up to {max} chars"),
                ErrorFieldType::length(min as u64, max as u64),
            );
        }
    }
}

pub struct IdAllocator {
    next_id: u16,
    used: std::collections::HashSet<u16>,
}

impl IdAllocator {
    /// Create a new ID allocator.
    pub fn new() -> Self {
        Self {
            next_id: 0,
            used: std::collections::HashSet::new(),
        }
    }

    /// Mark an ID as used.
    pub fn mark_used(&mut self, id: u16) {
        self.used.insert(id);
    }

    /// Allocate a new ID or use the requested one.
    pub fn allocate(&mut self, requested: Option<ComponentId>) -> ComponentId {
        if let Some(id) = requested {
            self.mark_used(id.0);
            return id;
        }

        while self.used.contains(&self.next_id) {
            if self.next_id == u16::MAX {
                // Should never happen given MAX_COMPONENTS = 64,
                // but needed for memory safety.
                panic!("Component ID namespace exhausted");
            }
            self.next_id += 1;
        }

        let id = ComponentId(self.next_id);
        self.mark_used(self.next_id);
        self.next_id += 1;
        id
    }
}
