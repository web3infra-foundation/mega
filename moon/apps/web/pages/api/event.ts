import { NextApiRequest, NextApiResponse } from 'next'

export const config = {
  api: {
    bodyParser: false,
    externalResolver: true
  }
}

export default function handler(req: NextApiRequest, res: NextApiResponse) {
  const { id } = req.query

  if (!id || typeof id !== 'string') {
    res.status(400).json({ error: 'Missing id parameter' })
    return
  }

  res.setHeader('Content-Type', 'text/event-stream')
  res.setHeader('Cache-Control', 'no-cache, no-transform')
  res.setHeader('Connection', 'keep-alive')
  res.flushHeaders?.()

  // eslint-disable-next-line no-console
  console.log(`New SSE connection for id=${id}`)

  let count = 0
  const sendData = () => {
    count++
    const payload = { id, count, timestamp: Date.now() }

    res.write(`data: ${JSON.stringify(payload)}\n\n`)
    // res.flush?.() // 强制立刻推送
  }

  sendData() // 立即发第一条

  const interval = setInterval(() => {
    if (count >= 10) {
      clearInterval(interval)
      const buildSuccess = Math.random() > 0.5 // 模拟 50% 成功率

      // 发送构建结果的事件
      res.write(
        `event: buildResult\ndata: ${JSON.stringify({
          id,
          status: buildSuccess ? 'success' : 'fail',
          finishedAt: Date.now()
        })}\n\n`
      )

      res.write(`event: close\ndata: "done"\n\n`)
      res.end()
      return
    }
    sendData()
  }, 1000)

  req.on('close', () => {
    // console.log(`SSE connection closed for id=${id}`)
    clearInterval(interval)
  })
}
