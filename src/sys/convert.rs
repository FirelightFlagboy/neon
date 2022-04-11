use super::{
    bindings as napi,
    raw::{Env, Local},
};

/// This API is currently unused, see <https://github.com/neon-bindings/neon/issues/572>
pub unsafe fn to_object(out: &mut Local, env: Env, value: Local) -> bool {
    let status = napi::coerce_to_object(env, value, out as *mut _);

    status == napi::Status::Ok
}

pub unsafe fn to_string(out: &mut Local, env: Env, value: Local) -> bool {
    let status = napi::coerce_to_string(env, value, out as *mut _);

    status == napi::Status::Ok
}
