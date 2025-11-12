import React from 'react'

import { Button, CheckIcon, ChevronDownIcon } from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { MenuItem } from '@gitmono/ui/Menu'

interface ReviewDropdownProps {
  options: string[]
  value: string
  onChange: (review: string) => void
  onClose?: (review: string) => void
}

export function ReviewDropdown({ options, value, onChange, onClose }: ReviewDropdownProps) {
  const items: MenuItem[] = options.map((option) => ({
    type: 'item' as const,
    label: (
      <div className='flex items-center gap-2'>
        <div className='h-4 w-4'>{value === option && <CheckIcon />}</div>
        <span className='flex-1'>{option}</span>
      </div>
    ),
    className: 'overflow-hidden',
    onSelect: (e: Event) => {
      e.preventDefault()
      if (option === value) {
        onChange('')
      } else {
        onChange(option)
      }
    }
  }))

  const handleOpenChange = (open: boolean) => {
    if (!open && onClose) {
      onClose(value)
    }
  }

  return (
    <DropdownMenu
      key='review'
      align='end'
      onOpenChange={handleOpenChange}
      desktop={{ width: 'w-72 max-h-[50vh] overflow-auto bg-white' }}
      items={items}
      trigger={
        <Button size='sm' variant={'plain'} tooltipShortcut='Reviews'>
          <div className='flex items-center'>
            Reviews <ChevronDownIcon />
          </div>
        </Button>
      }
    />
  )
}
