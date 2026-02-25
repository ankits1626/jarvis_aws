#!/usr/bin/env swift
// AX Chrome Explorer - Dumps Chrome's accessibility tree to find the Claude side panel
// Usage: swift ax_chrome_explorer.swift [chrome_pid]

import Cocoa
import ApplicationServices

// MARK: - Helpers

func getAttributeValue(_ element: AXUIElement, _ attribute: String) -> AnyObject? {
    var value: AnyObject?
    let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &value)
    return result == .success ? value : nil
}

func getRole(_ element: AXUIElement) -> String {
    return getAttributeValue(element, kAXRoleAttribute) as? String ?? "unknown"
}

func getTitle(_ element: AXUIElement) -> String {
    return getAttributeValue(element, kAXTitleAttribute) as? String ?? ""
}

func getDescription(_ element: AXUIElement) -> String {
    return getAttributeValue(element, kAXDescriptionAttribute) as? String ?? ""
}

func getValue(_ element: AXUIElement) -> String {
    if let val = getAttributeValue(element, kAXValueAttribute) {
        return "\(val)"
    }
    return ""
}

func getRoleDescription(_ element: AXUIElement) -> String {
    return getAttributeValue(element, kAXRoleDescriptionAttribute) as? String ?? ""
}

func getChildren(_ element: AXUIElement) -> [AXUIElement] {
    guard let children = getAttributeValue(element, kAXChildrenAttribute) as? [AXUIElement] else {
        return []
    }
    return children
}

func getURL(_ element: AXUIElement) -> String {
    return getAttributeValue(element, "AXURL") as? String ??
           (getAttributeValue(element, "AXURLAttribute") as? String ?? "")
}

// MARK: - Tree traversal

func dumpTree(_ element: AXUIElement, depth: Int = 0, maxDepth: Int = 6, filter: String? = nil) {
    let indent = String(repeating: "  ", count: depth)
    let role = getRole(element)
    let title = getTitle(element)
    let desc = getDescription(element)
    let value = getValue(element)
    let roleDesc = getRoleDescription(element)

    // Build info string
    var info = "\(indent)[\(role)]"
    if !roleDesc.isEmpty { info += " roleDesc=\"\(roleDesc)\"" }
    if !title.isEmpty { info += " title=\"\(title.prefix(100))\"" }
    if !desc.isEmpty { info += " desc=\"\(desc.prefix(100))\"" }
    if !value.isEmpty && value.count < 200 { info += " value=\"\(value.prefix(100))\"" }

    // Check if this looks like a side panel or Claude-related element
    let lowerTitle = title.lowercased()
    let lowerDesc = desc.lowercased()
    let lowerValue = value.lowercased()
    let isInteresting = lowerTitle.contains("claude") || lowerDesc.contains("claude") ||
                        lowerTitle.contains("side panel") || lowerDesc.contains("side panel") ||
                        lowerTitle.contains("sidebar") || role == "AXWebArea" ||
                        lowerTitle.contains("chat") || lowerDesc.contains("chat")

    if isInteresting {
        info += " <<<< INTERESTING"
    }

    print(info)

    if depth >= maxDepth { return }

    for child in getChildren(element) {
        dumpTree(child, depth: depth + 1, maxDepth: maxDepth, filter: filter)
    }
}

// Find web areas specifically (likely to contain side panel content)
func findWebAreas(_ element: AXUIElement, depth: Int = 0, maxDepth: Int = 15, path: String = "") -> [(path: String, element: AXUIElement, title: String)] {
    var results: [(path: String, element: AXUIElement, title: String)] = []

    let role = getRole(element)
    let title = getTitle(element)
    let currentPath = path.isEmpty ? role : "\(path) > \(role)"

    if role == "AXWebArea" {
        results.append((path: currentPath, element: element, title: title))
    }

    if depth >= maxDepth { return results }

    for child in getChildren(element) {
        results.append(contentsOf: findWebAreas(child, depth: depth + 1, maxDepth: maxDepth, path: currentPath))
    }

    return results
}

// Dump text content from a web area (conversation messages)
func dumpTextContent(_ element: AXUIElement, depth: Int = 0, maxDepth: Int = 20) -> [String] {
    var texts: [String] = []

    let role = getRole(element)
    let value = getValue(element)
    let title = getTitle(element)

    // Collect text from static text and other text-bearing elements
    if role == "AXStaticText" && !value.isEmpty {
        texts.append(value)
    } else if role == "AXHeading" && !title.isEmpty {
        texts.append("## \(title)")
    } else if role == "AXLink" && !title.isEmpty {
        texts.append("[link: \(title)]")
    }

    if depth >= maxDepth { return texts }

    for child in getChildren(element) {
        texts.append(contentsOf: dumpTextContent(child, depth: depth + 1, maxDepth: maxDepth))
    }

    return texts
}

// MARK: - Main

// Check accessibility permission
let trusted = AXIsProcessTrustedWithOptions(
    [kAXTrustedCheckOptionPrompt.takeUnretainedValue(): true] as CFDictionary
)

if !trusted {
    print("ERROR: Accessibility permission not granted.")
    print("Go to System Settings > Privacy & Security > Accessibility")
    print("and add Terminal (or your IDE) to the allowed apps.")
    exit(1)
}

// Get Chrome PID
let args = CommandLine.arguments
let chromePID: pid_t
if args.count > 1, let pid = Int32(args[1]) {
    chromePID = pid
} else {
    // Try to find Chrome automatically
    let workspace = NSWorkspace.shared
    if let chromeApp = workspace.runningApplications.first(where: { $0.bundleIdentifier == "com.google.Chrome" }) {
        chromePID = chromeApp.processIdentifier
    } else {
        print("ERROR: Google Chrome not found. Pass PID as argument.")
        exit(1)
    }
}

print("=== Chrome AX Explorer (PID: \(chromePID)) ===\n")

let chromeApp = AXUIElementCreateApplication(chromePID)

// Step 1: Find all web areas
print("--- Finding all AXWebArea elements (tabs, extension pages, side panels) ---\n")
let webAreas = findWebAreas(chromeApp)

for (i, wa) in webAreas.enumerated() {
    print("WebArea #\(i): title=\"\(wa.title)\"")
    print("  Path: \(wa.path)")

    // Get a preview of text content
    let texts = dumpTextContent(wa.element, maxDepth: 5)
    let preview = texts.prefix(10).joined(separator: " | ")
    if !preview.isEmpty {
        print("  Preview: \(preview.prefix(300))")
    }
    print()
}

// Step 2: Dump top-level tree structure (shallow)
print("\n--- Chrome window structure (depth 4) ---\n")
dumpTree(chromeApp, maxDepth: 4)

// Step 3: Look for Claude-specific content
print("\n--- Searching for Claude-related content in all web areas ---\n")
for (i, wa) in webAreas.enumerated() {
    let texts = dumpTextContent(wa.element, maxDepth: 15)
    let claudeTexts = texts.filter { $0.lowercased().contains("claude") }
    if !claudeTexts.isEmpty {
        print("WebArea #\(i) (title=\"\(wa.title)\") contains Claude references:")
        for t in claudeTexts.prefix(20) {
            print("  - \(t.prefix(200))")
        }
        print()
    }
}

// Step 4: If we found interesting web areas, dump one fully
if webAreas.count > 1 {
    print("\n--- Full text dump of non-primary web areas (potential side panel) ---\n")
    for (i, wa) in webAreas.enumerated() {
        // Skip the first one (likely the main tab content)
        if i == 0 { continue }
        print("=== WebArea #\(i): \"\(wa.title)\" ===")
        let texts = dumpTextContent(wa.element, maxDepth: 20)
        for t in texts.prefix(100) {
            print("  \(t.prefix(300))")
        }
        print()
    }
}

print("\nDone. Found \(webAreas.count) web area(s).")
