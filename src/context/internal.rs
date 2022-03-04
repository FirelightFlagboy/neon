use super::ModuleContext;
use crate::handle::Handle;
#[cfg(feature = "legacy-runtime")]
use crate::object::class::ClassMap;
use crate::result::NeonResult;
use crate::types::{JsObject, JsValue};
use neon_runtime;
use neon_runtime::raw;
use neon_runtime::scope::Root;
#[cfg(feature = "legacy-runtime")]
use neon_runtime::try_catch::TryCatchControl;
#[cfg(feature = "legacy-runtime")]
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::mem::MaybeUninit;
#[cfg(feature = "legacy-runtime")]
use std::os::raw::c_void;
#[cfg(feature = "legacy-runtime")]
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

#[cfg(feature = "legacy-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Env(raw::Isolate);

#[cfg(feature = "napi-1")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Env(raw::Env);

#[cfg(feature = "napi-1")]
impl From<raw::Env> for Env {
    fn from(env: raw::Env) -> Self {
        Self(env)
    }
}

thread_local! {
    #[allow(unused)]
    pub(crate) static IS_RUNNING: RefCell<bool> = RefCell::new(false);
}

#[cfg(feature = "legacy-runtime")]
extern "C" fn drop_class_map(map: Box<ClassMap>) {
    std::mem::drop(map);
}

impl Env {
    #[cfg(feature = "legacy-runtime")]
    pub(crate) fn to_raw(self) -> raw::Isolate {
        let Self(ptr) = self;
        ptr
    }

    #[cfg(feature = "napi-1")]
    pub(crate) fn to_raw(self) -> raw::Env {
        let Self(ptr) = self;
        ptr
    }

    #[cfg(feature = "legacy-runtime")]
    pub(crate) fn class_map(&mut self) -> &mut ClassMap {
        let mut ptr: *mut c_void = unsafe { neon_runtime::class::get_class_map(self.to_raw()) };
        if ptr.is_null() {
            let b: Box<ClassMap> = Box::new(ClassMap::new());
            let raw = Box::into_raw(b);
            ptr = raw.cast();
            let free_map: *mut c_void = unsafe { std::mem::transmute(drop_class_map as usize) };
            unsafe {
                neon_runtime::class::set_class_map(self.to_raw(), ptr, free_map);
            }
        }
        unsafe { &mut *ptr.cast() }
    }

    #[cfg(feature = "legacy-runtime")]
    pub(crate) fn current() -> Env {
        unsafe { std::mem::transmute(neon_runtime::call::current_isolate()) }
    }

    #[cfg(feature = "napi-1")]
    unsafe fn try_catch<T, F>(self, f: F) -> Result<T, raw::Local>
    where
        F: FnOnce() -> Result<T, crate::result::Throw>,
    {
        let result = f();
        let mut local: MaybeUninit<raw::Local> = MaybeUninit::zeroed();

        if neon_runtime::error::catch_error(self.to_raw(), local.as_mut_ptr()) {
            Err(local.assume_init())
        } else if let Ok(result) = result {
            Ok(result)
        } else {
            panic!("try_catch: unexpected Err(Throw) when VM is not in a throwing state");
        }
    }
}

pub struct ScopeMetadata {
    env: Env,
    active: Cell<bool>,
}

pub struct Scope<'a, R: Root + 'static> {
    pub metadata: ScopeMetadata,
    pub handle_scope: &'a mut R,
}

impl<'a, R: Root + 'static> Scope<'a, R> {
    pub fn with<T, F: for<'b> FnOnce(Scope<'b, R>) -> T>(env: Env, f: F) -> T {
        let mut handle_scope: R = unsafe { R::allocate() };
        unsafe {
            handle_scope.enter(env.to_raw());
        }
        let result = {
            let scope = Scope {
                metadata: ScopeMetadata {
                    env,
                    active: Cell::new(true),
                },
                handle_scope: &mut handle_scope,
            };
            f(scope)
        };
        unsafe {
            handle_scope.exit(env.to_raw());
        }
        result
    }
}

pub trait ContextInternal<'a>: Sized {
    fn scope_metadata(&self) -> &ScopeMetadata;

    fn env(&self) -> Env {
        self.scope_metadata().env
    }

    fn is_active(&self) -> bool {
        self.scope_metadata().active.get()
    }

    fn check_active(&self) {
        if !self.is_active() {
            panic!("execution context is inactive");
        }
    }

    fn activate(&self) {
        self.scope_metadata().active.set(true);
    }
    fn deactivate(&self) {
        self.scope_metadata().active.set(false);
    }

    #[cfg(feature = "legacy-runtime")]
    fn try_catch_internal<T, F>(&mut self, f: F) -> Result<T, Handle<'a, JsValue>>
    where
        F: FnOnce(&mut Self) -> NeonResult<T>,
    {
        // A closure does not have a guaranteed layout, so we need to box it in order to pass
        // a pointer to it across the boundary into C++.
        let rust_thunk = Box::into_raw(Box::new(f));

        let mut ok: MaybeUninit<T> = MaybeUninit::zeroed();
        let mut err: MaybeUninit<raw::Local> = MaybeUninit::zeroed();
        let mut unwind_value: MaybeUninit<*mut c_void> = MaybeUninit::zeroed();

        let ctrl = unsafe {
            neon_runtime::try_catch::with(
                try_catch_glue::<Self, T, F>,
                rust_thunk as *mut c_void,
                (self as *mut Self) as *mut c_void,
                ok.as_mut_ptr() as *mut c_void,
                err.as_mut_ptr(),
                unwind_value.as_mut_ptr(),
            )
        };

        match ctrl {
            TryCatchControl::Panicked => {
                let unwind_value: Box<dyn Any + Send> = *unsafe {
                    Box::from_raw(unwind_value.assume_init() as *mut Box<dyn Any + Send>)
                };
                resume_unwind(unwind_value);
            }
            TryCatchControl::Returned => Ok(unsafe { ok.assume_init() }),
            TryCatchControl::Threw => {
                let err = unsafe { err.assume_init() };
                Err(JsValue::new_internal(err))
            }
            TryCatchControl::UnexpectedErr => {
                panic!("try_catch: unexpected Err(Throw) when VM is not in a throwing state");
            }
        }
    }

    #[cfg(feature = "napi-1")]
    fn try_catch_internal<T, F>(&mut self, f: F) -> Result<T, Handle<'a, JsValue>>
    where
        F: FnOnce(&mut Self) -> NeonResult<T>,
    {
        unsafe {
            self.env()
                .try_catch(move || f(self))
                .map_err(JsValue::new_internal)
        }
    }
}

#[cfg(feature = "legacy-runtime")]
extern "C" fn try_catch_glue<'a, 'b: 'a, C, T, F>(
    rust_thunk: *mut c_void,
    cx: *mut c_void,
    returned: *mut c_void,
    unwind_value: *mut *mut c_void,
) -> TryCatchControl
where
    C: ContextInternal<'a>,
    F: FnOnce(&mut C) -> NeonResult<T>,
{
    let f: F = *unsafe { Box::from_raw(rust_thunk as *mut F) };
    let cx: &mut C = unsafe { &mut *cx.cast() };

    // The mutable reference to the context is a fiction of the Neon library,
    // since it doesn't actually contain any data in the Rust memory space,
    // just a link to the JS VM. So we don't need to do any kind of poisoning
    // of the context when a panic occurs. So we suppress the Rust compiler
    // errors from using the mutable reference across an unwind boundary.
    match catch_unwind(AssertUnwindSafe(|| f(cx))) {
        // No Rust panic, no JS exception.
        Ok(Ok(result)) => unsafe {
            (returned as *mut T).write(result);
            TryCatchControl::Returned
        },
        // No Rust panic, caught a JS exception.
        Ok(Err(_)) => TryCatchControl::Threw,
        // Rust panicked.
        Err(err) => unsafe {
            // A panic value has an undefined layout, so wrap it in an extra box.
            let boxed = Box::new(err);
            *unwind_value = Box::into_raw(boxed) as *mut c_void;
            TryCatchControl::Panicked
        },
    }
}

#[cfg(feature = "legacy-runtime")]
pub fn initialize_module(exports: Handle<JsObject>, init: fn(ModuleContext) -> NeonResult<()>) {
    let env = Env::current();

    ModuleContext::with(env, exports, |cx| {
        let _ = init(cx);
    });
}

#[cfg(feature = "napi-1")]
pub fn initialize_module(
    env: raw::Env,
    exports: Handle<JsObject>,
    init: fn(ModuleContext) -> NeonResult<()>,
) {
    unsafe {
        neon_runtime::setup(env);
    }

    IS_RUNNING.with(|v| {
        *v.borrow_mut() = true;
    });

    ModuleContext::with(Env(env), exports, |cx| {
        let _ = init(cx);
    });
}
