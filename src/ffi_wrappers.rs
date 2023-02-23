//!
//! Wrappers/helpers for setting up WLCS server integration
//!

use std::{ffi::{c_char, c_int}, os::fd::IntoRawFd};

use container_of::container_of;
use wayland_sys::{
    client::{wl_display, wl_proxy},
    common::wl_fixed_t,
    server::wl_event_loop,
};

use crate::{
    ffi_display_server_api::{WlcsDisplayServer, WlcsIntegrationDescriptor, WlcsServerIntegration},
    ffi_pointer_api::WlcsPointer,
    ffi_touch_api::WlcsTouch,
    Pointer, Touch, Wlcs,
};

struct DisplayServerHandle<W: Wlcs> {
    wlcs_display_server: WlcsDisplayServer,
    wlcs: W,
}

struct PointerHandle<W: Wlcs> {
    wlcs_pointer: WlcsPointer,
    p: W::Pointer,
}

struct TouchHandle<W: Wlcs> {
    wlcs_touch: WlcsTouch,
    t: W::Touch,
}

/// Helper function for getting a [`DisplayServerHandle`] from a [`WlcsDisplayServer`] pointer.
///
/// # Safety
///
/// - The pointer must be valid
/// - The library must have initialized the pointer as an instance of a [`DisplayServerHandle`].
/// - The caller has picked a suitable lifetime to ensure the returned mutable reference is not held when
///   control is returned to wlcs
unsafe fn get_display_server_handle_mut<'a, W: Wlcs>(
    ptr: *mut WlcsDisplayServer,
) -> &'a mut DisplayServerHandle<W> {
    unsafe { &mut *container_of!(ptr, DisplayServerHandle<W>, wlcs_display_server) }
}

/// Helper function for getting a [`DisplayServerHandle`] from a [`WlcsDisplayServer`] pointer.
///
/// # Safety
///
/// - The pointer must be valid
/// - The library must have initialized the pointer as an instance of a [`DisplayServerHandle`].
/// - The caller has picked a suitable lifetime to ensure the returned reference is not held when control is
///   returned to wlcs
unsafe fn get_display_server_handle_ref<'a, W: Wlcs>(
    ptr: *const WlcsDisplayServer,
) -> &'a DisplayServerHandle<W> {
    unsafe { &*container_of!(ptr, DisplayServerHandle<W>, wlcs_display_server) }
}

unsafe fn get_pointer_handle<'a, W: Wlcs>(ptr: *mut WlcsPointer) -> &'a mut PointerHandle<W> {
    unsafe { &mut *container_of!(ptr, PointerHandle<W>, wlcs_pointer) }
}

unsafe fn get_touch_handle<'a, W: Wlcs>(ptr: *mut WlcsTouch) -> &'a mut TouchHandle<W> {
    unsafe { &mut *container_of!(ptr, TouchHandle<W>, wlcs_touch) }
}

#[allow(unused)]
unsafe extern "C" fn create_server_ffi<W: Wlcs>(
    _argc: c_int,
    _argv: *mut *const c_char,
) -> *mut WlcsDisplayServer {
    // block the SIGPIPE signal here, we are a cdynlib so Rust does not do it for us
    match std::panic::catch_unwind(|| {
        use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};

        unsafe {
            sigaction(
                Signal::SIGPIPE,
                &SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty()),
            )
            .unwrap();
        }

        let wlcs = W::new();
        let dsh = Box::new(DisplayServerHandle {
            wlcs_display_server: wlcs_display_server::<W>(),
            wlcs,
        });
        let handle = Box::into_raw(dsh);
        std::ptr::addr_of_mut!((*handle).wlcs_display_server)
    }) {
        Ok(ptr) => ptr,
        Err(err) => {
            println!(
                "panic in create_server_ffi on ptr: {:p} (type {:?})",
                err.as_ref() as *const _,
                err.type_id()
            );
            std::ptr::null_mut()
        }
    }
}

#[allow(unused)]
unsafe extern "C" fn destroy_server_ffi<W: Wlcs>(ptr: *mut WlcsDisplayServer) {
    if let Err(err) = std::panic::catch_unwind(|| {
        // SAFETY:
        // - wlcs will no longer use the WlcsDisplayServer pointer. This ensures we take back ownership of the
        //   allocation.
        // - The DisplayServerHandle was created using Box::from_raw, ensuring the memory layout is correct.
        let _server = unsafe {
            Box::from_raw(container_of!(
                ptr,
                DisplayServerHandle::<W>,
                wlcs_display_server
            ))
        };
        assert_eq!(_server.wlcs_display_server.version, 3);
    }) {
        println!(
            "panic in destroy_server_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

#[allow(unused)]
unsafe extern "C" fn start_server_ffi<W: Wlcs>(ptr: *mut WlcsDisplayServer) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        server.wlcs.start()
    }) {
        println!(
            "panic in start_server_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

#[allow(unused)]
unsafe extern "C" fn stop_server_ffi<W: Wlcs>(ptr: *mut WlcsDisplayServer) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        server.wlcs.stop();
    }) {
        println!(
            "panic in stop_server_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

#[allow(unused)]
unsafe extern "C" fn create_client_socket_ffi<W: Wlcs>(ptr: *mut WlcsDisplayServer) -> c_int {
    match std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        server.wlcs.create_client_socket()
    }) {
        // WLCS takes ownership of the file descriptor for the client socket.
        Ok(ret) => ret.into_raw_fd(),
        Err(err) => {
            println!(
                "panic in wlcs_display_server::create_client_socket_ffi on ptr: {:p} (type {:?})",
                err.as_ref() as *const _,
                err.type_id()
            );
            -1
        }
    }
}

unsafe extern "C" fn position_window_absolute_ffi<W: Wlcs>(
    ptr: *mut WlcsDisplayServer,
    display: *mut wl_display,
    surface: *mut wl_proxy,
    x: c_int,
    y: c_int,
) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        server.wlcs.position_window_absolute(display, surface, x, y);
    }) {
        println!(
            "panic in wlcs_display_server::position_window_absolute_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

#[allow(unused)]
unsafe extern "C" fn create_pointer_ffi<W: Wlcs>(ptr: *mut WlcsDisplayServer) -> *mut WlcsPointer {
    match std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        let Some(p) = server.wlcs.create_pointer() else { return std::ptr::null_mut() };

        let handle: *mut PointerHandle<W> = Box::into_raw(Box::new(PointerHandle {
            wlcs_pointer: wlcs_pointer::<W>(),
            p,
        }));
        std::ptr::addr_of_mut!((*handle).wlcs_pointer)
    }) {
        Ok(ptr) => ptr,
        Err(err) => {
            println!(
                "panic in wlcs_display_server::create_pointer_ffi on ptr: {:p} (type {:?})",
                err.as_ref() as *const _,
                err.type_id()
            );
            std::ptr::null_mut()
        }
    }
}

#[allow(unused)]
unsafe extern "C" fn create_touch_ffi<W: Wlcs>(ptr: *mut WlcsDisplayServer) -> *mut WlcsTouch {
    match std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        let Some(t) = server.wlcs.create_touch() else { return std::ptr::null_mut(); };
        let handle: *mut TouchHandle<W> = Box::into_raw(Box::new(TouchHandle {
            wlcs_touch: wlcs_touch::<W>(),
            t,
        }));
        std::ptr::addr_of_mut!((*handle).wlcs_touch)
    }) {
        Ok(ptr) => ptr,
        Err(err) => {
            println!(
                "panic in wlcs_display_server::create_touch_ffi on ptr: {:p} (type {:?})",
                err.as_ref() as *const _,
                err.type_id()
            );
            std::ptr::null_mut()
        }
    }
}

#[allow(unused)]
unsafe extern "C" fn get_descriptor_ffi<W: Wlcs>(
    ptr: *const WlcsDisplayServer,
) -> *const WlcsIntegrationDescriptor {
    match std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_ref::<W>(ptr) };
        server.wlcs.get_descriptor()
    }) {
        Ok(ptr) => ptr as *const WlcsIntegrationDescriptor,
        Err(err) => {
            println!(
                "panic in wlcs_display_server::get_descriptor_ffi on ptr: {:p} (type {:?})",
                err.as_ref() as *const _,
                err.type_id()
            );
            std::ptr::null_mut()
        }
    }
}

#[allow(unused)]
unsafe extern "C" fn start_on_this_thread_ffi<W: Wlcs>(
    ptr: *mut WlcsDisplayServer,
    event_loop: *mut wl_event_loop,
) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let server = unsafe { get_display_server_handle_mut::<W>(ptr) };
        assert_eq!(server.wlcs_display_server.version, 3);
        server.wlcs.start_on_this_thread(event_loop)
    }) {
        println!(
            "panic in start_on_this_thread_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

const fn wlcs_display_server<W: Wlcs>() -> WlcsDisplayServer {
    WlcsDisplayServer {
        version: 3,
        start: Some(start_server_ffi::<W>),
        stop: Some(stop_server_ffi::<W>),
        create_client_socket: Some(create_client_socket_ffi::<W>),
        position_window_absolute: Some(position_window_absolute_ffi::<W>),
        create_pointer: Some(create_pointer_ffi::<W>),
        create_touch: Some(create_touch_ffi::<W>),
        get_descriptor: Some(get_descriptor_ffi::<W>),
        start_on_this_thread: Some(start_on_this_thread_ffi::<W>),
    }
}

/// Instantiate the WlcsServerIntegration for WLCS FFI.
///
/// This should not be called directly. Instead [`crate::wlcs_server_integration!`] should be used
#[allow(unused)]
pub const fn wlcs_server<W>() -> WlcsServerIntegration
where
    W: Wlcs,
{
    WlcsServerIntegration {
        version: 1,
        create_server: Some(create_server_ffi::<W>),
        destroy_server: Some(destroy_server_ffi::<W>),
    }
}

/// Instantiate the WlcsServerIntegration for the specified type.
///
/// See [`Wlcs`] trait.
#[macro_export]
macro_rules! wlcs_server_integration {
    ($handle: ident) => {
        #[no_mangle]
        static wlcs_server_integration: WlcsServerIntegration = wlcs_server::<$handle>();
    };
}

unsafe extern "C" fn pointer_move_absolute_ffi<W: Wlcs>(
    ptr: *mut WlcsPointer,
    x: wl_fixed_t,
    y: wl_fixed_t,
) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let pointer = unsafe { get_pointer_handle::<W>(ptr) };
        pointer.p.move_absolute(x, y);
    }) {
        println!(
            "panic in pointer_move_absolute_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn pointer_move_relative_ffi<W: Wlcs>(
    ptr: *mut WlcsPointer,
    dx: wl_fixed_t,
    dy: wl_fixed_t,
) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let pointer = unsafe { get_pointer_handle::<W>(ptr) };
        pointer.p.move_relative(dx, dy);
    }) {
        println!(
            "panic in pointer_move_relative_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn pointer_button_up_ffi<W: Wlcs>(ptr: *mut WlcsPointer, button: i32) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let pointer = unsafe { get_pointer_handle::<W>(ptr) };
        pointer.p.button_up(button)
    }) {
        println!(
            "panic in pointer_button_up_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn pointer_button_down_ffi<W: Wlcs>(ptr: *mut WlcsPointer, button: i32) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let pointer = unsafe { get_pointer_handle::<W>(ptr) };
        pointer.p.button_down(button)
    }) {
        println!(
            "panic in pointer_button_down_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn pointer_destroy_ffi<W: Wlcs>(ptr: *mut WlcsPointer) {
    if let Err(err) = std::panic::catch_unwind(|| {
        // SAFETY:
        // - wlcs will no longer use the WlcsPointer pointer. This ensures we take back ownership of the
        //   allocation.
        // - The PointerHandle was created using Box::from_raw, ensuring the memory layout is correct.
        let mut pointer =
            unsafe { Box::from_raw(container_of!(ptr, PointerHandle<W>, wlcs_pointer)) };
        pointer.p.destroy()
    }) {
        println!(
            "panic in pointer_destroy_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

const fn wlcs_pointer<W: Wlcs>() -> WlcsPointer {
    WlcsPointer {
        version: 1,
        move_absolute: Some(pointer_move_absolute_ffi::<W>),
        move_relative: Some(pointer_move_relative_ffi::<W>),
        button_up: Some(pointer_button_up_ffi::<W>),
        button_down: Some(pointer_button_down_ffi::<W>),
        destroy: Some(pointer_destroy_ffi::<W>),
    }
}

unsafe extern "C" fn touch_down_ffi<W: Wlcs>(ptr: *mut WlcsTouch, x: wl_fixed_t, y: wl_fixed_t) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let touch = unsafe { get_touch_handle::<W>(ptr) };
        touch.t.touch_down(x, y);
    }) {
        println!(
            "panic in touch_down_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn touch_move_ffi<W: Wlcs>(ptr: *mut WlcsTouch, x: wl_fixed_t, y: wl_fixed_t) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let touch = unsafe { get_touch_handle::<W>(ptr) };
        touch.t.touch_move(x, y);
    }) {
        println!(
            "panic in touch_down_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn touch_up_ffi<W: Wlcs>(ptr: *mut WlcsTouch) {
    if let Err(err) = std::panic::catch_unwind(|| {
        let touch = unsafe { get_touch_handle::<W>(ptr) };
        touch.t.touch_up();
    }) {
        println!(
            "panic in touch_up_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

unsafe extern "C" fn touch_destroy_ffi<W: Wlcs>(ptr: *mut WlcsTouch) {
    if let Err(err) = std::panic::catch_unwind(|| {
        // SAFETY:
        // - wlcs will no longer use the WlcsTouch pointer. This ensures we take back ownership of the
        //   allocation.
        // - The TouchHandle was created using Box::from_raw, ensuring the memory layout is correct.
        let mut touch = unsafe { Box::from_raw(container_of!(ptr, TouchHandle<W>, wlcs_touch)) };
        touch.t.destroy()
    }) {
        println!(
            "panic in touch_destroy_ffi on ptr: {:p} (type {:?})",
            err.as_ref() as *const _,
            err.type_id()
        );
    }
}

const fn wlcs_touch<W: Wlcs>() -> WlcsTouch {
    WlcsTouch {
        version: 1,
        touch_down: Some(touch_down_ffi::<W>),
        touch_move: Some(touch_move_ffi::<W>),
        touch_up: Some(touch_up_ffi::<W>),
        destroy: Some(touch_destroy_ffi::<W>),
    }
}
