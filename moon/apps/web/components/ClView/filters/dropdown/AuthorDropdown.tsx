import React from 'react'

import { SyncOrganizationMember } from '@gitmono/types'

import { DropdownItemwithAvatar } from '../DropdownItems'
import { FilterDropdown } from '../FilterDropdown'

interface AuthorDropdownProps {
  members: SyncOrganizationMember[]
  value: string
  onChange: (author: string) => void
  onClose?: (author: string) => void
}

export function AuthorDropdown({ members, value, onChange, onClose }: AuthorDropdownProps) {
  const items = members.map((member) => ({
    type: 'item' as const,
    label: <DropdownItemwithAvatar member={member} classname='text-sm' />,
    className: 'overflow-hidden',
    onSelect: (e: Event) => {
      e.preventDefault()
      const newValue = member.user.username === value ? '' : member.user.username

      onChange(newValue)
    }
  }))

  const selectedItems = items.filter((item) => {
    const member = members.find(
      (m) =>
        item.label &&
        React.isValidElement(item.label) &&
        (item.label.props as { member?: typeof m }).member === m
    )

    return member && value.includes(member.user.username)
  })


  const handleOpenChange = (open: boolean) => {
    if (!open && onClose) {
      onClose(value)
    }
  }

  return (
    <FilterDropdown
      name='Author'
      items={items}
      selectedItems={selectedItems}
      isChosen={value === ''}
      hasSearch={true}
      onOpenChange={handleOpenChange}
    />
  )
}

