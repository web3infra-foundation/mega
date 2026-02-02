import React from 'react'

import { Button, CheckIcon, OrderedListIcon } from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { MenuItem } from '@gitmono/ui/Menu'

interface OrderDropdownProps {
  sortOptions: string[]
  timeOptions: string[]
  currentSort: string
  currentTime: string
  onChange: (sort: string, time: string) => void
}

export function OrderDropdown({ sortOptions, timeOptions, currentSort, currentTime, onChange }: OrderDropdownProps) {
  const sortItems: MenuItem[] = sortOptions.map((option) => ({
    type: 'item' as const,
    label: (
      <div className='flex items-center gap-2'>
        <div className='h-4 w-4'>{currentSort === option && <CheckIcon />}</div>
        <span className='flex-1'>{option}</span>
      </div>
    ),
    className: 'overflow-hidden',
    onSelect: (e: Event) => {
      e.preventDefault()
      onChange(option, currentTime)
    }
  }))

  const timeItems: MenuItem[] = timeOptions.map((option) => ({
    type: 'item' as const,
    label: (
      <div className='flex items-center gap-2'>
        <div className='h-4 w-4'>{currentTime === option && <CheckIcon />}</div>
        <span className='flex-1'>{option}</span>
      </div>
    ),
    className: 'overflow-hidden',
    onSelect: (e: Event) => {
      e.preventDefault()
      onChange(currentSort, option)
    }
  }))

  return (
    <DropdownMenu
      key='order'
      align='end'
      desktop={{ width: 'w-72 max-h-[50vh] overflow-auto bg-primary' }}
      items={[
        {
          type: 'item',
          disabled: true,
          label: <p>Sort by</p>
        },
        ...sortItems,
        { type: 'separator' },
        {
          type: 'item',
          disabled: true,
          label: <p>Order</p>
        },
        ...timeItems
      ]}
      trigger={
        <Button size='sm' variant={'plain'} tooltipShortcut={currentSort}>
          <div className='flex items-center'>
            {currentSort}
            <OrderedListIcon />
          </div>
        </Button>
      }
    />
  )
}
