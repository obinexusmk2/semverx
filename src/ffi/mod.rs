//! Polyglot FFI abstraction layer for `semverx`.
//!
//! This module is the inter-dependent component abstraction layer inspired by
//! `libpolycall`. It exposes SemverX through a C-ABI surface consumable from
//! many languages (Python via `ctypes`, Node.js via N-API / `ffi-napi`, PHP
//! via `FFI::cdef`, Lua via `ffi.cdef`, etc.), so a SemverX version managed in
//! Rust can drive dependency resolution across a heterogeneous toolchain.
//!
//! # Binding modes
//!
//! Matching the OBINexus registry schema
//! (`@obinexus/<pkg>.monoglot`, `.polyglot`, `.hybrid`):
//!
//! * [`BindingMode::Monoglot`] — a single-language binding. The consumer is
//!   locked to one language; no foreign-call bridging is advertised. Think of
//!   a pure-Python app using only the pure-Python SemverX package.
//! * [`BindingMode::Polyglot`] — the standardised, cross-language FFI. The
//!   same `libsemverx.{so,dylib,dll}` / `libsemverx.a` artifacts are consumed
//!   from any language that can call C.
//! * [`BindingMode::Hybrid`] — a monoglot host with polyglot extensions. The
//!   host program runs in its own language but is free to delegate parts of
//!   the resolution graph across FFI when it needs to.
//!
//! # Stability
//!
//! The functions exported here are `#[no_mangle] extern "C"`; their ABI is
//! part of the crate's public surface. Memory handed out by this layer MUST be
//! released by the matching `_free` function, never by the caller's allocator.
//!
//! The richer binding in [`c_api`] targets a superset SemverX type system
//! (classifiers, stress zones, SEI metadata) that is still under construction
//! and therefore left disabled behind a feature flag.

#![allow(unsafe_code)]
#![allow(missing_docs)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::core::Version;

// The extended C API is kept in-tree for reference but not wired into the
// module graph yet — it depends on types (`SemverX`, `BubblingError`, ...)
// that have not landed in `core::` at this version. Re-enable by removing
// the `cfg(any())` guard once those types are merged.
#[cfg(any())]
pub mod c_api;

/// Binding mode advertised to the caller.
///
/// Encoded as `#[repr(C)]` with stable discriminants so that the value can
/// be round-tripped across FFI without language-specific enum encodings.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingMode {
    /// Single-language host (`@obinexus/<pkg>.monoglot`).
    Monoglot = 0,
    /// Many-language, C-ABI host (`@obinexus/<pkg>.polyglot`).
    Polyglot = 1,
    /// Monoglot host with polyglot extension points (`@obinexus/<pkg>.hybrid`).
    Hybrid = 2,
}

/// C-compatible view of a parsed [`Version`].
///
/// `pre` and `build` are either `NULL` or heap-allocated C strings that must
/// be released by [`semverx_free`] (which owns the whole `CVersion`).
#[repr(C)]
pub struct CVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: *const c_char,
    pub build: *const c_char,
}

/// Parse a nul-terminated UTF-8 version string.
///
/// Returns a heap-allocated `*mut CVersion` on success, or `NULL` on invalid
/// input. Release with [`semverx_free`].
#[no_mangle]
pub extern "C" fn semverx_parse(version_str: *const c_char) -> *mut CVersion {
    if version_str.is_null() {
        return ptr::null_mut();
    }
    let raw = unsafe { CStr::from_ptr(version_str) };
    let s = match raw.to_str() {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };
    match Version::parse(s) {
        Ok(v) => Box::into_raw(Box::new(CVersion {
            major: v.major,
            minor: v.minor,
            patch: v.patch,
            pre: opt_cstr(v.pre.as_deref()),
            build: opt_cstr(v.build.as_deref()),
        })),
        Err(_) => ptr::null_mut(),
    }
}

/// Release a [`CVersion`] previously returned by [`semverx_parse`].
///
/// Safe to call on `NULL`. Double-free is undefined behaviour, as with any
/// C allocator.
#[no_mangle]
pub extern "C" fn semverx_free(version: *mut CVersion) {
    if version.is_null() {
        return;
    }
    unsafe {
        let v = Box::from_raw(version);
        if !v.pre.is_null() {
            let _ = CString::from_raw(v.pre as *mut c_char);
        }
        if !v.build.is_null() {
            let _ = CString::from_raw(v.build as *mut c_char);
        }
    }
}

/// Compare two `CVersion` pointers by major/minor/patch.
///
/// Returns `-1` if `a < b`, `0` if equal, `1` if `a > b`, and `-2` if either
/// pointer is `NULL`.
#[no_mangle]
pub extern "C" fn semverx_compare(a: *const CVersion, b: *const CVersion) -> c_int {
    if a.is_null() || b.is_null() {
        return -2;
    }
    let (va, vb) = unsafe { (&*a, &*b) };
    match (va.major, va.minor, va.patch).cmp(&(vb.major, vb.minor, vb.patch)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Advertise the default binding mode for this build.
///
/// Returned as a `c_int` because some hosts (notably older ctypes users)
/// handle `#[repr(C)]` enums unevenly. Cast to [`BindingMode`] by value.
#[no_mangle]
pub extern "C" fn semverx_default_mode() -> c_int {
    BindingMode::Hybrid as c_int
}

/// Convert a [`BindingMode`] discriminant to its string name.
///
/// The returned pointer is owned by the caller and MUST be released with
/// [`semverx_string_free`].
#[no_mangle]
pub extern "C" fn semverx_binding_mode_name(mode: c_int) -> *mut c_char {
    let name = match mode {
        0 => "monoglot",
        1 => "polyglot",
        2 => "hybrid",
        _ => "unknown",
    };
    CString::new(name)
        .map(|c| c.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Free a C string previously returned by this FFI layer.
#[no_mangle]
pub extern "C" fn semverx_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn opt_cstr(s: Option<&str>) -> *const c_char {
    match s.and_then(|v| CString::new(v).ok()) {
        Some(c) => c.into_raw() as *const c_char,
        None => ptr::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_free_roundtrip() {
        let input = CString::new("1.2.3").unwrap();
        let v = semverx_parse(input.as_ptr());
        assert!(!v.is_null());
        unsafe {
            assert_eq!((*v).major, 1);
            assert_eq!((*v).minor, 2);
            assert_eq!((*v).patch, 3);
        }
        semverx_free(v);
    }

    #[test]
    fn compare_orders_versions() {
        let a = semverx_parse(CString::new("1.0.0").unwrap().as_ptr());
        let b = semverx_parse(CString::new("1.0.1").unwrap().as_ptr());
        assert_eq!(semverx_compare(a, b), -1);
        assert_eq!(semverx_compare(a, a), 0);
        assert_eq!(semverx_compare(b, a), 1);
        semverx_free(a);
        semverx_free(b);
    }

    #[test]
    fn default_mode_is_hybrid() {
        assert_eq!(semverx_default_mode(), BindingMode::Hybrid as c_int);
    }
}
