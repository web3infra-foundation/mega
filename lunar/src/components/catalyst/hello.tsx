'use client'
// show helo world
import React, { useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

export default function HelloRust({ ...props }) {
  const [content, setContent] = React.useState<string>('数据加载中...')
  useEffect(() => {
    // sleep 1s, to see the loading effect
    setTimeout(() => {
      invoke('hello_string', { name: 'lunar' }).then((response: string) => {
        setContent(response)
      })
    }, 1000)
  }, [])
  return <div style={{ textAlign: 'center' }}>{content}</div>
}
