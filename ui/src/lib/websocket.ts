import { useEffect, useRef, useCallback, useState } from 'react'
import { useAppStore } from '../store/appStore'

export interface WebSocketMessage {
  type: string
  payload: unknown
}

export function useWebSocket(url?: string) {
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const { gatewayUrl, addMessage } = useAppStore()
  
  const connect = useCallback(() => {
    const wsUrl = url || gatewayUrl.replace('http', 'ws') + '/ws'
    const ws = new WebSocket(wsUrl)
    
    ws.onopen = () => {
      console.log('WebSocket connected')
    }
    
    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data)
        
        switch (message.type) {
          case 'message':
            addMessage({
              id: message.payload.id || Date.now().toString(),
              content: message.payload.content,
              sender: 'agent',
              timestamp: new Date(message.payload.timestamp || Date.now()),
            })
            break
          case 'presence':
            console.log('Presence update:', message.payload)
            break
          case 'error':
            console.error('WebSocket error:', message.payload)
            break
        }
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error)
      }
    }
    
    ws.onclose = () => {
      console.log('WebSocket disconnected, reconnecting...')
      reconnectTimeoutRef.current = setTimeout(connect, 3000)
    }
    
    ws.onerror = (error) => {
      console.error('WebSocket error:', error)
    }
    
    wsRef.current = ws
  }, [url, gatewayUrl, addMessage])
  
  const send = useCallback((message: WebSocketMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message))
    }
  }, [])
  
  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
    }
    wsRef.current?.close()
  }, [])
  
  useEffect(() => {
    connect()
    return () => disconnect()
  }, [connect, disconnect])
  
  return { send, disconnect, reconnect: connect }
}

interface SpeechRecognitionEvent extends Event {
  results: SpeechRecognitionResultList
  resultIndex: number
}

interface SpeechRecognitionErrorEvent extends Event {
  error: string
}

interface SpeechRecognition extends EventTarget {
  continuous: boolean
  interimResults: boolean
  onstart: (() => void) | null
  onresult: ((event: SpeechRecognitionEvent) => void) | null
  onerror: ((event: SpeechRecognitionErrorEvent) => void) | null
  onend: (() => void) | null
  start: () => void
  stop: () => void
}

declare global {
  interface Window {
    SpeechRecognition: new () => SpeechRecognition
    webkitSpeechRecognition: new () => SpeechRecognition
  }
}

export function useVoice() {
  const recognitionRef = useRef<SpeechRecognition | null>(null)
  const [isListening, setIsListening] = useState(false)
  const [transcript, setTranscript] = useState('')
  
  const startListening = useCallback(() => {
    const SpeechRecognitionClass = window.SpeechRecognition || window.webkitSpeechRecognition
    if (!SpeechRecognitionClass) {
      console.error('Speech recognition not supported')
      return
    }
    
    const recognition = new SpeechRecognitionClass()
    recognition.continuous = true
    recognition.interimResults = true
    
    recognition.onstart = () => {
      setIsListening(true)
    }
    
    recognition.onresult = (event: SpeechRecognitionEvent) => {
      let finalTranscript = ''
      for (let i = event.resultIndex; i < event.results.length; i++) {
        const transcript = event.results[i][0].transcript
        if (event.results[i].isFinal) {
          finalTranscript += transcript
        }
      }
      if (finalTranscript) {
        setTranscript(finalTranscript)
      }
    }
    
    recognition.onerror = (event: SpeechRecognitionErrorEvent) => {
      console.error('Speech recognition error:', event.error)
      setIsListening(false)
    }
    
    recognition.onend = () => {
      setIsListening(false)
    }
    
    recognition.start()
    recognitionRef.current = recognition
  }, [])
  
  const stopListening = useCallback(() => {
    recognitionRef.current?.stop()
    setIsListening(false)
  }, [])
  
  return { isListening, transcript, startListening, stopListening }
}
