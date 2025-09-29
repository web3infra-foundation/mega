import { useState } from 'react'
import toast from 'react-hot-toast'

import { Button } from '@gitmono/ui/Button'
import { Dialog } from '@gitmono/ui/Dialog'
import { Select, SelectTrigger, SelectValue } from '@gitmono/ui/Select'

import { useCreateEntry } from '@/hooks/useCreateEntry'

import MarkdownEditor from './MarkdownEditor'
import PathInput from './PathInput'

const NewCodeView = () => {
  const [path, setPath] = useState('')
  const [name, setName] = useState('')
  const [fileType, setFileType] = useState<'folder' | 'file'>('file')
  const [dialogOpen, setDialogOpen] = useState(false)
  const [content, setContent] = useState('')
  const createEntryHook = useCreateEntry()

  const handlerSubmit = () => {
    createEntryHook.mutate(
      {
        name: name,
        path: '/' + path.replace('/'+name, ''),
        is_directory: fileType === 'folder',
        content: fileType === 'file' ? content : ''
      },
      {
        onSuccess: () => {
          toast.success('Create Success!')
          setDialogOpen(false)
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
        <MarkdownEditor contentState={[content, setContent]} />
      </div>
    </div>
  )
}

export default NewCodeView
