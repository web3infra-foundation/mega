import { useEffect, useRef, useState } from 'react'
import { useAtom } from 'jotai'

import { loadingAtom, logsAtom, statusAtom } from '../components/Checks/cpns/store'

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

export const useTaskSSE = () => {
  const eventSourcesRef = useRef<Record<string, EventSource>>({})
  const [logsMap, setLogsMap] = useAtom(logsAtom)
  const [_, setLoading] = useAtom(loadingAtom)
  const [_status, setStatus] = useAtom(statusAtom)

  const setEventSource: (taskId: string) => void = (taskId: string) => {
    if (eventSourcesRef.current[taskId]) return
    const es = new EventSource(`/sse/task-output/${taskId}`)
    // const es = new EventSource(`/api/event?id=${taskId}`)

    es.onmessage = (e) => {
      setLogsMap((prev) => {
        const prevLogs = prev[taskId] ?? ''
        const newLog = e.data + '\n'

        // 判断最后一条是否重复
        if (prevLogs.endsWith(newLog)) {
          return prev // 重复就直接返回原对象
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
          [taskId]: result.status // 每条消息生成新数组
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
    // 组件卸载时关闭所有连接
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
