// TODO: remove this in favor of the proc macro?
// i guess theres no harm in supporting a builder as well

use crate::{
    v1::types::{components::IdAllocator, metadata::Metadata, misc::Color},
    v2::types::components::{
        Component, ComponentId, ComponentType, Components,
        acl::Allow,
        action::ButtonAction,
        interactive::{ButtonStyle, Label},
    },
};

/// utility to build `Components`
pub struct ComponentsBuilder {
    id_alloc: IdAllocator,
    inner: Vec<Component>,
    roots: Vec<ComponentId>,
    metadata: Metadata,
}

pub struct Builder<'builder> {
    builder: &'builder mut ComponentsBuilder,
    children: Vec<ComponentId>,
}

impl Components {
    pub fn builder() -> ComponentsBuilder {
        ComponentsBuilder::new()
    }
}

impl ComponentsBuilder {
    pub fn new() -> Self {
        Self::with_id_alloc(IdAllocator::new())
    }

    pub fn with_id_alloc(id_alloc: IdAllocator) -> Self {
        Self {
            id_alloc,
            inner: Vec::new(),
            roots: Vec::new(),
            metadata: Metadata::default(),
        }
    }

    /// Add components to the root
    pub fn root<F>(mut self, f: F) -> Self
    where
        F: for<'a> ContainerFn<'a>,
    {
        let components = {
            let mut b = Builder {
                builder: &mut self,
                children: Vec::new(),
            };
            f.call_once(&mut b);
            b.children
        };

        self.roots.extend(components);
        self
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn build(self) -> Components {
        Components {
            roots: self.roots,
            items: self.inner,
            media: vec![],
            metadata: self.metadata,
        }
    }
}

impl<'builder> Builder<'builder> {
    /// Create a sub-builder borrowing from the current allocator and context
    fn fork(&mut self) -> Builder<'_> {
        Builder {
            builder: &mut *self.builder,
            children: Vec::new(),
        }
    }

    /// Create a component and append it to this container
    pub fn child(
        &mut self,
        ty: ComponentType,
        allow: Option<Allow>,
        id: Option<ComponentId>,
    ) -> &mut Self {
        let id = self.builder.id_alloc.allocate(id);
        self.builder.inner.push(Component { id, ty, allow });
        self.children.push(id);
        self
    }

    /// Append a pre-constructed component directly
    pub fn child_component(&mut self, component: Component) -> &mut Self {
        let id = component.id;
        self.builder.id_alloc.mark_used(id.0);
        self.builder.inner.push(component);
        self.children.push(id);
        self
    }

    /// Append markdown text to this context
    pub fn text(&mut self, content: impl Into<String>) -> &mut Self {
        self.child(
            ComponentType::Text {
                content: content.into(),
            },
            None,
            None,
        )
    }

    /// Append a button to this context
    pub fn button(
        &mut self,
        label: impl Into<Label>,
        style: ButtonStyle,
        action: ButtonAction,
    ) -> &mut Self {
        self.child(
            ComponentType::Button {
                label: label.into(),
                style,
                action,
            },
            None,
            None,
        )
    }

    /// Append a container with nested components
    pub fn container<F>(&mut self, color: Option<Color>, f: F) -> &mut Self
    where
        F: for<'a> ContainerFn<'a>,
    {
        let components = {
            let mut b = self.fork();
            f.call_once(&mut b);
            b.children
        };

        self.child(ComponentType::Container { components, color }, None, None)
    }

    /// Append a section without margins/padding
    pub fn section<F>(&mut self, color: Option<Color>, f: F) -> &mut Self
    where
        F: for<'a> ContainerFn<'a>,
    {
        let components = {
            let mut b = self.fork();
            f.call_once(&mut b);
            b.children
        };

        self.child(ComponentType::Section { components, color }, None, None)
    }
}

/// trait to allow closures to return any type
pub trait ContainerFn<'a> {
    type Output;
    fn call_once(self, builder: &'a mut Builder<'_>) -> Self::Output;
}

impl<'a, F, O> ContainerFn<'a> for F
where
    F: FnOnce(&'a mut Builder<'_>) -> O,
{
    type Output = O;

    #[inline]
    fn call_once(self, builder: &'a mut Builder<'_>) -> Self::Output {
        self(builder)
    }
}

// reexport macro
pub use lamprey_macros::components;
