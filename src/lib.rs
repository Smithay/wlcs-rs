//!
//! wlcs Bindings for Rust
//!

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use std::os::fd::OwnedFd;

use wayland_sys::{
    client::{wl_display, wl_proxy},
    common::wl_fixed_t,
    server::wl_event_loop,
};

use crate::ffi_display_server_api::WlcsIntegrationDescriptor;

pub mod ffi_display_server_api;
pub mod ffi_pointer_api;
pub mod ffi_touch_api;
pub mod ffi_wrappers;

/// Build WLCS extension extension_list
///
/// # Arguments
///
/// * `name` - The name of the Wayland protocol
/// * `version` - Version number of the corresponding protocol
///
/// # Examples
/// use wlcs_rs::extension_list
///
/// static SUPPORTED_EXTENSIONS: &[ffi_display_server_api::WlcsExtensionDescriptor] = extension_list!(
///     ("wl_compositor", 4),
///     ("wl_subcompositor", 1),
///     ("wl_data_device_manager", 3),
///     ("wl_seat", 7),
///     ("wl_output", 4),
///     ("xdg_wm_base", 3),
/// );
///
/// static DESCRIPTOR: WlcsIntegrationDescriptor = WlcsIntegrationDescriptor {
///     version: 1,
///     num_extensions: SUPPORTED_EXTENSIONS.len(),
///     supported_extensions: SUPPORTED_EXTENSIONS.as_ptr(),
/// };
#[macro_export]
macro_rules! extension_list {
    ($(($name: expr, $version: expr)),* $(,)?) => {
        &[$(
            WlcsExtensionDescriptor {
                name: concat!($name, "\0").as_ptr() as *const std::os::raw::c_char,
                version: $version
            }
        ),*]
    };
}

/// Trait to be implemented by Wlcs clients
pub trait Wlcs {
    /// The pointer type is what will be implemented and called by [`Wlcs::create_pointer`]
    type Pointer: Pointer;

    /// The touch type is what will be implemented and called by [`Wlcs::create_touch`]
    type Touch: Touch;

    /// .
    fn new() -> Self;

    /// Start the display server
    fn start(&mut self);

    /// Stop the display server
    fn stop(&mut self);

    /// Create a socket for a Wayland client.
    fn create_client_socket(&self) -> OwnedFd;

    /// Position a window in absolute coordinates
    fn position_window_absolute(
        &self,
        display: *mut wl_display,
        surface: *mut wl_proxy,
        x: i32,
        y: i32,
    );

    /// Create a wl_pointer
    fn create_pointer(&mut self) -> Option<Self::Pointer>;

    /// Create a wl_touch
    fn create_touch(&mut self) -> Option<Self::Touch>;

    /// Get the Integration descriptor
    fn get_descriptor(&self) -> &WlcsIntegrationDescriptor;

    /// Option current thread startup
    fn start_on_this_thread(&self, _event_loop: *mut wl_event_loop) {}
}

/// Trait for Wlcs clients implementing Pointer testing
pub trait Pointer {
    /// Absolute pointer movement event
    fn move_absolute(&mut self, x: wl_fixed_t, y: wl_fixed_t);

    /// Relative pointer movement event
    fn move_relative(&mut self, dx: wl_fixed_t, dy: wl_fixed_t);

    /// Release of button
    fn button_up(&mut self, button: i32);

    /// Press of button
    fn button_down(&mut self, button: i32);

    /// Destroy the pointer handle.
    fn destroy(&mut self) {}
}

/// Trait for Wlcs clients implementing Pointer testing
pub trait Touch {
    /// Start of a touch event
    fn touch_down(&mut self, x: wl_fixed_t, y: wl_fixed_t);

    /// A "drag" event
    fn touch_move(&mut self, x: wl_fixed_t, y: wl_fixed_t);

    /// Event that bookends touch_down
    fn touch_up(&mut self);

    /// Destroy a touch handle
    fn destroy(&mut self) {}
}
