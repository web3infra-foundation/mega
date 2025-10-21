import React, { useCallback, useEffect, useMemo, useState } from 'react'

import { Button, UIText } from '@gitmono/ui'
import { Dialog } from '@gitmono/ui/Dialog'

import { usePostRepoClone } from '@/hooks/usePostRepoClone'

interface Props {
  currentPath?: string
}

const formatBasePath = (path?: string): string => {
  if (!path || path === '/') return '/'
  return path.endsWith('/') ? path : `${path}/`
}

const isFieldValid = (value: string): boolean => value.trim().length > 0

export default function SyncRepoButton({ currentPath }: Props) {
  const [open, setOpen] = useState(false)
  const [owner, setOwner] = useState('')
  const [repo, setRepo] = useState('')
  const [repoName, setRepoName] = useState('')

  const { mutateAsync, isPending } = usePostRepoClone()

  const basePath = useMemo(() => formatBasePath(currentPath), [currentPath])

  const fullPath = useMemo(() => (repoName ? `${basePath}${repoName}` : basePath), [basePath, repoName])

  const canSubmit = useMemo(
    () => isFieldValid(owner) && isFieldValid(repo) && isFieldValid(repoName) && !isPending,
    [owner, repo, repoName, isPending]
  )

  const resetForm = useCallback(() => {
    setOwner('')
    setRepo('')
    setRepoName('')
  }, [])

  const handleOpenChange = useCallback(
    (isOpen: boolean) => {
      setOpen(isOpen)
      if (!isOpen) {
        resetForm()
      }
    },
    [resetForm]
  )

  const handleSync = useCallback(async () => {
    await mutateAsync({
      owner: owner.trim(),
      repo: repo.trim(),
      path: fullPath.trim()
    })

    handleOpenChange(false)
  }, [owner, repo, fullPath, mutateAsync, handleOpenChange])

  useEffect(() => {
    setRepoName(repo)
  }, [repo])

  return (
    <>
      <Button variant='base' onClick={() => setOpen(true)}>
        Sync Repository
      </Button>

      <Dialog.Root open={open} onOpenChange={handleOpenChange}>
        <Dialog.Content>
          <Dialog.CloseButton />
          <Dialog.Header>
            <Dialog.Title>Sync Repository</Dialog.Title>
            <Dialog.Description>{/* Sync a repository from GitHub to your server */}</Dialog.Description>
          </Dialog.Header>

          <div className='flex flex-col gap-4 py-2'>
            <div className='flex flex-col gap-2'>
              <label className='text-quaternary text-sm'>
                GitHub Owner <span className='text-red-500'>*</span>
              </label>
              <input
                className='rounded-md border px-3 py-2 text-sm outline-none focus:ring-2'
                value={owner}
                onChange={(e) => setOwner(e.target.value)}
                placeholder='e.g., facebook'
                disabled={isPending}
              />
            </div>

            <div className='flex flex-col gap-2'>
              <label className='text-quaternary text-sm'>
                Repository Name <span className='text-red-500'>*</span>
              </label>
              <input
                className='rounded-md border px-3 py-2 text-sm outline-none focus:ring-2'
                value={repo}
                onChange={(e) => setRepo(e.target.value)}
                placeholder='e.g., react'
                disabled={isPending}
              />
            </div>

            <div className='flex flex-col gap-2'>
              <label className='text-quaternary text-sm'>
                Target Path <span className='text-red-500'>*</span>
              </label>
              <div className='flex items-center gap-0 overflow-hidden rounded-md border focus-within:ring-2'>
                <span className='border-r bg-gray-100 px-3 py-2 text-sm text-gray-600'>{basePath}</span>
                <input
                  className='flex-1 border-none px-3 py-2 text-sm outline-none'
                  value={repoName}
                  onChange={(e) => setRepoName(e.target.value)}
                  placeholder='repo-name'
                  disabled={isPending}
                />
              </div>
            </div>

            {owner && repo && (
              <div className='rounded-md bg-gray-50 p-3'>
                <UIText quaternary size='text-xs' className='mb-1'>
                  Will sync from:
                </UIText>
                <UIText size='text-sm' className='font-mono'>
                  https://github.com/{owner}/{repo}
                </UIText>
                <UIText quaternary size='text-xs' className='mt-2'>
                  To: {fullPath}
                </UIText>
              </div>
            )}
          </div>

          <Dialog.Footer>
            <Dialog.TrailingActions>
              <Button variant='plain' onClick={() => handleOpenChange(false)} disabled={isPending}>
                Cancel
              </Button>

              <Button disabled={!canSubmit} onClick={handleSync} loading={isPending}>
                {isPending ? 'Syncing...' : 'Start Sync'}
              </Button>
            </Dialog.TrailingActions>
          </Dialog.Footer>
        </Dialog.Content>
      </Dialog.Root>
    </>
  )
}
