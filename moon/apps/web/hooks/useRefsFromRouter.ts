import { useRouter } from 'next/router'
import { useCallback, useMemo } from 'react'

export function useRefsFromRouter() {
  const router = useRouter()
  const refs = useMemo(() => {
    const r = router.query.refs

    if (!r) return undefined
    return Array.isArray(r) ? r[0] : r
  }, [router.query.refs])

  const setRefs = useCallback(
    (newRefs?: string) => {
      const { pathname, query } = router
      const nextQuery: Record<string, any> = { ...query }
      
      if (!newRefs) {
        delete nextQuery.refs
      } else {
        nextQuery.refs = newRefs
      }
      router.push({ pathname, query: nextQuery }, undefined, { shallow: false })
    },
    [router]
  )

  return { refs, setRefs }
}
