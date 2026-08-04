pub enum PlatformCommand {
    /// another platform wants to open a portal to this platform
    PortalRequested,
}

// TODO: maybe implement this trait more?
// pub trait Platform {
//     fn dispatch(&mut self, command: PlatformCommand);
// }

// struct PlatformHandle<P: Platform> {}
