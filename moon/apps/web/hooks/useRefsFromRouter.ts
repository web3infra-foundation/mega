import { useCallback } from 'react'
import { useRouter } from 'next/router'

// The file can be deleted.
export function useRefsFromRouter() {
  const router = useRouter()
  const refs = router.query.version as string

  const setRefs = useCallback(
    (newRefs?: string) => {
      const { query } = router
      const org = query.org

      let pathArray: string[] = []

      if (query.path) {
        pathArray = Array.isArray(query.path) ? query.path : [query.path]
      }
      const currentPath = pathArray.join('/')

      if (!newRefs) {
        router.push(`/${org}/code/tree/main/${currentPath}`)
      } else {
        router.push(`/${org}/code/tree/${encodeURIComponent(newRefs)}/${currentPath}`)
      }
    },
    [router]
  )

  return { refs, setRefs }
}
