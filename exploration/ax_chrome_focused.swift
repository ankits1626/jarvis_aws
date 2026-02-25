#!/usr/bin/env swift
// Focused AX Chrome Explorer - Extract page URL and Claude side panel last message
import Cocoa
import ApplicationServices

// MARK: - Helpers

func getAttr(_ element: AXUIElement, _ attribute: String) -> AnyObject? {
    var value: AnyObject?
    let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &value)
    return result == .success ? value : nil
}

func getRole(_ el: AXUIElement) -> String { getAttr(el, kAXRoleAttribute) as? String ?? "" }
func getTitle(_ el: AXUIElement) -> String { getAttr(el, kAXTitleAttribute) as? String ?? "" }
func getValue(_ el: AXUIElement) -> String {
    if let val = getAttr(el, kAXValueAttribute) { return "\(val)" }
    return ""
}
func getChildren(_ el: AXUIElement) -> [AXUIElement] {
    getAttr(el, kAXChildrenAttribute) as? [AXUIElement] ?? []
}
func getURL(_ el: AXUIElement) -> String {
    if let url = getAttr(el, "AXURL") {
        return "\(url)"
    }
    return ""
}
func getRoleDescription(_ el: AXUIElement) -> String {
    getAttr(el, kAXRoleDescriptionAttribute) as? String ?? ""
}
func getSubrole(_ el: AXUIElement) -> String {
    getAttr(el, "AXSubrole") as? String ?? ""
}

// Find all AXWebArea elements recursively
func findWebAreas(_ el: AXUIElement, depth: Int = 0, maxDepth: Int = 15) -> [(element: AXUIElement, title: String)] {
    var results: [(element: AXUIElement, title: String)] = []
    let role = getRole(el)
    if role == "AXWebArea" {
        results.append((element: el, title: getTitle(el)))
    }
    if depth >= maxDepth { return results }
    for child in getChildren(el) {
        results.append(contentsOf: findWebAreas(child, depth: depth + 1, maxDepth: maxDepth))
    }
    return results
}

// Extract all text from a web area with structure info
struct TextBlock {
    let role: String
    let text: String
    let depth: Int
}

func extractText(_ el: AXUIElement, depth: Int = 0, maxDepth: Int = 25) -> [TextBlock] {
    var blocks: [TextBlock] = []
    let role = getRole(el)
    let value = getValue(el)
    let title = getTitle(el)

    if role == "AXStaticText" && !value.isEmpty {
        blocks.append(TextBlock(role: role, text: value, depth: depth))
    } else if role == "AXHeading" && !title.isEmpty {
        blocks.append(TextBlock(role: role, text: "## \(title)", depth: depth))
    } else if role == "AXLink" && !title.isEmpty {
        blocks.append(TextBlock(role: role, text: "[link: \(title)]", depth: depth))
    } else if role == "AXTextField" || role == "AXTextArea" {
        let val = getValue(el)
        if !val.isEmpty {
            blocks.append(TextBlock(role: role, text: "[input: \(val)]", depth: depth))
        }
        // Also check placeholder
        if let placeholder = getAttr(el, "AXPlaceholderValue") as? String, !placeholder.isEmpty {
            blocks.append(TextBlock(role: role, text: "[placeholder: \(placeholder)]", depth: depth))
        }
    }

    if depth >= maxDepth { return blocks }
    for child in getChildren(el) {
        blocks.append(contentsOf: extractText(child, depth: depth + 1, maxDepth: maxDepth))
    }
    return blocks
}

// Find URL bar value
func findURLBar(_ el: AXUIElement, depth: Int = 0, maxDepth: Int = 8) -> String? {
    let role = getRole(el)
    let roleDesc = getRoleDescription(el)

    // Chrome's address bar is an AXTextField with specific attributes
    if role == "AXTextField" {
        let value = getValue(el)
        let desc = getAttr(el, kAXDescriptionAttribute) as? String ?? ""
        // Chrome's URL bar has description "Address and search bar" or similar
        if desc.lowercased().contains("address") || desc.lowercased().contains("url") ||
           desc.lowercased().contains("search bar") || roleDesc.lowercased().contains("address") {
            return value
        }
        // Also check if the value looks like a URL
        if value.hasPrefix("http") || value.contains(".com") || value.contains(".org") || value.contains(".io") {
            return value
        }
    }

    if depth >= maxDepth { return nil }
    for child in getChildren(el) {
        if let url = findURLBar(child, depth: depth + 1, maxDepth: maxDepth) {
            return url
        }
    }
    return nil
}

// MARK: - Main

let trusted = AXIsProcessTrustedWithOptions(
    [kAXTrustedCheckOptionPrompt.takeUnretainedValue(): true] as CFDictionary
)
if !trusted {
    print("ERROR: Accessibility permission not granted.")
    exit(1)
}

let workspace = NSWorkspace.shared
guard let chromeApp = workspace.runningApplications.first(where: { $0.bundleIdentifier == "com.google.Chrome" }) else {
    print("ERROR: Chrome not running")
    exit(1)
}

let app = AXUIElementCreateApplication(chromeApp.processIdentifier)

// 1. Find the URL bar
print("=== ACTIVE TAB URL ===\n")
if let url = findURLBar(app) {
    print("URL: \(url)")
} else {
    print("Could not find URL bar")
}

// 2. Find all web areas
let webAreas = findWebAreas(app)
print("\n=== WEB AREAS FOUND: \(webAreas.count) ===\n")

for (i, wa) in webAreas.enumerated() {
    print("WebArea #\(i): \"\(wa.title)\"")
}

// 3. Find the Claude side panel and extract conversation
print("\n=== CLAUDE SIDE PANEL CONTENT ===\n")

let claudeAreas = webAreas.filter { $0.title.lowercased().contains("claude") }
if claudeAreas.isEmpty {
    print("No Claude side panel found. Is it open?")
} else {
    for ca in claudeAreas {
        print("--- \(ca.title) ---\n")
        let blocks = extractText(ca.element)

        // Print all text blocks
        print("Full conversation (\(blocks.count) text blocks):\n")

        // Group into messages by looking at structure
        var allText: [String] = []
        for block in blocks {
            allText.append(block.text)
        }

        // Print last N lines (likely the last message)
        let lastN = 30
        print("--- LAST \(lastN) TEXT BLOCKS (likely last message) ---\n")
        let startIdx = max(0, allText.count - lastN)
        for i in startIdx..<allText.count {
            print("  [\(i)] \(allText[i].prefix(300))")
        }

        // Also print the first few (likely user's message)
        print("\n--- FIRST 10 TEXT BLOCKS (likely user's prompt) ---\n")
        for i in 0..<min(10, allText.count) {
            print("  [\(i)] \(allText[i].prefix(300))")
        }

        print("\n--- TOTAL TEXT BLOCKS: \(allText.count) ---")
    }
}

// 4. Also get the active tab's page title
print("\n=== ACTIVE PAGE ===\n")
let pageAreas = webAreas.filter { !$0.title.lowercased().contains("claude") }
for pa in pageAreas {
    print("Page: \"\(pa.title)\"")
    // Get first few text blocks as preview
    let blocks = extractText(pa.element, maxDepth: 5)
    let preview = blocks.prefix(5).map { $0.text }.joined(separator: " | ")
    print("Preview: \(preview.prefix(300))")
}
