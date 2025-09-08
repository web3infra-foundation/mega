import { useQuery } from '@tanstack/react-query'

export interface CommitBindingData {
  commit_sha: string
  author_email: string
  user?: {
    id: string
    username: string
    display_name?: string
    avatar_url?: string
    email: string
  }
  is_anonymous: boolean
  display_name: string
  avatar_url?: string
  is_verified_user: boolean
}

export function useGetCommitBinding(sha: string | undefined) {
  return useQuery({
    queryKey: ['commit-binding', sha],
    queryFn: async (): Promise<CommitBindingData> => {
      const baseUrl = process.env.NEXT_PUBLIC_MONO_API_URL || process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'
      const url = `${baseUrl}/api/v1/mega/commits/${sha}`
      
      try {
        const response = await fetch(url, {
          headers: {
            'Content-Type': 'application/json'
          },
          credentials: 'include'
        })
        
        if (!response.ok) {
          if (response.status === 404) {
            return {
              commit_sha: sha || 'unknown',
              author_email: 'unknown',
              is_anonymous: true,
              display_name: '匿名提交',
              is_verified_user: false
            }
          }
          throw new Error(`Failed to fetch: ${response.status}`)
        }
        
        const result = await response.json()
        const data = result.data
        
        return {
          commit_sha: data.binding?.commit_sha || sha || 'unknown',
          author_email: data.binding?.author_email || 'unknown',
          user: data.binding?.user,
          is_anonymous: data.binding?.is_anonymous ?? true,
          display_name: data.display_name,
          avatar_url: data.avatar_url,
          is_verified_user: data.is_verified_user
        }
      } catch (error) {
        if (typeof window !== 'undefined') {
          // eslint-disable-next-line no-console
          console.error('Commit binding fetch error:', error, 'URL:', url)
        }
        
        return {
          commit_sha: sha || 'unknown',
          author_email: 'unknown',
          is_anonymous: true,
          display_name: '匿名提交',
          is_verified_user: false
        }
      }
    },
    enabled: !!sha && sha.length >= 3,
    staleTime: 5 * 60 * 1000,
    retry: 1,
    refetchOnWindowFocus: false
  })
}
