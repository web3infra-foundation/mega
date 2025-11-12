// components/Labels/NewLabelDialog.tsx
import React, { useState } from 'react'
import { random } from 'colord'

import { Button, Dialog, RefreshIcon, TextField } from '@gitmono/ui'

import { getFontColor } from '@/utils/getFontColor'

interface NewLabelDialogProps {
  isOpen: boolean
  onClose: () => void
  onCreateLabel: (name: string, description: string, color: string) => void
}

export const NewLabelDialog: React.FC<NewLabelDialogProps> = ({ isOpen, onClose, onCreateLabel }) => {
  const [color, setColor] = useState(random().toHex())
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')

  const fontColor = getFontColor(color)

  const generateRandomColor = () => {
    setColor(random().toHex())
  }

  const handleCreateLabel = () => {
    if (name.trim()) {
      onCreateLabel(name, description, color)
      setName('')
      setDescription('')
      generateRandomColor()
      onClose()
    }
  }

  return (
    <Dialog.Root open={isOpen} onOpenChange={onClose}>
      <Dialog.Title className='w-full p-4'>New Label</Dialog.Title>
      <Dialog.Content>
        <div className='w-full max-w-md p-4'>
          {/* label preview */}
          <div className='mb-4 flex items-center justify-center'>
            <div
              style={{
                backgroundColor: color,
                color: fontColor.toHex(),
                borderRadius: '16px',
                padding: '2px 8px',
                fontSize: '12px',
                fontWeight: '600',
                display: 'inline-block',
                textAlign: 'center'
              }}
            >
              {name || 'label preview'}
            </div>
          </div>

          <div className='mb-4'>
            <TextField label='Name' value={name} onChange={(e) => setName(e)} placeholder='Label name' />
          </div>

          <div className='mb-4'>
            <TextField
              label='Description'
              value={description}
              onChange={(e) => setDescription(e)}
              placeholder='Optionally add a description.'
            />
          </div>

          <div className='mb-6'>
            <label className='mb-1 block text-sm font-medium text-gray-700'>Color</label>
            <div className='flex items-center gap-2'>
              <Button size='sm' onClick={generateRandomColor} className={`flex-shrink-0 bg-[${color}]`}>
                <RefreshIcon className='h-4 w-4' />
              </Button>
              <div className='flex flex-grow items-center gap-2 rounded-md border px-2 py-1'>
                <input
                  className='flex-grow border-none bg-transparent p-0 text-sm outline-none ring-0 focus:ring-0'
                  value={color}
                  onChange={(e) => setColor(e.target.value)}
                />
              </div>
            </div>
          </div>

          <div className='flex justify-end gap-2'>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant='primary' className='bg-[#1f883d]' onClick={handleCreateLabel} disabled={!name.trim()}>
              Create label
            </Button>
          </div>
        </div>
      </Dialog.Content>
    </Dialog.Root>
  )
}
