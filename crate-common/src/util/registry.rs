use utoipa::ToSchema;

/// a type registry visitor
pub trait Registry {
    /// register a type
    #[cfg(feature = "utoipa")]
    fn register<T: ToSchema>(&mut self);

    /// register a type
    #[cfg(not(feature = "utoipa"))]
    fn register<T>(&mut self);
}

// TODO: export all models

macro_rules! export_models {
    (@item $r:ident; ) => {};

    (@item $r:ident; use $path:path, $($rest:tt)*) => {
        {
            use $path as __export_models_target;
            __export_models_target::register($r);
        }
        export_models!(@item $r; $($rest)*);
    };

    (@item $r:ident; use $path:path) => {
        {
            use $path as __export_models_target;
            __export_models_target::register($r);
        }
    };

    (@item $r:ident; $model:ident, $($rest:tt)*) => {
        $r.register::<$model>();
        export_models!(@item $r; $($rest)*);
    };

    (@item $r:ident; $model:ident) => {
        $r.register::<$model>();
    };

    ($($item:tt)*) => {
        pub fn register<R: crate::util::registry::Registry>(r: &mut R) {
            export_models!(@item r; $($item)*);
        }
    };
}

pub(crate) use export_models;
