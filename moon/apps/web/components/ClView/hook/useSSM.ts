import { useEffect, useRef, useState } from 'react'
import { useAtom } from 'jotai'

import { orionApiClient } from '@/utils/queryClient'

import { loadingAtom, logsAtomFamily, statusAtom } from '../components/Checks/cpns/store'

/**
 * Get SSE URL for task output streaming
 * Corresponds to orionApiClient.id.getTaskOutputById() API endpoint
 * Path: GET:/task-output/{id}
 */
export const getTaskOutputSSEUrl = (taskId: string) => {
  const baseUrl = (orionApiClient as any).baseUrl || ''

  return `${baseUrl}/task-output/${taskId}`
}

export const useSSM = () => {
  const sseUrl = useRef('')
  const createEventSource = (baseUrl: string): Promise<EventSource> => {
    return new Promise<EventSource>((res, rej) => {
      const es = new EventSource(baseUrl)

      es.onopen = () => {
        res(es)
      }
      es.onerror = () => {
        rej('eventsource connection failed')
      }
    })
  }

  const initial = () => {
    const baseUrl = (orionApiClient as any).baseUrl || ''

    sseUrl.current = `${baseUrl}/logs?follow=true`
  }

  return {
    createEventSource,
    initial,
    sseUrl
  }
}

// SSE log lines can arrive dozens of times per second while a build is running.
// Pushing each line straight into state re-renders the viewer on every line,
// which shows up as flicker. Instead we buffer incoming lines and flush them
// together at most once per interval.
const LOG_FLUSH_INTERVAL_MS = 150

export const useTaskSSE = (cl: string) => {
  const eventSourcesRef = useRef<Record<string, EventSource>>({})
  // Per-task "force flush remaining buffer + cancel pending timer". Used when a
  // stream ends or the component unmounts so we never drop the trailing lines.
  const flushersRef = useRef<Record<string, () => void>>({})
  const [logsMap, setLogsMap] = useAtom(logsAtomFamily(cl))
  const [_, setLoading] = useAtom(loadingAtom)
  const [_status, setStatus] = useAtom(statusAtom)

  const setEventSource: (taskId: string) => void = (taskId: string) => {
    if (eventSourcesRef.current[taskId]) return

    const es = new EventSource(getTaskOutputSSEUrl(taskId))

    // Accumulate incoming lines here and commit them to state in batches.
    let buffer = ''
    let timer: ReturnType<typeof setTimeout> | null = null

    const flush = () => {
      timer = null

      if (!buffer) return

      const chunk = buffer

      buffer = ''

      setLogsMap((prev) => {
        const prevLogs = prev[taskId] ?? ''

        if (prevLogs.endsWith(chunk)) {
          return prev
        }

        return {
          ...prev,
          [taskId]: prevLogs + chunk
        }
      })

      setLoading(false)
    }

    const scheduleFlush = () => {
      if (timer != null) return

      timer = setTimeout(flush, LOG_FLUSH_INTERVAL_MS)
    }

    const finalize = () => {
      if (timer != null) {
        clearTimeout(timer)
        timer = null
      }

      flush()
    }

    flushersRef.current[taskId] = finalize

    const appendLog = (data: string) => {
      buffer += data + '\n'
      scheduleFlush()
    }

    es.onmessage = (e) => {
      appendLog(e.data)
    }

    es.addEventListener('log', (e) => {
      appendLog(e.data)
    })

    // status
    es.addEventListener('buildResult', (e) => {
      const result = JSON.parse(e.data)

      // Make sure any buffered tail lines land before the stream closes.
      finalize()

      setStatus((prev) => {
        return {
          ...prev,
          [taskId]: result.status
        }
      })
      es.close()
    })

    es.onerror = () => {
      finalize()
      es.close()
    }

    eventSourcesRef.current[taskId] = es
  }

  const closeEventSource = (taskId: string) => {
    flushersRef.current[taskId]?.()
    delete flushersRef.current[taskId]
    eventSourcesRef.current[taskId]?.close()
    delete eventSourcesRef.current[taskId]
  }

  useEffect(() => {
    const flushers = flushersRef.current

    return () => {
      Object.values(flushers).forEach((finalize) => finalize())
      Object.values(eventSourcesRef.current).forEach((es) => es.close())
      eventSourcesRef.current = {}
      flushersRef.current = {}
      setLoading(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return { eventSourcesRef, setEventSource, closeEventSource, logsMap, setLogsMap }
}

export const useMultiTaskSSE = (taskIds: string[]) => {
  const [eventsMap, setEventsMap] = useState<Record<string, string[]>>({})
  const eventSourcesRef = useRef<Record<string, EventSource>>({})

  useEffect(() => {
    Object.keys(eventSourcesRef.current).forEach((id) => {
      if (!taskIds.includes(id)) {
        eventSourcesRef.current[id].close()
        delete eventSourcesRef.current[id]
      }
    })

    taskIds.forEach((taskId) => {
      if (!eventSourcesRef.current[taskId]) {
        const es = new EventSource(getTaskOutputSSEUrl(taskId))

        es.onmessage = (e) => {
          setEventsMap((prev) => {
            const prevEvents = prev[taskId] || []

            return { ...prev, [taskId]: [...prevEvents, e.data] }
          })
        }

        es.onerror = () => {
          es.close()
        }

        eventSourcesRef.current[taskId] = es
      }
    })

    return () => {
      Object.values(eventSourcesRef.current).forEach((es) => es.close())
      eventSourcesRef.current = {}
    }
  }, [taskIds])

  return eventsMap
}
