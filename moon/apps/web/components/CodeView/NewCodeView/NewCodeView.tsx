import { useState } from 'react'
import { useRouter } from 'next/router'
import { useQueryClient } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import toast from 'react-hot-toast'

import { Button } from '@gitmono/ui/Button'
import { Dialog } from '@gitmono/ui/Dialog'
import { Select, SelectTrigger, SelectValue } from '@gitmono/ui/Select'

import { useCreateEntry } from '@/hooks/useCreateEntry'
import { useScope } from '@/contexts/scope'
import { expandedNodesAtom } from '@/components/CodeView/TreeView/codeTreeAtom'
import { legacyApiClient } from '@/utils/queryClient'

import MarkdownEditor from './MarkdownEditor'
import PathInput from './PathInput'

interface NewCodeViewProps {
  currentPath?: string
  onClose?: () => void
}

const NewCodeView = ({ currentPath = '', onClose }: NewCodeViewProps) => {
  const router = useRouter()
  const { scope } = useScope()
  const queryClient = useQueryClient()
  const [expandedNodes, setExpandedNodes] = useAtom(expandedNodesAtom)
  const [path, setPath] = useState(currentPath)
  const [name, setName] = useState('')
  const [fileType, setFileType] = useState<'folder' | 'file'>('file')
  const [dialogOpen, setDialogOpen] = useState(false)
  const [content, setContent] = useState('')
  const createEntryHook = useCreateEntry()

  const handlerSubmit = () => {
    const entryPath = '/' + path.replace('/'+name, '')
    
    createEntryHook.mutate(
      {
        name: name,
        path: entryPath,
        is_directory: fileType === 'folder',
        content: fileType === 'file' ? content : ''
      },
      {
        onSuccess: async () => {
          toast.success('Create Success!')
          setDialogOpen(false)
          
          const fullPath = entryPath === '/' ? `/${name}` : `${entryPath}/${name}`
          
          if (fileType === 'folder') {
            const pathParts = fullPath.split('/').filter(Boolean)

            const pathsToExpand = ['/', ...pathParts.map((_, i) => '/' + pathParts.slice(0, i + 1).join('/'))]

            setExpandedNodes(Array.from(new Set([...expandedNodes, ...pathsToExpand])))
          }
          
          await Promise.all([
            queryClient.refetchQueries({
              queryKey: legacyApiClient.v1.getApiTree().requestKey({ path: entryPath })
            }),
            queryClient.refetchQueries({
              queryKey: legacyApiClient.v1.getApiTreeCommitInfo().requestKey({ path: entryPath })
            }),
            ...(fileType === 'folder' ? [
              queryClient.refetchQueries({
                queryKey: legacyApiClient.v1.getApiTree().requestKey({ path: fullPath })
              }),
              queryClient.refetchQueries({
                queryKey: legacyApiClient.v1.getApiTreeCommitInfo().requestKey({ path: fullPath })
              })
            ] : [])
          ])
          
          if (fileType === 'file') {
            router.push(`/${scope}/code/blob${fullPath}`)
          } else {
            router.push(`/${scope}/code/tree${fullPath}`)
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

  return (
    <div className='flex h-full w-full flex-col gap-2'>
      <Dialog.Root open={dialogOpen} onOpenChange={setDialogOpen}>
        <Dialog.Content>
          <Dialog.Header>
            <Dialog.Title>Create folder</Dialog.Title>
          </Dialog.Header>
          <Dialog.Content>
            Creating a folder will clear the current content in the editor, and this action cannot be undone. Do you
            want to continue?
          </Dialog.Content>
          <Dialog.Footer>
            <Dialog.TrailingActions>
              <Button variant='flat' onClick={() => setDialogOpen(false)}>
                Cancel
              </Button>
              <Button onClick={handlerSubmit}>Create</Button>
            </Dialog.TrailingActions>
          </Dialog.Footer>
        </Dialog.Content>
      </Dialog.Root>
      <div className='flex min-h-14 w-full items-center justify-between pl-2 pr-4'>
        <PathInput pathState={[path, setPath]} nameState={[name, setName]} />
        <div className='flex gap-2'>
          <Button
            disabled={name === ''}
            onClick={() => {
              if (fileType === 'folder') {
                setDialogOpen(true)
              } else {
                handlerSubmit()
              }
            }}
          >
            Create
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

      <div className='w-full flex-1 overflow-y-auto'>
        <MarkdownEditor contentState={[content, setContent]} disabled={fileType === 'folder'} />
      </div>
    </div>
  )
}
  
export default NewCodeView
