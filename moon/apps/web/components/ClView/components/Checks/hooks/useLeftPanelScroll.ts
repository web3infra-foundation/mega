import { RefObject, useCallback, useEffect, useLayoutEffect, useRef } from 'react'

export function useLeftPanelScroll(cl: string, buildId: string, leftPanelRef: RefObject<HTMLDivElement | null>) {
  const leftPanelScrollRef = useRef(0)
  const pendingRestoreRef = useRef<number | null>(null)
  const isRestoringRef = useRef(false)
  const prevClRef = useRef(cl)

  useEffect(() => {
    if (prevClRef.current !== cl) {
      prevClRef.current = cl
      leftPanelScrollRef.current = 0
      pendingRestoreRef.current = null
    }
  }, [cl])

  const preserveScroll = useCallback(() => {
    const panel = leftPanelRef.current

    if (panel) {
      pendingRestoreRef.current = panel.scrollTop
    }
  }, [leftPanelRef])

  useEffect(() => {
    const panel = leftPanelRef.current

    if (!panel) return

    const onScroll = () => {
      if (isRestoringRef.current) return

      leftPanelScrollRef.current = panel.scrollTop
    }

    panel.addEventListener('scroll', onScroll, { passive: true })

    return () => panel.removeEventListener('scroll', onScroll)
  }, [leftPanelRef])

  // Restore before paint so a re-render cannot reset scroll to 0 and clobber the
  // saved position via the scroll listener.
  useLayoutEffect(() => {
    const panel = leftPanelRef.current

    if (!panel) return

    const saved = pendingRestoreRef.current ?? leftPanelScrollRef.current

    pendingRestoreRef.current = null

    isRestoringRef.current = true
    panel.scrollTop = saved
    leftPanelScrollRef.current = saved

    requestAnimationFrame(() => {
      isRestoringRef.current = false
    })
  }, [buildId, leftPanelRef])

  return { preserveScroll }
}
