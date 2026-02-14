/// version 1 of the api (still unstable)
pub mod v1;

/// version 2 of the api (also unstable)
// since everything has breaking changes everywhere anyways maybe i'll merge this into v1
pub mod v2;

/// unstable development version that i hack on
pub mod unstable;

// TODO: use Ptr types?
// also consider letting consumers configure between String and Box<str>
#[cfg(feature = "ptr_arc")]
pub type Ptr<T> = Arc<T>;

#[cfg(feature = "ptr_rc")]
pub type Ptr<T> = Rc<T>;

#[cfg(feature = "ptr_box")]
pub type Ptr<T> = Box<T>;

#[cfg(all(
    not(feature = "ptr_arc"),
    not(feature = "ptr_rc"),
    not(feature = "ptr_box")
))]
compile_error!("Enable one of: ptr_arc, ptr_rc, ptr_box");
