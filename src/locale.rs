/*
Copyright (C) 2020 Dorian Rudolph

sirula is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

sirula is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with sirula.  If not, see <https://www.gnu.org/licenses/>.
*/

use libc::{setlocale, strcoll};
pub use locale_types::LocaleIdentifier;
use std::{
    cmp::{Ord, Ordering},
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
};

pub fn string_collate(a: &str, b: &str) -> Ordering {
    // Note: Only works properly if locale is set to UTF-8
    let ord = unsafe {
        let c_a = CString::new(a).unwrap();
        let c_b = CString::new(b).unwrap();
        strcoll(c_a.as_ptr(), c_b.as_ptr())
    };
    ord.cmp(&0)
}

unsafe fn setlocale_wrapper(category: i32, locale: *const c_char) -> Option<String> {
    let ret = setlocale(category, locale);
    if ret.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ret).to_str().unwrap().to_owned())
    }
}

pub fn set_locale(category: i32, locale: &str) -> Option<String> {
    unsafe {
        let c_locale = CString::new(locale).unwrap();
        setlocale_wrapper(category, c_locale.as_ptr())
    }
}

#[allow(unused)]
pub fn get_locale(category: i32) -> Option<String> {
    unsafe { setlocale_wrapper(category, ptr::null()) }
}
