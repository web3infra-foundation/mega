import { useGetCurrentUser } from '@/hooks/useGetCurrentUser'

import { useGetUserPermissions } from './useGetUserPermissions'

interface UseNotePermissionOptions {
  noteId: string
  enabled?: boolean
}

export function useNotePermission({ noteId, enabled = true }: UseNotePermissionOptions) {
  const { data: currentUser } = useGetCurrentUser()
  const username = currentUser?.username || ''

  const { data: permissionData, isLoading } = useGetUserPermissions(username, 'note', noteId, {
    enabled: enabled && !!username && !!noteId
  })

  const permission = permissionData?.data

  return {
    isAdmin: permission?.is_admin || false,
    hasRead: permission?.has_read || false,
    hasWrite: permission?.has_write || false,
    hasAdmin: permission?.has_admin || false,
    permission: permission?.permission || null,
    isLoading
  }
}
