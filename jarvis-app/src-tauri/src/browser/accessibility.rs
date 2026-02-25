#[cfg(target_os = "macos")]
use core_foundation::base::{CFTypeRef, TCFType};
#[cfg(target_os = "macos")]
use core_foundation::string::{CFString, CFStringRef};
#[cfg(target_os = "macos")]
use core_foundation::array::CFArrayRef;
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::thread;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
pub type AXUIElementRef = CFTypeRef;

#[cfg(target_os = "macos")]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AXError {
    Success = 0,
    Failure = -25200,
    IllegalArgument = -25201,
    InvalidUIElement = -25202,
    InvalidUIElementObserver = -25203,
    CannotComplete = -25204,
    AttributeUnsupported = -25205,
    ActionUnsupported = -25206,
    NotificationUnsupported = -25207,
    NotImplemented = -25208,
    NotificationAlreadyRegistered = -25209,
    NotificationNotRegistered = -25210,
    APIDisabled = -25211,
    NoValue = -25212,
    ParameterizedAttributeUnsupported = -25213,
    NotEnoughPrecision = -25214,
}

#[cfg(target_os = "macos")]
impl AXError {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => AXError::Success,
            -25200 => AXError::Failure,
            -25201 => AXError::IllegalArgument,
            -25202 => AXError::InvalidUIElement,
            -25203 => AXError::InvalidUIElementObserver,
            -25204 => AXError::CannotComplete,
            -25205 => AXError::AttributeUnsupported,
            -25206 => AXError::ActionUnsupported,
            -25207 => AXError::NotificationUnsupported,
            -25208 => AXError::NotImplemented,
            -25209 => AXError::NotificationAlreadyRegistered,
            -25210 => AXError::NotificationNotRegistered,
            -25211 => AXError::APIDisabled,
            -25212 => AXError::NoValue,
            -25213 => AXError::ParameterizedAttributeUnsupported,
            -25214 => AXError::NotEnoughPrecision,
            _ => AXError::Failure,
        }
    }
}

// Attribute name constants
#[cfg(target_os = "macos")]
pub const K_AX_ROLE_ATTRIBUTE: &str = "AXRole";
#[cfg(target_os = "macos")]
pub const K_AX_TITLE_ATTRIBUTE: &str = "AXTitle";
#[cfg(target_os = "macos")]
pub const K_AX_VALUE_ATTRIBUTE: &str = "AXValue";
#[cfg(target_os = "macos")]
pub const K_AX_CHILDREN_ATTRIBUTE: &str = "AXChildren";
#[cfg(target_os = "macos")]
pub const K_AX_PLACEHOLDER_VALUE_ATTRIBUTE: &str = "AXPlaceholderValue";

// FFI declarations
#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
    pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    pub fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    pub fn AXUIElementCopyAttributeNames(
        element: AXUIElementRef,
        names: *mut CFArrayRef,
    ) -> i32;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFArrayGetCount(array: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> CFTypeRef;
}

/// Maximum recursion depth for accessibility tree traversal to prevent stack overflow
#[cfg(target_os = "macos")]
const MAX_TRAVERSAL_DEPTH: usize = 150;

#[cfg(target_os = "macos")]
pub struct WebArea {
    pub title: String,
    pub element: AXUIElementRef,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct TextBlock {
    pub text: String,
    pub role: String,
    pub depth: usize,
    pub parent_role: Option<String>,
}

#[cfg(target_os = "macos")]
pub struct AccessibilityReader;

#[cfg(target_os = "macos")]
impl AccessibilityReader {
    pub fn check_permission() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn find_chrome_pid() -> Result<i32, String> {
        // Use osascript to query NSWorkspace for Chrome's process ID
        let output = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events" to get unix id of processes whose bundle identifier is "com.google.Chrome""#)
            .output()
            .map_err(|e| format!("Failed to execute osascript: {}", e))?;

        if !output.status.success() {
            return Err("Chrome is not running".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pid_str = stdout.trim();
        
        if pid_str.is_empty() {
            return Err("Chrome is not running".to_string());
        }

        // Parse the first PID (osascript returns comma-separated list if multiple instances)
        let first_pid = pid_str.split(',').next().unwrap_or("").trim();
        
        first_pid
            .parse::<i32>()
            .map_err(|_| "Chrome is not running".to_string())
    }

    pub fn find_web_areas(pid: i32) -> Result<Vec<WebArea>, String> {
        unsafe {
            let app_element = AXUIElementCreateApplication(pid);
            
            let mut web_areas = Vec::new();
            Self::traverse_for_web_areas(app_element, &mut web_areas)?;
            
            // If no web areas found, retry once after 500ms delay
            if web_areas.is_empty() {
                thread::sleep(Duration::from_millis(500));
                Self::traverse_for_web_areas(app_element, &mut web_areas)?;
            }
            
            Ok(web_areas)
        }
    }

    unsafe fn traverse_for_web_areas(
        element: AXUIElementRef,
        web_areas: &mut Vec<WebArea>,
    ) -> Result<(), String> {
        Self::traverse_for_web_areas_recursive(element, web_areas, 0)
    }

    unsafe fn traverse_for_web_areas_recursive(
        element: AXUIElementRef,
        web_areas: &mut Vec<WebArea>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > MAX_TRAVERSAL_DEPTH {
            return Ok(());
        }

        // Get role
        let role = Self::get_attribute_string(element, K_AX_ROLE_ATTRIBUTE);

        if let Some(role_str) = role {
            if role_str == "AXWebArea" {
                // Extract title
                let title = Self::get_attribute_string(element, K_AX_TITLE_ATTRIBUTE)
                    .unwrap_or_else(|| "Untitled".to_string());

                // Retain element since it will be used later
                CFRetain(element);
                web_areas.push(WebArea {
                    title,
                    element,
                });
            }
        }

        // Traverse children
        let children = Self::get_children(element);
        if let Some(children_array) = children {
            for child in children_array {
                Self::traverse_for_web_areas_recursive(child, web_areas, depth + 1)?;
                // Release child after traversal (was retained in get_children)
                CFRelease(child);
            }
        }

        Ok(())
    }

    pub fn extract_text_content(web_area: AXUIElementRef) -> Result<Vec<TextBlock>, String> {
        let mut text_blocks = Vec::new();
        unsafe {
            Self::traverse_for_text(web_area, &mut text_blocks, 0, None)?;
        }
        Ok(text_blocks)
    }

    unsafe fn traverse_for_text(
        element: AXUIElementRef,
        text_blocks: &mut Vec<TextBlock>,
        depth: usize,
        parent_role: Option<String>,
    ) -> Result<(), String> {
        if depth > MAX_TRAVERSAL_DEPTH {
            return Ok(());
        }

        let role = Self::get_attribute_string(element, K_AX_ROLE_ATTRIBUTE);

        if let Some(role_str) = &role {
            match role_str.as_str() {
                "AXStaticText" => {
                    if let Some(value) = Self::get_attribute_string(element, K_AX_VALUE_ATTRIBUTE) {
                        if !value.trim().is_empty() {
                            text_blocks.push(TextBlock {
                                text: value,
                                role: role_str.clone(),
                                depth,
                                parent_role: parent_role.clone(),
                            });
                        }
                    }
                }
                "AXHeading" => {
                    if let Some(title) = Self::get_attribute_string(element, K_AX_TITLE_ATTRIBUTE) {
                        if !title.trim().is_empty() {
                            text_blocks.push(TextBlock {
                                text: format!("## {}", title),
                                role: role_str.clone(),
                                depth,
                                parent_role: parent_role.clone(),
                            });
                        }
                    }
                }
                "AXLink" => {
                    if let Some(title) = Self::get_attribute_string(element, K_AX_TITLE_ATTRIBUTE) {
                        if !title.trim().is_empty() {
                            text_blocks.push(TextBlock {
                                text: format!("[link: {}]", title),
                                role: role_str.clone(),
                                depth,
                                parent_role: parent_role.clone(),
                            });
                        }
                    }
                }
                "AXTextField" => {
                    if let Some(placeholder) = Self::get_attribute_string(element, K_AX_PLACEHOLDER_VALUE_ATTRIBUTE) {
                        if !placeholder.trim().is_empty() {
                            text_blocks.push(TextBlock {
                                text: format!("[input: {}]", placeholder),
                                role: role_str.clone(),
                                depth,
                                parent_role: parent_role.clone(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Traverse children
        let children = Self::get_children(element);
        if let Some(children_array) = children {
            for child in children_array {
                Self::traverse_for_text(child, text_blocks, depth + 1, role.clone())?;
                // Release child after traversal (was retained in get_children)
                CFRelease(child);
            }
        }

        Ok(())
    }

    unsafe fn get_attribute_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
        let attr_name = CFString::new(attribute);
        let mut value: CFTypeRef = std::ptr::null();
        
        let result = AXUIElementCopyAttributeValue(
            element,
            attr_name.as_concrete_TypeRef(),
            &mut value,
        );
        
        if result == 0 && !value.is_null() {
            let cf_string = CFString::wrap_under_create_rule(value as CFStringRef);
            Some(cf_string.to_string())
        } else {
            None
        }
    }

    unsafe fn get_children(element: AXUIElementRef) -> Option<Vec<AXUIElementRef>> {
        let attr_name = CFString::new(K_AX_CHILDREN_ATTRIBUTE);
        let mut value: CFTypeRef = std::ptr::null();

        let result = AXUIElementCopyAttributeValue(
            element,
            attr_name.as_concrete_TypeRef(),
            &mut value,
        );

        if result == 0 && !value.is_null() {
            let array_ref = value as CFArrayRef;
            let count = CFArrayGetCount(array_ref);
            let mut children = Vec::with_capacity(count as usize);

            for i in 0..count {
                let child = CFArrayGetValueAtIndex(array_ref, i);
                if !child.is_null() {
                    // Retain each child so it survives after the array is released
                    CFRetain(child);
                    children.push(child);
                }
            }

            // Release the array — children are independently retained
            CFRelease(value);

            Some(children)
        } else {
            None
        }
    }
}
