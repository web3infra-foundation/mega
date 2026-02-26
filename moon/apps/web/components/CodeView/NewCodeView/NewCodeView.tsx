import { useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { Button } from '@gitmono/ui/Button'
import { Dialog } from '@gitmono/ui/Dialog'
import { Select, SelectTrigger, SelectValue } from '@gitmono/ui/Select'

import { expandedNodesAtom } from '@/components/CodeView/TreeView/codeTreeAtom'
import { useScope } from '@/contexts/scope'
import { useCreateEntry } from '@/hooks/useCreateEntry'
import { useGetCurrentUser } from '@/hooks/useGetCurrentUser'
import { legacyApiClient } from '@/utils/queryClient'

import MarkdownEditor from './MarkdownEditor'
import PathInput from './PathInput'

interface NewCodeViewProps {
  currentPath?: string
  onClose?: () => void
  defaultType?: 'folder' | 'file'
  version?: string
}

const NewCodeView = ({ currentPath = '', onClose, defaultType = 'file', version }: NewCodeViewProps) => {
  const router = useRouter()
  const { scope } = useScope()
  const queryClient = useQueryClient()
  const [expandedNodes, setExpandedNodes] = useAtom(expandedNodesAtom)
  const [path, setPath] = useState(currentPath)
  const [name, setName] = useState('')

  const [skipBuild, setSkipBuild] = useState(false)

  const [fileType, setFileType] = useState<'folder' | 'file'>(defaultType)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [content, setContent] = useState('')
  const createEntryHook = useCreateEntry()
  const { data: currentUser } = useGetCurrentUser()

  const handleSubmit = () => {
    createEntryHook.mutate(
      {
        name: name,
        path: currentPath,
        is_directory: fileType === 'folder',
        content: fileType === 'file' ? content : '',
        author_email: currentUser?.email,
        author_username: currentUser?.username,
        mode: 'force_create',
        skip_build: skipBuild
      },
      {
        onSuccess: async () => {
          toast.success('Create Success!')
          setDialogOpen(false)

          if (fileType === 'folder') {
            const pathParts = path.split('/').filter(Boolean)

            const pathsToExpand = ['/', ...pathParts.map((_, i) => '/' + pathParts.slice(0, i + 1).join('/'))]

            setExpandedNodes(Array.from(new Set([...expandedNodes, ...pathsToExpand])))
          }

          await Promise.all([
            queryClient.refetchQueries({
              queryKey: legacyApiClient.v1.getApiTree().requestKey({ path: path })
            }),
            queryClient.refetchQueries({
              queryKey: legacyApiClient.v1.getApiTreeCommitInfo().requestKey({ path: path })
            }),
            ...(fileType === 'folder'
              ? [
                  queryClient.refetchQueries({
                    queryKey: legacyApiClient.v1.getApiTree().requestKey({ path: path })
                  }),
                  queryClient.refetchQueries({
                    queryKey: legacyApiClient.v1.getApiTreeCommitInfo().requestKey({ path: path })
                  })
                ]
              : [])
          ])

          if (fileType === 'file') {
            router.push(`/${scope}/code/blob/${version}${path}`)
          } else {
            router.push(`/${scope}/code/tree/${version}${path}`)
          }

          onClose?.()
        },
        onError: (error: any) => {
          // Try to read a useful message from the error object
          const msg =
            error?.message ||
            (error?.response && error.response.data && error.response.data.message) ||
            'Create failed. Please try again.'

          toast.error(msg)
        }
      }
    )
  }

  const handleCommitClick = () => {
    setDialogOpen(true)
    setSkipBuild(false)
  }

  const handleDialogClose = (open: boolean) => {
    setDialogOpen(open)
    if (!open) {
      setSkipBuild(false)
    }
  }

  return (
    <div className='flex h-full w-full flex-col gap-2'>
      <div className='flex min-h-14 w-full items-center justify-between pl-2 pr-4'>
        <PathInput pathState={[path, setPath]} nameState={[name, setName]} />
        <div className='flex gap-2'>
          <Button disabled={name === ''} onClick={handleCommitClick}>
            Create CL
          </Button>
          <Select
            typeAhead
            options={[
              { value: 'folder', label: 'Folder' },
              { value: 'file', label: 'File' }
            ]}
            value={fileType}
            onChange={(value) => {
              setFileType(value as 'folder' | 'file')
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder='Select Create Type' />
            </SelectTrigger>
          </Select>
        </div>
      </div>

      {/*The second parameter of MarkdownEditor is to disable the editor, which is currently hidden directly.  */}
      {fileType === 'file' && (
        <div className='w-full flex-1 overflow-y-auto'>
          {/*<MarkdownEditor contentState={[content, setContent]} disabled={fileType === 'folder'} />*/}
          <MarkdownEditor contentState={[content, setContent]} disabled={false} />
        </div>
      )}

      <Dialog.Root open={dialogOpen} onOpenChange={handleDialogClose}>
        <Dialog.Content>
          <Dialog.CloseButton />
          <Dialog.Header>
            <Dialog.Title>Create {fileType === 'folder' ? 'Folder' : 'File'}</Dialog.Title>
          </Dialog.Header>

          <div className='flex flex-col gap-4 py-4'>
            <div className='flex items-center gap-2'>
              <input
                type='checkbox'
                id='skipBuild_creat'
                checked={skipBuild}
                onChange={(e) => setSkipBuild(e.target.checked)}
                className='h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500'
                disabled={createEntryHook.isPending}
              />
              <label htmlFor='skipBuild' className='text-sm font-medium text-gray-700'>
                Skip automatic build after commit
              </label>
            </div>
          </div>

          <Dialog.Footer>
            <Dialog.TrailingActions>
              <Button variant='flat' onClick={() => handleDialogClose(false)} disabled={createEntryHook.isPending}>
                Cancel
              </Button>
              <Button onClick={handleSubmit} disabled={createEntryHook.isPending}>
                {createEntryHook.isPending ? 'Creating...' : 'Confirm'}
              </Button>
            </Dialog.TrailingActions>
          </Dialog.Footer>
        </Dialog.Content>
      </Dialog.Root>
    </div>
  )
}

export default NewCodeView
