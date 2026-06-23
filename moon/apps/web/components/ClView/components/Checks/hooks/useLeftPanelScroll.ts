import { RefObject, useEffect, useRef } from 'react'

export function useLeftPanelScroll(cl: string, buildId: string, leftPanelRef: RefObject<HTMLDivElement | null>) {
  const leftPanelScrollRef = useRef(0)
  const prevClRef = useRef(cl)

  useEffect(() => {
    if (prevClRef.current !== cl) {
      prevClRef.current = cl
      leftPanelScrollRef.current = 0
    }
  }, [cl])

  useEffect(() => {
    const panel = leftPanelRef.current

    if (!panel) return

    const onScroll = () => {
      leftPanelScrollRef.current = panel.scrollTop
    }

    panel.addEventListener('scroll', onScroll, { passive: true })

    return () => panel.removeEventListener('scroll', onScroll)
  }, [leftPanelRef])

  useEffect(() => {
    const panel = leftPanelRef.current

    if (!panel) return

    const saved = leftPanelScrollRef.current

    requestAnimationFrame(() => {
      panel.scrollTop = saved
    })
  }, [buildId, leftPanelRef])

  return {}
}
