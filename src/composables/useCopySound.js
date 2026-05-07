let copySoundContext = null
let copySoundPending = false
let copySoundResumePromise = null

function getCopySoundContext() {
  const AudioContext = window.AudioContext || window.webkitAudioContext
  if (!AudioContext) {
    return null
  }

  if (!copySoundContext || copySoundContext.state === 'closed') {
    copySoundContext = new AudioContext()
  }

  return copySoundContext
}

function playGeneratedCopyTone(context) {
  const startTime = context.currentTime
  const duration = 0.06
  const endTime = startTime + duration
  const oscillator = context.createOscillator()
  const filter = context.createBiquadFilter()
  const gain = context.createGain()

  oscillator.type = 'triangle'
  oscillator.frequency.setValueAtTime(625, startTime)
  oscillator.frequency.exponentialRampToValueAtTime(460, endTime)

  filter.type = 'bandpass'
  filter.frequency.setValueAtTime(700, startTime)
  filter.Q.setValueAtTime(6, startTime)

  gain.gain.setValueAtTime(0.0001, startTime)
  gain.gain.exponentialRampToValueAtTime(0.28, startTime + 0.004)
  gain.gain.exponentialRampToValueAtTime(0.0001, endTime)

  oscillator.connect(filter)
  filter.connect(gain)
  gain.connect(context.destination)

  oscillator.start(startTime)
  oscillator.stop(endTime + 0.01)
}

function markCopySoundPending() {
  copySoundPending = true
}

function clearPendingCopySound() {
  copySoundPending = false
}

function tryPlayCopySound(context) {
  try {
    playGeneratedCopyTone(context)
    clearPendingCopySound()
    return true
  } catch (error) {
    console.warn('Failed to play copy sound', error)
    return false
  }
}

function resumeCopySoundContext(context) {
  if (copySoundResumePromise) {
    return copySoundResumePromise
  }

  copySoundResumePromise = context
    .resume()
    .then(() => {
      copySoundResumePromise = null
      return true
    })
    .catch((error) => {
      copySoundResumePromise = null
      console.warn('Failed to resume copy sound context', error)
      return false
    })

  return copySoundResumePromise
}

export function playCopySoundFallback() {
  const context = getCopySoundContext()
  if (!context) {
    return
  }

  if (context.state === 'suspended') {
    markCopySoundPending()
    void resumeCopySoundContext(context).then((resumed) => {
      if (resumed) {
        tryPlayCopySound(context)
      }
    })
    return
  }

  tryPlayCopySound(context)
}

export function flushPendingCopySound() {
  if (!copySoundPending) {
    return
  }

  const context = getCopySoundContext()
  if (!context) {
    return
  }

  if (context.state === 'running') {
    tryPlayCopySound(context)
    return
  }

  if (context.state === 'suspended') {
    void resumeCopySoundContext(context).then((resumed) => {
      if (resumed) {
        tryPlayCopySound(context)
      }
    })
  }
}
