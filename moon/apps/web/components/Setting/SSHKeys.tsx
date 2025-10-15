import React, {useState} from 'react';
import {LoadingSpinner, LockIcon, Button, TextField, PlusIcon} from '@gitmono/ui'
import * as Dialog from '@gitmono/ui/src/Dialog'
import {ListSSHKey} from "@gitmono/types";
import {useGetSSHList} from '@/hooks/useGetSSHList'
import {usePostSSHKey} from '@/hooks/usePostSSHKey'
import {useDeleteSSHKeyById} from '@/hooks/useDeleteSSHKeyById'
import {legacyApiClient} from "@/utils/queryClient";
import {useQueryClient} from "@tanstack/react-query";
import HandleTime from "@/components/ClView/components/HandleTime";

const SshKeyItem = ({keyData}: { keyData: ListSSHKey }) => {
  const {mutate: deleteSSHKey} = useDeleteSSHKeyById()
  const fetchSSHList = legacyApiClient.v1.getApiUserSshList()
  const queryClient = useQueryClient()

  return (
    <div className="flex items-center justify-between py-4 border-b border-gray-200 last:border-b-0">
      <div className="flex items-start">
        <LockIcon className="w-6 h-6 text-gray-400" aria-hidden="true"/>
        <div className="ml-4">
          <p className="text-base font-bold text-gray-900">{keyData.title}</p>
          <p className="text-sm font-mono text-gray-500 mt-1">{keyData.finger}</p>
          <p className="text-xs text-gray-500 mt-2">
            <HandleTime created_at={keyData.created_at}/>
          </p>
        </div>
      </div>
      <button
        onClick={() => deleteSSHKey(
          {keyId: keyData.id},
          {
            onSuccess: () => {
              queryClient.invalidateQueries({queryKey: fetchSSHList.requestKey()})
            }
          })
        }
        className="px-4 py-1 text-sm font-semibold text-red-500 border border-gray-300 rounded-md hover:bg-red-500 hover:text-white transition-colors duration-200"
      >
        Delete
      </button>
    </div>
  )
}

interface NewSSHKeyDialogProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

const NewSSHKeyDialog = ({open, setOpen}: NewSSHKeyDialogProps) => {
  const {mutate: postSSHKey, isPending} = usePostSSHKey()
  const [title, setTitle] = useState('')
  const [sshKey, setSshKey] = useState('')
  const [errors, setErrors] = useState<{ title?: string; sshKey?: string }>({})

  const fetchSSHList = legacyApiClient.v1.getApiUserSshList()
  const queryClient = useQueryClient()

  const handleSubmit = (e?: React.FormEvent | React.MouseEvent) => {
    if (e) e.preventDefault()
    const nextErrors: { title?: string; sshKey?: string } = {}

    if (!title.trim()) nextErrors.title = 'Title is required'
    if (!sshKey.trim()) nextErrors.sshKey = 'SSH key is required'
    setErrors(nextErrors)
    if (Object.keys(nextErrors).length > 0) return
    postSSHKey(
      {data: {title: title.trim(), ssh_key: sshKey}},
      {
        onSuccess: () => {
          setOpen(false)
          setTitle('')
          setSshKey('')
          setErrors({})

          queryClient.invalidateQueries({queryKey: fetchSSHList.requestKey()})
        }
      }
    )
  }

  return (
    <Dialog.Root
      open={open}
      onOpenChange={setOpen}
      visuallyHiddenDescription='Add a new SSH key'
    >
      <Dialog.Title className="p-4 w-full">
        Add SSH key
      </Dialog.Title>
      <Dialog.Content className="p-4 w-full max-w-md">
        <div className='mb-4'>
          <TextField
            autoFocus
            label='title'
            value={title}
            onChange={setTitle}
          />
          {errors.title && <span className='text-red-500 text-xs'>{errors.title}</span>}
        </div>

        <div className='mb-4'>
          <TextField
            placeholder='begins with "ssh-rsa" or "ssh-ed25519"'
            multiline
            minRows={5}
            label='ssh_key'
            value={sshKey}
            onChange={setSshKey}
          />
          {errors.sshKey && <span className='text-red-500 text-xs'>{errors.sshKey}</span>}
        </div>
      </Dialog.Content>

      <Dialog.Footer>
        <Dialog.TrailingActions>
          <Button variant='flat' onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            variant='primary'
            className="bg-[#1f883d]"
            onClick={handleSubmit}
            disabled={isPending || !title.trim() || !sshKey.trim()}
            loading={isPending}
          >
            Add key
          </Button>
        </Dialog.TrailingActions>
      </Dialog.Footer>
    </Dialog.Root>
  )
}

const SSHKeys = () => {
  const {sshKeys, isLoading} = useGetSSHList()
  const [open, setOpen] = useState(false)

  return (
    <>
      <div className="bg-white text-gray-700 p-8 rounded-lg border border-gray-200 max-w-4xl mx-auto font-sans">
        <header className="flex items-center justify-between pb-4">
          <h1 className="text-3xl font-bold text-gray-900">SSH keys</h1>
          <Button
            variant='primary'
            className="bg-[#1f883d]"
            leftSlot={<PlusIcon/>}
            onClick={() => setOpen(true)}
          >
            New SSH key
          </Button>
        </header>

        <p className="mb-8">
          This is a list of SSH keys associated with your account. Remove any keys that you do not recognize.
        </p>

        <section>
          <h2 className="text-xl font-semibold text-gray-900 pb-2 border-b border-gray-200">
            Authentication keys
          </h2>
          {isLoading ? (
            <div className='flex h-[400px] items-center justify-center'>
              <LoadingSpinner/>
            </div>
          ) : (
            <div>
              {sshKeys?.map((key) => (
                <SshKeyItem key={key.id} keyData={key}/>
              ))}
            </div>
          )}
        </section>
      </div>
      <NewSSHKeyDialog
        open={open}
        setOpen={setOpen}
      />
    </>
  );
};

export default SSHKeys;