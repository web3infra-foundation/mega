'use client'

import { useEffect, useState } from 'react'
import { useTheme } from 'next-themes'
import Markdown from 'react-markdown'

import styles from './ThemedMarkdown.module.css'

interface ThemedMarkdownProps {
  children: string
  className?: string
  style?: React.CSSProperties
}

const ThemedMarkdown = ({ children, className = '', style }: ThemedMarkdownProps) => {
  const { theme, resolvedTheme } = useTheme()
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  const currentTheme = mounted ? (theme === 'system' ? resolvedTheme : theme) || 'light' : 'light'

  return (
    <div className={`${styles.markdownWrapper} ${className}`} data-theme={currentTheme} style={style}>
      <div className={styles.markdownBody}>
        <Markdown>{children}</Markdown>
      </div>
    </div>
  )
}

export default ThemedMarkdown
