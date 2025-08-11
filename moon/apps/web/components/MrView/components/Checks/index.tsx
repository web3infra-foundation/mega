import { memo, useEffect, useState } from 'react'
import { LazyLog } from '@melloware/react-logviewer'

import { useGetMrTask } from '@/hooks/SSE/useGetMrTask'

import { useTaskSSE } from '../../hook/useSSM'

enum Status {
  Pending = 'pending',
  Fullfilled = 'fullfilled',
  Rejected = 'rejected'
}

const Checks = ({ mr }: { mr: string }) => {
  // const serverStream = useRef('')
  // const es = useRef<EventSource | null>()
  // const baseUrl = useRef('http://47.79.95.33:3000/logs?follow=true')
  // const status = useRef(Status.Pending)
  const [displayTest, setDisplayText] = useState('')
  // const { createEventSource, initial, sseUrl } = useSSM()
  const { data } = useGetMrTask('VDPUMBPI')
  const [taskids, setTaskids] = useState<string[]>([])

  useEffect(() => {
    if (data) {
      const ids = data.map((i) => i.build_id)

      setTaskids(ids)
    }
  }, [data])

  const eventMap = useTaskSSE(taskids)

  console.log(eventMap)

  // 页面初始化时建立连接
  // useEffect(() => {
  //   initial()
  //   if (status.current !== Status.Fullfilled) {
  //     createEventSource(sseUrl.current)
  //       .then((res) => {
  //         es.current = res
  //         status.current = Status.Fullfilled
  //         es.current.onmessage = (event) => {
  //           serverStream.current += event.data + '\n'
  //           setDisplayText(serverStream.current)
  //         }
  //       })
  //       .catch(() => (status.current = Status.Rejected))
  //   }

  //   return () => {
  //     // 关闭连接
  //     status.current = Status.Pending
  //     es.current?.close()
  //     es.current = null
  //     serverStream.current = ''
  //     setDisplayText('')
  //   }
  //   // eslint-disable-next-line react-hooks/exhaustive-deps
  // }, [])

  return (
    <>
      <div style={{ height: `calc(100vh - 104px)` }}>
        {displayTest && <LazyLog extraLines={1} text={displayTest} stream enableSearch caseInsensitive follow />}
      </div>
    </>
  )
}

export default memo(Checks)
