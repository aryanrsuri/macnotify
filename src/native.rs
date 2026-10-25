//! Native notifications only; the main thread sleeps inside the macOS run loop.
use std::ffi::{CStr, c_char, c_void};
use std::ptr::null_mut;

type Object = *mut c_void;
type Selector = *mut c_void;

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Object;
    fn sel_registerName(name: *const c_char) -> Selector;
    fn objc_msgSend();
    fn objc_allocateClassPair(parent: Object, name: *const c_char, extra: usize) -> Object;
    fn objc_registerClassPair(class: Object);
    fn class_addMethod(
        class: Object,
        selector: Selector,
        implementation: *const c_void,
        types: *const c_char,
    ) -> bool;
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSWorkspaceDidWakeNotification: Object;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFNotificationCenterGetDistributedCenter() -> Object;
    fn CFNotificationCenterAddObserver(
        center: Object,
        observer: Object,
        callback: extern "C" fn(Object, Object, Object, Object, Object),
        name: Object,
        object: Object,
        suspension_behavior: isize,
    );
    fn CFStringCreateWithCString(allocator: Object, text: *const c_char, encoding: u32) -> Object;
    fn CFRelease(object: Object);
    static kCFRunLoopDefaultMode: Object;
    fn CFRunLoopRunInMode(mode: Object, seconds: f64, return_after_source: u8) -> i32;
}

extern "C" fn appearance_changed(_: Object, _: Object, _: Object, _: Object, _: Object) {
    crate::notify();
}

extern "C" fn woke(_: Object, _: Selector, _: Object) {
    crate::notify();
}

// This helper is only used for zero-argument Objective-C methods returning objects.
unsafe fn send_object(receiver: Object, name: &CStr) -> Object {
    unsafe {
        let send: unsafe extern "C" fn(Object, Selector) -> Object =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        send(receiver, sel_registerName(name.as_ptr()))
    }
}

pub fn run() -> ! {
    unsafe {
        let pool = send_object(objc_getClass(c"NSAutoreleasePool".as_ptr()), c"new");
        let name = CFStringCreateWithCString(
            null_mut(),
            c"AppleInterfaceThemeChangedNotification".as_ptr(),
            0x08000100,
        );
        assert!(!name.is_null());
        CFNotificationCenterAddObserver(
            CFNotificationCenterGetDistributedCenter(),
            null_mut(),
            appearance_changed,
            name,
            null_mut(),
            4, // DeliverImmediately, including while inactive.
        );
        CFRelease(name);

        let class = objc_allocateClassPair(
            objc_getClass(c"NSObject".as_ptr()),
            c"TermThemeWakeObserver".as_ptr(),
            0,
        );
        assert!(!class.is_null(), "cannot allocate wake observer");
        let selector = sel_registerName(c"woke:".as_ptr());
        assert!(class_addMethod(
            class,
            selector,
            woke as *const c_void,
            c"v@:@".as_ptr()
        ));
        objc_registerClassPair(class);
        // Retain the observer for the process lifetime.
        let observer = send_object(class, c"new");
        let workspace = send_object(objc_getClass(c"NSWorkspace".as_ptr()), c"sharedWorkspace");
        let center = send_object(workspace, c"notificationCenter");
        assert!(
            !observer.is_null() && !center.is_null(),
            "cannot register wake observer"
        );
        let add: unsafe extern "C" fn(Object, Selector, Object, Selector, Object, Object) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        add(
            center,
            sel_registerName(c"addObserver:selector:name:object:".as_ptr()),
            observer,
            selector,
            NSWorkspaceDidWakeNotification,
            null_mut(),
        );
        let drain: unsafe extern "C" fn(Object, Selector) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        drain(pool, sel_registerName(c"drain".as_ptr()));

        crate::notify();
        loop {
            let pool = send_object(objc_getClass(c"NSAutoreleasePool".as_ptr()), c"new");
            // No polling interval: sleep until a source fires, then release Cocoa temporaries.
            let result = CFRunLoopRunInMode(kCFRunLoopDefaultMode, f64::MAX, 1);
            drain(pool, sel_registerName(c"drain".as_ptr()));
            assert!(result == 4, "macOS notification run loop stopped: {result}");
        }
    }
}
