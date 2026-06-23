import { useCallback, useEffect, useRef, useState } from 'react'

const MIN_LEFT_WIDTH = 200
const MAX_LEFT_WIDTH_PERCENT = 0.7
const DEFAULT_LEFT_WIDTH_PERCENT = 0.2

export function useResizablePanels() {
  const containerRef = useRef<HTMLDivElement>(null)
  const leftPanelRef = useRef<HTMLDivElement>(null)
  const rightPanelRef = useRef<HTMLDivElement>(null)
  const logContainerRef = useRef<HTMLDivElement>(null)
  const scrollPositionRef = useRef(0)
  const startWidthRef = useRef(0)

  const [leftWidth, setLeftWidth] = useState<number | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [logViewerHeight, setLogViewerHeight] = useState(0)

  useEffect(() => {
    if (containerRef.current && leftWidth === null) {
      setLeftWidth(containerRef.current.offsetWidth * DEFAULT_LEFT_WIDTH_PERCENT)
    }
  }, [leftWidth])

  useEffect(() => {
    const el = containerRef.current

    if (!el) return

    const update = () => setLogViewerHeight(el.clientHeight)

    update()

    const observer = new ResizeObserver(update)

    observer.observe(el)

    return () => observer.disconnect()
  }, [])

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!containerRef.current || !leftPanelRef.current) return

    const containerRect = containerRef.current.getBoundingClientRect()
    const newLeftWidth = e.clientX - containerRect.left
    const maxWidth = containerRect.width * MAX_LEFT_WIDTH_PERCENT
    const clampedWidth = Math.max(MIN_LEFT_WIDTH, Math.min(newLeftWidth, maxWidth))

    leftPanelRef.current.style.width = `${clampedWidth}px`
  }, [])

  const handleMouseUp = useCallback(() => {
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', handleMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''

    if (rightPanelRef.current) {
      rightPanelRef.current.style.display = 'block'
    }

    if (leftPanelRef.current) {
      setLeftWidth(leftPanelRef.current.offsetWidth)
    }

    setIsDragging(false)
  }, [handleMouseMove])

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()

      if (rightPanelRef.current) {
        rightPanelRef.current.style.display = 'none'
      }

      if (logContainerRef.current) {
        const scrollEl = logContainerRef.current.querySelector('.log-viewer-scroll')

        if (scrollEl) {
          scrollPositionRef.current = scrollEl.scrollTop
        }
      }

      if (leftPanelRef.current) {
        startWidthRef.current = leftPanelRef.current.offsetWidth
      }

      requestAnimationFrame(() => {
        setIsDragging(true)
      })

      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = 'col-resize'
      document.body.style.userSelect = 'none'
    },
    [handleMouseMove, handleMouseUp]
  )

  useEffect(() => {
    if (!isDragging && logContainerRef.current && scrollPositionRef.current > 0) {
      requestAnimationFrame(() => {
        const scrollEl = logContainerRef.current?.querySelector('.log-viewer-scroll')

        if (scrollEl) {
          scrollEl.scrollTop = scrollPositionRef.current
        }
      })
    }
  }, [isDragging])

  return {
    containerRef,
    leftPanelRef,
    rightPanelRef,
    logContainerRef,
    leftWidth,
    isDragging,
    logViewerHeight,
    handleMouseDown,
    defaultLeftWidthPercent: DEFAULT_LEFT_WIDTH_PERCENT
  }
}
