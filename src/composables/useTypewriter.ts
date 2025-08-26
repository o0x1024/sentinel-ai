import { ref, nextTick } from 'vue'

interface TypewriterState {
  fullContent: string
  displayedContent: string
  isTyping: boolean
  currentIndex: number
  intervalId: number | null
  pendingContent?: string
}

export const useTypewriter = () => {
  const typewriterState = ref<Map<string, TypewriterState>>(new Map())
  const enableTypewriter = ref(true)
  const typewriterSpeed = ref(30)

  // Start typewriter effect for incremental content
  const startTypewriterIncremental = (messageId: string, initialContent: string, speed: number = 30) => {
    const state = typewriterState.value.get(messageId)
    if (!state) return

    if (state.intervalId) {
      clearTimeout(state.intervalId)
      state.intervalId = null
    }

    state.isTyping = true

    if (!enableTypewriter.value) {
      state.displayedContent = state.fullContent
      state.currentIndex = state.fullContent.length
      state.isTyping = false
      return
    }

    const dynamicSpeed = Math.max(20, Math.min(speed * 1.2, 60))

    const typeNextChar = () => {
      const currentState = typewriterState.value.get(messageId)
      if (!currentState || !currentState.isTyping) return

      if (currentState.pendingContent && currentState.pendingContent.length > 0) {
        currentState.pendingContent = ''
      }

      if (currentState.currentIndex < currentState.fullContent.length) {
        const remainingChars = currentState.fullContent.length - currentState.currentIndex
        const charsToAdd = Math.min(remainingChars, Math.max(1, Math.floor(dynamicSpeed / 15)))

        currentState.currentIndex += charsToAdd
        currentState.displayedContent = currentState.fullContent.substring(0, currentState.currentIndex)

        const baseDelay = 1000 / dynamicSpeed
        const adaptiveDelay = remainingChars > 200 ? baseDelay * 0.7 : baseDelay
        const randomDelay = adaptiveDelay + (Math.random() - 0.5) * (adaptiveDelay * 0.1)

        currentState.intervalId = window.setTimeout(typeNextChar, Math.max(10, randomDelay))
      } else {
        if (currentState.intervalId) {
          clearTimeout(currentState.intervalId)
          currentState.intervalId = null
        }
      }
    }

    state.intervalId = window.setTimeout(typeNextChar, 20)
  }

  // Update typewriter content incrementally
  const updateTypewriterContentIncremental = (messageId: string, deltaContent: string) => {
    console.log('Updating typewriter content incrementally:', {
      messageId,
      deltaLength: deltaContent.length,
      enableTypewriter: enableTypewriter.value
    })
    
    let state = typewriterState.value.get(messageId)

    if (!state) {
      console.log('Creating new typewriter state for message:', messageId)
      const newState: TypewriterState = {
        fullContent: deltaContent,
        displayedContent: '',
        isTyping: false,
        currentIndex: 0,
        intervalId: null,
        pendingContent: ''
      }
      typewriterState.value.set(messageId, newState)
      state = newState

      if (enableTypewriter.value && deltaContent.length > 0) {
        startTypewriterIncremental(messageId, deltaContent, typewriterSpeed.value)
      } else if (!enableTypewriter.value) {
        state.displayedContent = deltaContent
        state.currentIndex = deltaContent.length
      }
    } else {
      console.log('Appending to existing typewriter state:', {
        currentLength: state.fullContent.length,
        newDeltaLength: deltaContent.length
      })
      state.fullContent += deltaContent

      if (enableTypewriter.value) {
        if (state.isTyping && state.intervalId) {
          if (!state.pendingContent) state.pendingContent = ''
          state.pendingContent += deltaContent
        } else {
          appendToTypewriter(messageId, deltaContent)
        }
      } else {
        state.displayedContent = state.fullContent
        state.currentIndex = state.fullContent.length
      }
    }
  }

  // Update typewriter content (non-incremental, replaces content)
  const updateTypewriterContent = (messageId: string, fullContent: string) => {
    let state = typewriterState.value.get(messageId)

    if (!state) {
      const newState: TypewriterState = {
        fullContent: fullContent,
        displayedContent: '',
        isTyping: false,
        currentIndex: 0,
        intervalId: null,
        pendingContent: ''
      }
      typewriterState.value.set(messageId, newState)
      state = newState
    } else {
      // Stop current typing and reset content
      if (state.intervalId) {
        clearTimeout(state.intervalId)
        state.intervalId = null
      }
      state.fullContent = fullContent
      state.currentIndex = 0
      state.displayedContent = ''
      state.isTyping = false
    }

    if (enableTypewriter.value && fullContent.length > 0) {
      startTypewriterIncremental(messageId, fullContent, typewriterSpeed.value)
    } else if (!enableTypewriter.value) {
      state.displayedContent = fullContent
      state.currentIndex = fullContent.length
    }
  }

  // Append new content to an active typewriter
  const appendToTypewriter = (messageId: string, newContent: string) => {
    const state = typewriterState.value.get(messageId)
    if (!state) return

    if (!state.isTyping || !state.intervalId) {
      state.isTyping = true
      const dynamicSpeed = Math.max(30, Math.min(typewriterSpeed.value * 1.5, 80))

      const typeNextChar = () => {
        const currentState = typewriterState.value.get(messageId)
        if (!currentState || !currentState.isTyping) return

        if (currentState.pendingContent && currentState.pendingContent.length > 0) {
          currentState.pendingContent = ''
        }

        if (currentState.currentIndex < currentState.fullContent.length) {
          const remainingChars = currentState.fullContent.length - currentState.currentIndex
          const charsToAdd = Math.min(remainingChars, Math.max(1, Math.floor(dynamicSpeed / 20)))

          currentState.currentIndex += charsToAdd
          currentState.displayedContent = currentState.fullContent.substring(0, currentState.currentIndex)

          const baseDelay = 1000 / dynamicSpeed
          const adaptiveDelay = remainingChars > 100 ? baseDelay * 0.5 : baseDelay
          const randomDelay = adaptiveDelay + (Math.random() - 0.5) * (adaptiveDelay * 0.1)

          currentState.intervalId = window.setTimeout(typeNextChar, Math.max(5, randomDelay))
        } else {
          if (currentState.intervalId) {
            clearTimeout(currentState.intervalId)
            currentState.intervalId = null
          }
        }
      }

      state.intervalId = window.setTimeout(typeNextChar, 5)
    } else {
      if (!state.pendingContent) state.pendingContent = ''
      state.pendingContent += newContent
    }
  }

  // Stop typewriter effect
  const stopTypewriter = (messageId: string) => {
    const state = typewriterState.value.get(messageId)
    if (state) {
      state.isTyping = false
      if (state.intervalId) {
        clearTimeout(state.intervalId)
        state.intervalId = null
      }

      if (state.fullContent) {
        state.displayedContent = state.fullContent
        state.currentIndex = state.fullContent.length
      }
    }
  }

  // Skip typewriter effect
  const skipTypewriter = (messageId: string) => {
    const state = typewriterState.value.get(messageId)
    if (state && state.isTyping) {
      stopTypewriter(messageId)
    }
  }

  // Get displayed content for typewriter effect
  const getDisplayedTypewriterContent = (messageId: string): string => {
    const state = typewriterState.value.get(messageId)
    if (state) {
      if (state.isTyping) {
        return state.displayedContent || ''
      }
      if (state.fullContent) {
        return state.fullContent
      }
    }
    return ''
  }

  // Check if message is currently typing
  const isMessageTyping = (messageId: string) => {
    const state = typewriterState.value.get(messageId)
    return state?.isTyping || false
  }

  // Get final content from typewriter state
  const getFinalContentFromTypewriterState = (messageId: string): string | null => {
    const state = typewriterState.value.get(messageId)
    return state?.fullContent || null
  }

  // Debug helpers
  const getTypewriterMode = (messageId: string): string => {
    const state = typewriterState.value.get(messageId)
    if (!state) return 'No State'
    if (state.isTyping && state.intervalId) return '📝 Incremental Typing'
    if (state.isTyping) return '⏸️ Typing Paused'
    if (state.fullContent.length > 0) return '✅ Content Ready'
    return '🔄 Initialized'
  }

  const getTypewriterProgress = (messageId: string): string => {
    const state = typewriterState.value.get(messageId)
    if (!state) return '0/0 (0%)'
    const percentage = state.fullContent.length > 0 ? Math.round((state.currentIndex / state.fullContent.length) * 100) : 0
    return `${state.currentIndex}/${state.fullContent.length} (${percentage}%)`
  }

  // Clean up all typewriter states
  const cleanupTypewriter = () => {
    for (const messageId of typewriterState.value.keys()) {
      stopTypewriter(messageId)
    }
    typewriterState.value.clear()
  }

  return {
    typewriterState,
    enableTypewriter,
    typewriterSpeed,
    startTypewriterIncremental,
    updateTypewriterContentIncremental,
    updateTypewriterContent,
    appendToTypewriter,
    stopTypewriter,
    skipTypewriter,
    getDisplayedTypewriterContent,
    isMessageTyping,
    getFinalContentFromTypewriterState,
    getTypewriterMode,
    getTypewriterProgress,
    cleanupTypewriter
  }
}