# Project Structure

## Layout
- Sources/ — all Swift source files (flat, no subdirectories for this small module)
- Sources/main.swift — entry point, argument parsing, signal handling, run loop
- Sources/AudioCapture.swift — ScreenCaptureKit wrapper, SCStream setup
- Sources/PCMConverter.swift — audio format conversion + stereo interleaving
- Sources/AudioCaptureProvider.swift — protocol definition for swappable backends

## Naming
- Types: PascalCase (AudioCapture, PCMConverter)
- Methods/properties: camelCase (startCapture, sampleRate)
- Constants: camelCase (defaultSampleRate)
- Files: PascalCase matching the primary type they contain

## Output
- Binary name: JarvisListen
- All PCM data goes to stdout (FileHandle.standardOutput)
- All log/error messages go to stderr (FileHandle.standardError)