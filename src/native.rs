//! macOS owns event delivery; this module never polls preferences.
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

extern "C" fn appearance_changed(
    _: Object,
    _: Selector,
    _: Object,
    _: Object,
    _: Object,
    _: Object,
) {
    eprintln!("pubrat: appearance changed");
    crate::notify();
}

extern "C" fn woke(_: Object, _: Selector, _: Object) {
    eprintln!("pubrat: woke from sleep");
    crate::notify();
}

// Only zero-argument Objective-C methods returning objects belong in this helper.
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
        let app = send_object(
            objc_getClass(c"NSApplication".as_ptr()),
            c"sharedApplication",
        );
        assert!(!app.is_null(), "cannot initialize NSApplication");
        let set_policy: unsafe extern "C" fn(Object, Selector, isize) -> bool =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        set_policy(
            app,
            sel_registerName(c"setActivationPolicy:".as_ptr()),
            2, // Prohibited: no Dock icon, menus, windows, or activation.
        );
        // A background process may already be prohibited; verify the state,
        // rather than requiring the setter to report a transition.
        let get_policy: unsafe extern "C" fn(Object, Selector) -> isize =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        assert_eq!(
            get_policy(app, sel_registerName(c"activationPolicy".as_ptr())),
            2,
            "cannot keep listener in the background",
        );

        let class = objc_allocateClassPair(
            objc_getClass(c"NSObject".as_ptr()),
            c"PubratObserver".as_ptr(),
            0,
        );
        assert!(!class.is_null(), "cannot allocate event observer");
        assert!(class_addMethod(
            class,
            sel_registerName(c"observeValueForKeyPath:ofObject:change:context:".as_ptr()),
            appearance_changed as *const c_void,
            c"v@:@@@^v".as_ptr(),
        ));
        let wake_selector = sel_registerName(c"woke:".as_ptr());
        assert!(class_addMethod(
            class,
            wake_selector,
            woke as *const c_void,
            c"v@:@".as_ptr(),
        ));
        objc_registerClassPair(class);
        // Keep the observer and key path alive for the process lifetime.
        let observer = send_object(class, c"new");
        let key = send_object(objc_getClass(c"NSString".as_ptr()), c"alloc");
        let init_string: unsafe extern "C" fn(Object, Selector, *const c_char) -> Object =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let key = init_string(
            key,
            sel_registerName(c"initWithUTF8String:".as_ptr()),
            c"effectiveAppearance".as_ptr(),
        );
        assert!(!observer.is_null() && !key.is_null());
        let observe: unsafe extern "C" fn(Object, Selector, Object, Object, usize, Object) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        observe(
            app,
            sel_registerName(c"addObserver:forKeyPath:options:context:".as_ptr()),
            observer,
            key,
            0,
            null_mut(),
        );

        let workspace = send_object(objc_getClass(c"NSWorkspace".as_ptr()), c"sharedWorkspace");
        let center = send_object(workspace, c"notificationCenter");
        assert!(!center.is_null(), "cannot register wake observer");
        let add: unsafe extern "C" fn(Object, Selector, Object, Selector, Object, Object) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        add(
            center,
            sel_registerName(c"addObserver:selector:name:object:".as_ptr()),
            observer,
            wake_selector,
            NSWorkspaceDidWakeNotification,
            null_mut(),
        );
        let send_void: unsafe extern "C" fn(Object, Selector) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        send_void(pool, sel_registerName(c"drain".as_ptr()));

        eprintln!("pubrat: listener started");
        crate::notify();
        // AppKit processes system appearance changes and drains per-event autorelease pools.
        send_void(app, sel_registerName(c"run".as_ptr()));
    }
    panic!("macOS application event loop stopped");
}
