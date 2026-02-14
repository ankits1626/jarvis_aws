# Technology Stack

## Language
- Swift 5.9+ with Swift concurrency (async/await)

## Frameworks (Apple — no external deps)
- ScreenCaptureKit — SCStream for system audio + microphone capture
- CoreMedia — CMSampleBuffer handling
- AVFoundation — AVAudioConverter, AVAudioPCMBuffer, AVAudioFormat
- Foundation — FileHandle (stdout), signal handling, argument parsing

## Audio Format
- Output: 16kHz sample rate, 16-bit signed integer, little-endian
- Stereo: Ch0 = microphone, Ch1 = system audio
- Chunk size: 100ms (6,400 bytes per chunk for stereo)

## Key APIs
- SCShareableContent.current — discover displays/apps
- SCContentFilter — define capture scope
- SCStreamConfiguration — capturesAudio, captureMicrophone, excludesCurrentProcessAudio
- SCStream — start/stop capture
- SCStreamOutput — delegate callbacks for .audio and .microphone types
- AVAudioConverter — resample to 16kHz mono s16le per channel
- CMSampleBuffer — extract AudioBufferList

## Coding Conventions
- Swift concurrency with async/await (no completion handlers)
- Protocol-oriented: AudioCaptureProvider protocol for swappable backends
- No force unwraps — proper error handling with do/try/catch
- All logging to stderr (stdout is reserved for PCM data)