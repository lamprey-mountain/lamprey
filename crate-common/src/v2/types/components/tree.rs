// TODO: rename module to tree?

use crate::{
    v1::types::{components::IdAllocator, error::ApiResult},
    v2::types::components::{Component, ComponentId, Components},
};

// pub struct ComponentsTree<'a> {
//     inner: &'a Components,
//     id_allocator: IdAllocator,
// }

// impl<'a> ComponentsTree<'a> {
//     pub fn a() {}
// }

// should i make ComponentsTree own Components?
pub struct ComponentTree {
    inner: Components,
    id_allocator: IdAllocator,
}

/// a reference to a `Component` inside a `ComponentTree`
#[derive(Debug, Clone, Copy)]
pub struct ComponentRef<'c> {
    components: &'c Components,
    component: &'c Component,
}

#[derive(Debug)]
pub struct ComponentRefMut<'c> {
    components: &'c Components,
    component: &'c mut Component,
}

impl ComponentTree {
    /// create a new [`ComponentsTree`] and validate components
    pub fn parse(components: Components) -> ApiResult<Self> {
        let mut id_allocator = IdAllocator::new();
        for c in &components.items {
            id_allocator.mark_used2(c.id.0)?;
        }

        // TODO: validate and minimize

        todo!()
    }

    pub fn get(&self, id: ComponentId) -> Option<ComponentRef<'_>> {
        todo!()
    }

    pub fn get_mut(&mut self, id: ComponentId) -> Option<ComponentRefMut<'_>> {
        todo!()
    }

    // pub fn iter(&self) -> impl Iterator<Item = ComponentRef<'_>> {}
    // pub fn roots(&self) -> impl Iterator<Item = ComponentRef<'_>> {}
    // pub fn is_interactive(&self) -> bool {}

    // pub fn delete(&mut self, id: ComponentId) -> bool {}
    // fn import(&mut self, target: ComponentRef, id_allocator: &mut IdAllocator) -> Component {}
    // pub fn minimize(self) -> Self {}
    // pub fn all_media_ids(&self) -> Vec<MediaId> {}
    // pub fn missing_media_ids(&self) -> Vec<MediaId> {}
    // pub fn as_text(&self) -> Option<&str> {}
}

impl<'c> ComponentRef<'c> {
    // pub fn children(&self) -> impl Iterator<Item = ComponentRef<'c>> {}
    // fn fold_all_children<F, B>(&self, init: B, f: F) -> B {}
    // pub fn is_interactive(&self) -> bool {}
}

impl<'c> ComponentRefMut<'c> {
    // pub fn freeze(&self) -> ComponentRef<'c> {
    //     todo!()
    // }
}

// impl<'c> AsRef<ComponentRef<'c>> for ComponentRefMut<'c> {
//     fn as_ref(&self) -> &ComponentRef<'c> {
//         todo!()
//     }
// }

mod validate {
    //! logic for validating component trees

    use crate::{
        v1::types::{
            Permission, RoomMember, User,
            error::{ApiResult, ErrorField},
            interactions::InteractionCreate,
        },
        v2::types::components::{ComponentType, Components, tree::ComponentTree},
    };

    pub struct ComponentValidator<'a> {
        /// the components being validated
        components: &'a Components,

        /// all errors encountered so far
        errors: Vec<ErrorField>,

        path: Vec<String>,
        // TODO: limit total number of components
        // TODO: limit total text length
        // config: ComponentLimits,
    }

    impl<'a> ComponentValidator<'a> {
        pub fn new(components: &'a Components) -> Self {
            Self {
                path: vec![],
                errors: vec![],
                components,
            }
        }

        fn validate_type(&mut self, ty: &ComponentType) {
            todo!()
        }
    }

    impl ComponentTree {
        /// validate this tree
        pub fn validate(&self) -> ApiResult<()> {
            todo!()
        }

        /// check whether this interaction can be applied to these components
        pub fn allows(&self, interaction: ComponentInteraction) -> ApiResult<()> {
            todo!()
        }
    }

    #[derive(Debug)]
    pub struct ComponentInteraction<'a> {
        interaction_create: &'a InteractionCreate,
        room_member: Option<&'a RoomMember>,
        user: &'a User,
        permissions: &'a [Permission],
    }
}

mod delta {
    //! logic for applying deltas to component trees

    use crate::{
        v1::types::{error::ApiResult, flume::FlumeDelta},
        v2::types::components::tree::ComponentTree,
    };

    impl ComponentTree {
        pub fn patch(&mut self, delta: FlumeDelta) -> ApiResult<()> {
            todo!()
        }

        // pub fn append(&mut self, target_id: ComponentId, other: Components) -> ApiResult<()> {}
        // pub fn replace(&mut self, target_id: ComponentId, replacements: Vec<Component>) -> bool {}
        // pub fn resolve(
        //     self,
        //     prev: Option<Components>,
        //     media: Vec<ComponentMedia>,
        // ) -> ApiResult<Self> {
        // }
    }
}

// how would this work with components with multiple sets of children, eg. details/summary?
#[cfg(any())]
mod cursor {
    pub struct ComponentsCursor<'a> {
        components: &'a Components,
        path: Vec<ComponentId>,
    }

    pub struct ComponentsCursorMut<'a> {
        components: &'a mut Components,
    }

    impl<'a> ComponentsCursor<'a> {
        /// get the current component
        pub fn get(&mut self) -> &'a Component {}

        /// go to the next sibling component
        pub fn next(&mut self) -> Option<&'a Component>;

        /// go to the previous sibling component
        pub fn prev(&mut self) -> Option<&'a Component>;

        /// go to the parent component
        pub fn parent(&mut self) -> Option<&'a Component>;

        /// get the zero-based index of the current component among its siblings
        pub fn index(&mut self) -> Option<usize>;

        /// get the depth of the current component in the tree
        pub fn depth(&mut self) -> Option<usize>;

        // go to the root component
        // iterate over child components
    }

    impl<'a> ComponentsCursorMut<'a> {
        /// remove the current component
        pub fn remove(&mut self);

        /// insert a component after the current component
        pub fn insert(&mut self);
    }
}

#[cfg(any())]
mod builder {
    // maybe? maybe not?
}
