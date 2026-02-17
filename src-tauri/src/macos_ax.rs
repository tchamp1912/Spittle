#![cfg(target_os = "macos")]

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::base::{Boolean, CFIndex, CFRange, CFRelease, CFTypeRef};
use core_foundation_sys::string::CFStringRef;
use std::ffi::c_void;
use std::ptr;

type AXError = i32;
const K_AX_ERROR_SUCCESS: AXError = 0;
const K_AX_VALUE_TYPE_CF_RANGE: u32 = 4;

#[repr(C)]
struct __AXUIElement(c_void);
type AXUIElementRef = *const __AXUIElement;

#[repr(C)]
struct __AXValue(c_void);
type AXValueRef = *const __AXValue;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementIsAttributeSettable(
        element: AXUIElementRef,
        attribute: CFStringRef,
        settable: *mut Boolean,
    ) -> AXError;
    fn AXValueCreate(the_type: u32, value_ptr: *const c_void) -> AXValueRef;
    fn AXValueGetValue(value: AXValueRef, the_type: u32, value_ptr: *mut c_void) -> Boolean;
}

struct CfRef(CFTypeRef);

impl Drop for CfRef {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

impl CfRef {
    fn as_type_ref(&self) -> CFTypeRef {
        self.0
    }
}

fn ax_error_context(name: &str, code: AXError) -> String {
    format!("{name} failed with AXError={code}")
}

unsafe fn copy_attribute(element: AXUIElementRef, attribute: &str) -> Result<CfRef, String> {
    let attr = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let err =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err != K_AX_ERROR_SUCCESS {
        return Err(ax_error_context("AXUIElementCopyAttributeValue", err));
    }
    if value.is_null() {
        return Err(format!(
            "AXUIElementCopyAttributeValue returned null for attribute {attribute}"
        ));
    }
    Ok(CfRef(value))
}

unsafe fn focused_text_element() -> Result<CfRef, String> {
    let system = unsafe { AXUIElementCreateSystemWide() };
    if system.is_null() {
        return Err("AXUIElementCreateSystemWide returned null".to_string());
    }
    let _system_ref = CfRef(system as CFTypeRef);
    let focused_app = unsafe { copy_attribute(system, "AXFocusedApplication") }?;
    let focused_app_ref = focused_app.as_type_ref() as AXUIElementRef;
    if focused_app_ref.is_null() {
        return Err("AXFocusedApplication is null".to_string());
    }
    let focused_element = unsafe { copy_attribute(focused_app_ref, "AXFocusedUIElement") }?;
    if (focused_element.as_type_ref() as AXUIElementRef).is_null() {
        return Err("AXFocusedUIElement is null".to_string());
    }
    Ok(focused_element)
}

pub fn try_select_replace_range_before_cursor(
    delete_chars: usize,
    suffix_chars: usize,
) -> Result<(), String> {
    unsafe {
        let focused_element_ref = focused_text_element()?;
        let focused_element = focused_element_ref.as_type_ref() as AXUIElementRef;

        let mut settable: Boolean = 0;
        let range_attr = CFString::new("AXSelectedTextRange");
        let settable_err = AXUIElementIsAttributeSettable(
            focused_element,
            range_attr.as_concrete_TypeRef(),
            &mut settable,
        );
        if settable_err != K_AX_ERROR_SUCCESS {
            return Err(ax_error_context(
                "AXUIElementIsAttributeSettable",
                settable_err,
            ));
        }
        if settable == 0 {
            return Err("AXSelectedTextRange is not settable for focused element".to_string());
        }

        let selected_range_ref = copy_attribute(focused_element, "AXSelectedTextRange")?;
        let selected_range_ax = selected_range_ref.as_type_ref() as AXValueRef;
        if selected_range_ax.is_null() {
            return Err("AXSelectedTextRange is null".to_string());
        }

        let mut current: CFRange = CFRange {
            location: 0 as CFIndex,
            length: 0 as CFIndex,
        };
        let ok = AXValueGetValue(
            selected_range_ax,
            K_AX_VALUE_TYPE_CF_RANGE,
            &mut current as *mut CFRange as *mut c_void,
        );
        if ok == 0 {
            return Err("AXValueGetValue(AXSelectedTextRange) failed".to_string());
        }

        let delta = (delete_chars + suffix_chars) as CFIndex;
        if current.location < delta {
            return Err(format!(
                "Cursor location {} is before required replace boundary {}",
                current.location, delta
            ));
        }

        let new_range = CFRange {
            location: current.location - delta,
            length: delete_chars as CFIndex,
        };
        let new_range_ax = AXValueCreate(
            K_AX_VALUE_TYPE_CF_RANGE,
            &new_range as *const CFRange as *const c_void,
        );
        if new_range_ax.is_null() {
            return Err("AXValueCreate(CFRange) failed".to_string());
        }
        let new_range_guard = CfRef(new_range_ax as CFTypeRef);

        let set_err = AXUIElementSetAttributeValue(
            focused_element,
            range_attr.as_concrete_TypeRef(),
            new_range_guard.as_type_ref(),
        );
        if set_err != K_AX_ERROR_SUCCESS {
            return Err(ax_error_context("AXUIElementSetAttributeValue", set_err));
        }
    }

    Ok(())
}
