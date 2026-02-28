import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp?: string;
}

interface ChatPanelProps {
  sessionId: string;
  recordingFilename: string;
  status: 'preparing' | 'ready' | 'error';
  preparingMessage?: string;
  placeholder?: string;
}

export default function ChatPanel({
  sessionId,
  recordingFilename,
  status: initialStatus,
  preparingMessage = 'Preparing...',
  placeholder = 'Ask me anything about this recording.'
}: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const [status, setStatus] = useState(initialStatus);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to latest message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, thinking]);

  // Listen for chat-status events from background preparation
  useEffect(() => {
    const unlisten = listen('chat-status', (event: any) => {
      const { status: newStatus, message } = event.payload;
      if (newStatus === 'ready') {
        setStatus('ready');
      } else if (newStatus === 'error') {
        setStatus('error');
        setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${message}` }]);
      }
    });

    return () => {
      unlisten.then((fn: UnlistenFn) => fn());
    };
  }, []);

  const handleSendMessage = async () => {
    if (!input.trim() || thinking) return;

    const userMessage = input.trim();
    setInput('');
    
    // Add user message immediately
    setMessages(prev => [...prev, { role: 'user', content: userMessage }]);
    setThinking(true);

    try {
      const response = await invoke<string>('chat_send_message', {
        sessionId,
        recordingFilename,
        message: userMessage
      });

      // Add assistant response
      setMessages(prev => [...prev, { role: 'assistant', content: response }]);
    } catch (error) {
      // Display error as assistant message
      const errorMessage = `Error: ${error}`;
      setMessages(prev => [...prev, { role: 'assistant', content: errorMessage }]);
    } finally {
      setThinking(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  // Preparing state view
  if (status === 'preparing') {
    return (
      <div className="chat-panel">
        <div className="chat-preparing">
          <div className="spinner" />
          <span>{preparingMessage}</span>
        </div>
      </div>
    );
  }

  // Ready state - show chat interface
  return (
    <div className="chat-panel">
      <div className="chat-messages">
        {messages.length === 0 && !thinking && (
          <div className="chat-empty">{placeholder}</div>
        )}
        
        {messages.map((msg, index) => (
          <div key={index} className={`chat-message chat-${msg.role}`}>
            <div className="chat-bubble">{msg.content}</div>
          </div>
        ))}
        
        {thinking && (
          <div className="chat-message chat-assistant">
            <div className="chat-bubble thinking">Thinking...</div>
          </div>
        )}
        
        <div ref={messagesEndRef} />
      </div>

      <div className="chat-input-bar">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyPress={handleKeyPress}
          placeholder="Type your message..."
          disabled={thinking}
          className="chat-input"
        />
        <button
          onClick={handleSendMessage}
          disabled={!input.trim() || thinking}
          className="chat-send-button"
        >
          Send
        </button>
      </div>
    </div>
  );
}
