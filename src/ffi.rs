//! The subset of Hunspell's C API (`vendor/hunspell/hunspell.h`) the engine
//! calls. Every string is a NUL-terminated byte string in the dictionary's
//! encoding; the bundled dictionaries are all `SET UTF-8`.

use std::os::raw::{c_char, c_int};

#[repr(C)]
pub(crate) struct Hunhandle {
    _private: [u8; 0],
}

extern "C" {
    pub(crate) fn Hunspell_create(affpath: *const c_char, dpath: *const c_char) -> *mut Hunhandle;
    pub(crate) fn Hunspell_destroy(handle: *mut Hunhandle);
    pub(crate) fn Hunspell_spell(handle: *mut Hunhandle, word: *const c_char) -> c_int;
    pub(crate) fn Hunspell_get_dic_encoding(handle: *mut Hunhandle) -> *mut c_char;
    pub(crate) fn Hunspell_suggest(
        handle: *mut Hunhandle,
        list: *mut *mut *mut c_char,
        word: *const c_char,
    ) -> c_int;
    pub(crate) fn Hunspell_free_list(handle: *mut Hunhandle, list: *mut *mut *mut c_char, n: c_int);
    #[cfg(test)]
    pub(crate) fn Hunspell_get_library_version() -> *const c_char;
}
