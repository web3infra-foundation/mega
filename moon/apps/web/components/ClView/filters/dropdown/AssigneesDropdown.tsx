import React from 'react'

import { SyncOrganizationMember } from '@gitmono/types'

import { DropdownItemwithAvatar } from '../DropdownItems'
import { FilterDropdown } from '../FilterDropdown'

interface AssigneesDropdownProps {
  members: SyncOrganizationMember[]
  value: string[]
  onChange: (assignees: string[]) => void
  onClose?: (assignees: string[]) => void
}

export function AssigneesDropdown({ members, value, onChange, onClose }: AssigneesDropdownProps) {
  const items = members.map((member) => ({
    type: 'item' as const,
    label: <DropdownItemwithAvatar member={member} classname='text-sm' />,
    className: 'overflow-hidden',
    onSelect: (e: Event) => {
      e.preventDefault()
      const username = member.user.username

      if (value.includes(username)) {
        onChange(value.filter((u) => u !== username))
      } else {
        onChange([...value, username])
      }
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
      name='Assignees'
      items={items}
      selectedItems={selectedItems}
      isChosen={value.length === 0}
      hasSearch={true}
      onOpenChange={handleOpenChange}
    />
  )
}

