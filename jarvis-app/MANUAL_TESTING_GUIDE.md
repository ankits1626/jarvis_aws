# Manual Testing Guide for JarvisApp

This guide provides step-by-step instructions for manually testing all end-to-end flows in JarvisApp.

## Prerequisites

1. **macOS 15.0+** with Apple Silicon (arm64)
2. **JarvisListen sidecar binary** built and placed in `src-tauri/binaries/`
3. **Permissions granted**:
   - Screen Recording permission (System Settings → Privacy & Security → Screen Recording)
   - Microphone permission (System Settings → Privacy & Security → Microphone)

## Building and Running

```bash
cd jarvis-app
npm install
npm run tauri dev
```

## Test Scenarios

### 1. Complete Recording Lifecycle on macOS

**Objective**: Verify the full recording flow from start to stop.

**Steps**:
1. Launch the application
2. Verify the UI shows "Ready to record" status
3. Click the "Start Recording" button
4. **Expected**: 
   - Button changes to "Stop Recording" with red color and pulsing animation
   - Status shows "Recording..." with elapsed time counter
   - Timer increments every second
5. Wait 10-15 seconds
6. Click "Stop Recording"
7. **Expected**:
   - Button shows spinner and "Processing..." text
   - Button becomes disabled during processing
   - After a moment, returns to "Ready to record"
   - New recording appears in the recordings list
8. Verify the recording shows:
   - Filename in format `YYYYMMDD_HHMMSS.pcm`
   - Timestamp (formatted date/time)
   - Duration (in MM:SS format)
   - File size (in KB/MB)

**Success Criteria**:
- ✅ Recording starts and stops without errors
- ✅ PCM file is created in app data directory
- ✅ Recording appears in list with correct metadata
- ✅ UI transitions smoothly between states

---

### 2. Permission Error Handling and Recovery

**Objective**: Test the permission error flow and recovery mechanism.

**Steps**:
1. **Revoke permissions** (if currently granted):
   - Go to System Settings → Privacy & Security → Screen Recording
   - Uncheck JarvisApp
2. Launch the application
3. Click "Start Recording"
4. **Expected**:
   - Permission error dialog appears
   - Dialog shows message about Screen Recording/Microphone permissions
   - Three buttons: "Open System Settings", "Retry", "Close"
5. Click "Open System Settings"
6. **Expected**: System Settings opens to Privacy & Security
7. Grant Screen Recording permission to JarvisApp
8. Return to JarvisApp
9. Click "Retry" in the permission dialog
10. **Expected**:
    - Dialog closes
    - Recording starts successfully
    - Status shows "Recording..."

**Success Criteria**:
- ✅ Permission error is detected and dialog appears
- ✅ "Open System Settings" button opens correct settings page
- ✅ "Retry" successfully starts recording after permissions granted
- ✅ Error dialog has fade-in animation

---

### 3. Playback with Various Recording Lengths

**Objective**: Test audio playback functionality.

**Steps**:
1. Create 3 recordings of different lengths:
   - Short: 5 seconds
   - Medium: 30 seconds
   - Long: 2+ minutes
2. For each recording:
   - Click on the recording row
   - **Expected**:
     - Recording row highlights with blue background
     - Audio player appears below with fade-in animation
     - Player shows "Playing: [filename]"
     - HTML5 audio controls are visible
     - Audio starts playing automatically
   - Test play/pause controls
   - Test seek bar (drag to different positions)
   - Let audio play to completion
   - **Expected**: Audio resets to beginning (currentTime = 0)
   - Click the close button (✕)
   - **Expected**: Player closes, recording deselects

**Success Criteria**:
- ✅ WAV conversion completes without errors
- ✅ Audio plays correctly for all recording lengths
- ✅ Seek bar works properly
- ✅ Playback resets to beginning on completion
- ✅ Player closes cleanly

---

### 4. Deletion with Confirmation

**Objective**: Test recording deletion flow.

**Steps**:
1. Create a test recording
2. Hover over the recording row
3. **Expected**: Delete button (🗑️) becomes more visible
4. Click the delete button
5. **Expected**: Browser confirmation dialog appears with message "Delete recording '[filename]'?"
6. Click "Cancel"
7. **Expected**: Recording remains in list
8. Click delete button again
9. Click "OK" in confirmation dialog
10. **Expected**:
    - Recording disappears from list immediately
    - PCM file is deleted from disk
    - If recording was selected, audio player closes

**Success Criteria**:
- ✅ Confirmation prompt appears before deletion
- ✅ Cancel preserves the recording
- ✅ Confirm deletes the recording
- ✅ List updates immediately after deletion
- ✅ Audio player closes if deleted recording was playing

---

### 5. Global Shortcut (Cmd+Shift+R)

**Objective**: Test keyboard shortcut for hands-free recording control.

**Steps**:
1. Launch the application
2. Switch to another application (e.g., browser, text editor)
3. Press **Cmd+Shift+R**
4. **Expected**:
   - Recording starts (even though JarvisApp is in background)
   - Switch back to JarvisApp to verify status shows "Recording..."
5. Press **Cmd+Shift+R** again
6. **Expected**:
   - Recording stops
   - Status returns to "Ready to record"
   - New recording appears in list

**Success Criteria**:
- ✅ Shortcut works when app is in background
- ✅ Shortcut toggles recording on/off
- ✅ UI updates correctly when switching back to app
- ✅ If shortcut registration fails, app continues without error

---

### 6. Error Scenarios

**Objective**: Test various error conditions.

#### 6.1 Concurrent Recording Attempt

**Steps**:
1. Start a recording
2. While recording is active, click "Start Recording" again (if button is somehow accessible)
3. **Expected**: Inline error message appears: "A recording is already in progress"

#### 6.2 Missing Sidecar Binary

**Steps**:
1. Temporarily rename the sidecar binary in `src-tauri/binaries/`
2. Launch the application
3. Try to start recording
4. **Expected**: Error message about missing sidecar binary

#### 6.3 Sidecar Crash During Recording

**Steps**:
1. Start a recording
2. Manually kill the JarvisListen process:
   ```bash
   pkill -9 JarvisListen
   ```
3. **Expected**:
   - Error toast appears: "Recording process crashed with code [X]"
   - Status returns to "Ready to record"
   - Dismiss button closes error

#### 6.4 File I/O Errors

**Steps**:
1. Fill up disk space (or use a test directory with no write permissions)
2. Try to start recording
3. **Expected**: Error message includes file path and system error description

**Success Criteria**:
- ✅ All errors display user-friendly messages
- ✅ Errors can be dismissed
- ✅ App returns to idle state after errors
- ✅ Error toasts have fade-in animation

---

### 7. UI/UX Polish

**Objective**: Verify animations, loading states, and responsive design.

#### 7.1 Loading States

**Steps**:
1. Launch the application
2. **Expected**: Skeleton loaders appear while recordings are loading
3. After loading completes, skeleton loaders fade out and recordings appear

#### 7.2 Animations

**Steps**:
1. Verify the following animations work:
   - ✅ Recording button pulses when recording is active
   - ✅ Spinner appears in "Processing..." button
   - ✅ Status text "Recording..." pulses
   - ✅ Error toasts fade in
   - ✅ Permission dialog slides up with fade-in
   - ✅ Recording rows slide right on hover
   - ✅ Skeleton loaders pulse

#### 7.3 Responsive Design

**Steps**:
1. Resize the window to different sizes:
   - Large (800px+)
   - Medium (768px)
   - Small (480px)
2. **Expected**: Layout adapts appropriately at each breakpoint
3. Verify all elements remain accessible and readable

**Success Criteria**:
- ✅ All animations are smooth and performant
- ✅ Loading states provide clear feedback
- ✅ Responsive layout works at all sizes

---

### 8. Recordings List Behavior

**Objective**: Test list display and sorting.

**Steps**:
1. Create 5+ recordings at different times
2. **Expected**: Recordings are sorted newest first (descending by created_at)
3. Verify each recording displays:
   - Filename
   - Formatted timestamp
   - Duration in MM:SS format
   - File size in KB/MB
4. Scroll through the list
5. **Expected**: Custom scrollbar appears (purple/blue themed)
6. When list is empty:
   - **Expected**: Message "No recordings yet. Start recording to create your first one!"

**Success Criteria**:
- ✅ Recordings sorted correctly (newest first)
- ✅ All metadata displays correctly
- ✅ Scrollbar is styled and functional
- ✅ Empty state message appears when appropriate

---

## Platform-Specific Notes

### macOS (Supported)
- All features should work
- Test with actual audio (play music, join a call)
- Verify both microphone and system audio are captured

### Windows/Linux (Not Supported)
- App should build and run
- Attempting to record should show error: "not yet supported"
- Error should be user-friendly and explain platform limitation

---

## Troubleshooting

### Recording doesn't start
- Check permissions in System Settings
- Check Console.app for error messages
- Verify sidecar binary exists and is executable

### No audio in playback
- Verify PCM file has data (file size > 0)
- Check WAV conversion in browser console
- Verify audio element src is set correctly

### Shortcut doesn't work
- Check Console.app for shortcut registration errors
- Verify no other app is using Cmd+Shift+R
- App should continue working even if shortcut fails

---

## Reporting Issues

When reporting issues, include:
1. macOS version
2. Steps to reproduce
3. Expected vs actual behavior
4. Console logs (both app and browser DevTools)
5. Screenshots/screen recordings if applicable

---

## Success Checklist

After completing all tests, verify:

- [ ] Recording lifecycle works end-to-end
- [ ] Permission errors are handled gracefully
- [ ] Playback works for various recording lengths
- [ ] Deletion requires confirmation and works correctly
- [ ] Global shortcut toggles recording
- [ ] All error scenarios display appropriate messages
- [ ] All animations and loading states work
- [ ] Responsive design adapts to different window sizes
- [ ] Recordings list displays and sorts correctly
- [ ] Empty state message appears when appropriate

If all items are checked, the application is ready for deployment! 🎉
