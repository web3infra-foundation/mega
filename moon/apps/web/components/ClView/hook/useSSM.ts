import { useEffect, useRef, useState } from 'react'
import { useAtom } from 'jotai'

import { orionApiClient } from '@/utils/queryClient'

import { loadingAtom, logsAtom, statusAtom } from '../components/Checks/cpns/store'

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

export const useTaskSSE = () => {
  const eventSourcesRef = useRef<Record<string, EventSource>>({})
  const [logsMap, setLogsMap] = useAtom(logsAtom)
  const [_, setLoading] = useAtom(loadingAtom)
  const [_status, setStatus] = useAtom(statusAtom)

  const setEventSource: (taskId: string) => void = (taskId: string) => {
    if (eventSourcesRef.current[taskId]) return

    const es = new EventSource(getTaskOutputSSEUrl(taskId))

    es.onmessage = (e) => {
      setLogsMap((prev) => {
        const prevLogs = prev[taskId] ?? ''
        const newLog = e.data + '\n'

        if (prevLogs.endsWith(newLog)) {
          return prev
        }

        return {
          ...prev,
          [taskId]: prevLogs + newLog
        }
      })

      setLoading(false)
    }

    // status
    es.addEventListener('buildResult', (e) => {
      const result = JSON.parse(e.data)

      setStatus((prev) => {
        return {
          ...prev,
          [taskId]: result.status
        }
      })
      es.close()
    })

    es.onerror = () => {
      es.close()
    }

    eventSourcesRef.current[taskId] = es
  }

  useEffect(() => {
    return () => {
      Object.values(eventSourcesRef.current).forEach((es) => es.close())
      eventSourcesRef.current = {}
      setLoading(false)
      setLogsMap({})
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return { eventSourcesRef, setEventSource, logsMap, setLogsMap }
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
