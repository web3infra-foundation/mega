import React, { useState } from 'react';
import { LoadingSpinner, LockIcon, Button, TextField, PlusIcon } from '@gitmono/ui'
import * as Dialog from '@gitmono/ui/src/Dialog'
import { DateAndTimePicker } from "@/components/DateAndTimePicker";
import { useGetGPGList } from '@/hooks/useGetGPGList'
import { usePostGPGKey } from '@/hooks/usePostGPGKey'
import { useDeleteGPGKeyById } from '@/hooks/useDeleteGPGKeyById'
import { legacyApiClient } from "@/utils/queryClient";
import { useQueryClient } from "@tanstack/react-query";
import HandleTime from "@/components/ClView/components/HandleTime";
import { GpgKey } from "@gitmono/types";

const GpgKeyItem = ({ keyData } : { keyData: GpgKey }) => {
  const { mutate: deleteGPGKey } = useDeleteGPGKeyById()
  const fetchGPGList = legacyApiClient.v1.getApiGpgList()
  const queryClient = useQueryClient()

  return (
    <div className="flex items-center justify-between py-4 border-b border-gray-200 last:border-b-0">
      <div className="flex items-start">
        <LockIcon className="w-6 h-6 text-gray-400" aria-hidden="true"/>
        <div className="ml-4">
          <p className="text-base font-bold text-gray-900">{ keyData.fingerprint }</p>
          <p className="text-sm font-mono text-gray-500 mt-1">{ keyData.fingerprint }</p>
          <p className="text-xs text-gray-500 mt-2">
            <HandleTime created_at={ Math.floor(new Date(keyData.created_at).getTime()) }/>
          </p>
        </div>
      </div>
      <button
        onClick={ () => deleteGPGKey(
          {
            data: {
              key_id: keyData.key_id,
            }
          },
          {
            onSuccess: () => {
              queryClient.invalidateQueries({ queryKey: fetchGPGList.requestKey() })
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

interface NewGPGKeyDialogProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

const NewGPGKeyDialog = ({ open, setOpen }: NewGPGKeyDialogProps) => {
  const { mutate: postGPGKey, isPending } = usePostGPGKey()
  // const [title, setTitle] = useState('')
  const [gpg_content, setGpg_content] = useState('')
  const [errors, setErrors] = useState<{ title?: string; gpgKey?: string }>({})
  const [expires_days, setExpiresDays] = useState(new Date())

  const fetchGPGList = legacyApiClient.v1.getApiGpgList()
  const queryClient = useQueryClient()

  const handleSubmit = (e?: React.FormEvent | React.MouseEvent) => {
    if (e) e.preventDefault()
    const nextErrors: { title?: string; gpgKey?: string } = {}

    // if (!title.trim()) nextErrors.title = 'Title is required'
    if (!gpg_content.trim()) nextErrors.gpgKey = 'GPG key is required'
    setErrors(nextErrors)
    if (Object.keys(nextErrors).length > 0) return

    // Normalize Windows-style line endings to Unix-style
    const normalizedGpgContent = gpg_content.replace(/\r\n/g, '\n')

    postGPGKey(
      {
        data: {
          gpg_content: normalizedGpgContent
        }
      },
      {
        onSuccess: () => {
          setOpen(false)
          // setTitle('')
          setGpg_content('')
          setErrors({})

          queryClient.invalidateQueries({ queryKey: fetchGPGList.requestKey() })
        }
      }
    )
  }

  return (
    <Dialog.Root
      open={ open }
      onOpenChange={ setOpen }
      visuallyHiddenDescription='Add a new GPG key'
    >
      <Dialog.Title className="p-4 w-full">
        Add GPG key
      </Dialog.Title>
      <Dialog.Content className="p-4 w-full max-w-md">
        {/*<div className='mb-4'>*/ }
        {/*  <TextField*/ }
        {/*    autoFocus*/ }
        {/*    label='title'*/ }
        {/*    value={title}*/ }
        {/*    onChange={setTitle}*/ }
        {/*  />*/ }
        {/*  {errors.title && <span className='text-red-500 text-xs'>{errors.title}</span>}*/ }
        {/*</div>*/ }

        <div className='flex h-full w-full flex-col gap-3'>
          <TextField
            label='expires_days'
            value={ expires_days.toISOString() }
            disabled
          />
          <div className="justify-center w-full items-center">
            <DateAndTimePicker
              value={ expires_days }
              onChange={ setExpiresDays }
            />
          </div>
        </div>

        <div className='mb-4'>
          <TextField
            placeholder='begins with "-----BEGIN GPG PUBLIC KEY BLOCK-----"'
            multiline
            minRows={ 8 }
            label='gpg_key'
            value={ gpg_content }
            onChange={ setGpg_content }
          />
          { errors.gpgKey && <span className='text-red-500 text-xs'>{ errors.gpgKey }</span> }
        </div>
      </Dialog.Content>

      <Dialog.Footer>
        <Dialog.TrailingActions>
          <Button variant='flat' onClick={ () => {
            setOpen(false)
            setGpg_content("")
            setExpiresDays(new Date())
          } }>
            Cancel
          </Button>
          <Button
            variant='primary'
            className="bg-[#1f883d]"
            onClick={ handleSubmit }
            disabled={ isPending || !gpg_content.trim() }
            loading={ isPending }
          >
            Add key
          </Button>
        </Dialog.TrailingActions>
      </Dialog.Footer>
    </Dialog.Root>
  )
}

const GPGKeys = () => {
  const { gpgKeys, isLoading: isGPGLoading } = useGetGPGList()
  const [open, setOpen] = useState(false)

  return (
    <>
      <div className="bg-white text-gray-700 p-8 rounded-lg border border-gray-200 max-w-4xl mx-auto font-sans">
        <header className="flex items-center justify-between pb-4">
          <h1 className="text-3xl font-bold text-gray-900">GPG keys</h1>
          <Button
            variant='primary'
            className="bg-[#1f883d]"
            leftSlot={ <PlusIcon/> }
            onClick={ () => setOpen(true) }
          >
            New GPG key
          </Button>
        </header>

        <p className="mb-8">
          This is a list of GPG keys associated with your account. Remove any keys that you do not recognize.
        </p>

        <section>
          <h2 className="text-xl font-semibold text-gray-900 pb-2 border-b border-gray-200">
            Authentication keys
          </h2>
          { isGPGLoading? (
            <div className='flex h-[400px] items-center justify-center'>
              <LoadingSpinner/>
            </div>
          ) : (
            <div>
              { gpgKeys?.map((key) => (
                <GpgKeyItem key={ key.key_id } keyData={ key }/>
              )) }
            </div>
          ) }
        </section>
      </div>
      <NewGPGKeyDialog
        open={ open }
        setOpen={ setOpen }
      />
    </>
  );
};

export default GPGKeys;