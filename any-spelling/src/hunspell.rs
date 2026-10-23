//! A safe handle over one Hunspell dictionary.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Mutex;

use hunspell_sys as ffi;

struct Handle(NonNull<ffi::Hunhandle>);

// A Hunspell instance is not thread-safe, so the handle is reachable only
// through the dictionary's mutex; moving it between threads is fine.
unsafe impl Send for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { ffi::Hunspell_destroy(self.0.as_ptr()) }
    }
}

pub struct Dictionary {
    handle: Mutex<Handle>,
}

fn read_utf8(path: &Path, label: &str) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|_| format!("The {label} dictionary could not load."))?;
    String::from_utf8(bytes).map_err(|_| format!("The {label} dictionary is invalid. Try again."))
}

/// Hunspell reads any file as a dictionary, an empty or truncated one included,
/// and then accepts nothing or everything. Check the encoding and the header
/// first; header counts are hints, not exact line counts, in several packs.
fn validate(aff: &Path, dic: &Path, label: &str) -> Result<(), String> {
    let invalid = || format!("The {label} dictionary is invalid. Try again.");
    let aff_text = read_utf8(aff, label)?;
    if !aff_text.lines().any(|line| {
        let mut parts = line
            .trim_end_matches('\r')
            .split([' ', '\t'])
            .filter(|part| !part.is_empty());
        parts.next() == Some("SET") && parts.next() == Some("UTF-8") && parts.next().is_none()
    }) {
        return Err(invalid());
    }
    let dic_text = read_utf8(dic, label)?;
    let (header, rest) = dic_text.split_once('\n').ok_or_else(invalid)?;
    let header = header.trim();
    let count_valid = !header.is_empty()
        && header.bytes().all(|byte| byte.is_ascii_digit())
        && header.parse::<u64>().is_ok_and(|count| count > 0);
    if !count_valid || rest.trim().is_empty() {
        return Err(invalid());
    }
    Ok(())
}

fn path_arg(path: &Path) -> Result<CString, String> {
    #[cfg(windows)]
    {
        // Hunspell opens a path through the wide API only when it carries the
        // long-path prefix; otherwise a non-ASCII profile folder fails to load.
        let text = path
            .to_str()
            .ok_or("The dictionary path is not valid UTF-8.")?;
        let prefixed = if text.starts_with(r"\\?\") {
            text.to_owned()
        } else {
            format!(r"\\?\{text}")
        };
        CString::new(prefixed).map_err(|_| "The dictionary path is invalid.".to_owned())
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::ffi::OsStrExt;
        CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "The dictionary path is invalid.".to_owned())
    }
}

impl Dictionary {
    pub fn open(aff: &Path, dic: &Path, label: &str) -> Result<Self, String> {
        validate(aff, dic, label)?;
        let (aff_arg, dic_arg) = (path_arg(aff)?, path_arg(dic)?);
        let raw = unsafe { ffi::Hunspell_create(aff_arg.as_ptr(), dic_arg.as_ptr()) };
        let handle = Handle(
            NonNull::new(raw).ok_or_else(|| format!("The {label} dictionary could not load."))?,
        );
        let encoding = unsafe { ffi::Hunspell_get_dic_encoding(handle.0.as_ptr()) };
        if encoding.is_null() || unsafe { CStr::from_ptr(encoding) }.to_bytes() != b"UTF-8" {
            return Err(format!("The {label} dictionary is invalid. Try again."));
        }
        Ok(Self {
            handle: Mutex::new(handle),
        })
    }

    fn handle(&self) -> std::sync::MutexGuard<'_, Handle> {
        // Hunspell keeps no state a panicking caller could leave half-written.
        self.handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn spell(&self, word: &str) -> bool {
        let Ok(word) = CString::new(word) else {
            return false;
        };
        let handle = self.handle();
        unsafe { ffi::Hunspell_spell(handle.0.as_ptr(), word.as_ptr()) != 0 }
    }

    pub fn suggest(&self, word: &str) -> Vec<String> {
        let Ok(word) = CString::new(word) else {
            return Vec::new();
        };
        let handle = self.handle();
        let mut list: *mut *mut c_char = std::ptr::null_mut();
        let count = unsafe { ffi::Hunspell_suggest(handle.0.as_ptr(), &mut list, word.as_ptr()) };
        if count <= 0 || list.is_null() {
            return Vec::new();
        }
        let length = usize::try_from(count).unwrap_or(0);
        let suggestions = (0..length)
            .filter_map(|index| {
                let item = unsafe { *list.add(index) };
                (!item.is_null())
                    .then(|| {
                        unsafe { CStr::from_ptr(item) }
                            .to_str()
                            .ok()
                            .map(str::to_owned)
                    })
                    .flatten()
            })
            .collect();
        unsafe { ffi::Hunspell_free_list(handle.0.as_ptr(), &mut list, count) };
        suggestions
    }
}

#[cfg(test)]
pub fn library_version() -> String {
    let version = unsafe { ffi::Hunspell_get_library_version() };
    if version.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(version) }
        .to_string_lossy()
        .into_owned()
}
