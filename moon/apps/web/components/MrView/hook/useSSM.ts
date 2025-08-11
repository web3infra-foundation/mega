import { useEffect, useRef, useState } from 'react'

export const useSSM = () => {
  const sseUrl = useRef('')
  const createEventSource = (baseUrl: string): Promise<EventSource> => {
    return new Promise<EventSource>((res, rej) => {
      const es = new EventSource(baseUrl)

      es.onopen = () => {
        res(es)
      }
      es.onerror = () => {
        rej('eventsource建立失败')
      }
    })
  }

  const initial = () => {
    window.location.href.includes('app')
      ? (sseUrl.current = 'https://orion.gitmega.com/logs?follow=true')
      : (sseUrl.current = '/sse/logs?follow=true')
  }

  return {
    createEventSource,
    initial,
    sseUrl
  }
}

export const useTaskSSE = (taskIds: string[]) => {
  const [eventsMap, setEventsMap] = useState<Record<string, string[]>>({})
  const eventSourcesRef = useRef<Record<string, EventSource>>({})

  useEffect(() => {
    // 关闭并清理旧的连接（不在 taskIds 里的）
    Object.keys(eventSourcesRef.current).forEach((id) => {
      if (!taskIds.includes(id)) {
        eventSourcesRef.current[id].close()
        delete eventSourcesRef.current[id]
      }
    })

    // 为新 taskIds 建立连接
    taskIds.forEach((taskId) => {
      if (!eventSourcesRef.current[taskId]) {
        // const es = new EventSource(`/api/tasks/${taskId}/events`)
        const es = new EventSource(`/sse/task-output/${taskId}`)

        es.onmessage = (e) => {
          setEventsMap((prev) => {
            const prevEvents = prev[taskId] || []

            return { ...prev, [taskId]: [...prevEvents, e.data] }
          })
        }

        es.onerror = () => {
          es.close()
          // 这里可以做重连逻辑
        }

        eventSourcesRef.current[taskId] = es
      }
    })

    // 组件卸载时关闭所有连接
    return () => {
      Object.values(eventSourcesRef.current).forEach((es) => es.close())
      eventSourcesRef.current = {}
    }
  }, [taskIds])

  return eventsMap
}
