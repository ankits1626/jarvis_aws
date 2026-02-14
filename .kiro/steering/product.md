# JarvisListen — System Audio Capture CLI

## Purpose
A macOS command-line tool that captures ALL audio from any conversation
happening on the Mac — both the user's microphone input AND system audio
output from apps like Zoom, Google Meet, Slack, WhatsApp, Teams, FaceTime.

## Context
This is the "Listen" module of JARVIS, an AI assistant for the AWS 10,000
AIdeas Competition. The pipeline is:
  Listen (this module) → Transcribe → Augment (Bedrock RAG) → Display

## What It Does
- Captures system audio (other participants' voices) via ScreenCaptureKit
- Captures microphone audio (user's voice) via ScreenCaptureKit captureMicrophone
- Outputs both as a stereo PCM stream to stdout
- Channel 0 (left) = microphone, Channel 1 (right) = system audio

## Target Platform
- macOS 15.0+ (required for captureMicrophone API)
- Apple Silicon (arm64)
- Swift 5.9+

## Design Principles
- Unix philosophy: does one thing, outputs to stdout, composable via pipes
- Zero external dependencies — Apple frameworks only
- Downstream consumer reads stdin, knows nothing about how audio was captured