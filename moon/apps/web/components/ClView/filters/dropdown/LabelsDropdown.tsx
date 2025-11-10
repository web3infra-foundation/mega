import React, { useEffect, useRef } from 'react'
import { useRouter } from 'next/router'

import { LabelItem } from '@gitmono/types'

import { DropdownItemwithLabel } from '../DropdownItems'
import { FilterDropdown } from '../FilterDropdown'



interface LabelsDropdownProps {
  labels: LabelItem[]
  value: string[]
  onChange: (labels: string[]) => void
  onClose: (labels: string[]) => void
}

export function LabelsDropdown({ labels, value, onChange, onClose }: LabelsDropdownProps) {
  const router = useRouter()
  const initializedRef = useRef(false)

  useEffect(() => {
    if (initializedRef.current || labels.length === 0 || value.length > 0) {
      return
    }
    const q = router.query.q as string

    if (q) {
      const labelMatch = q.match(/^label:(.+)$/)

      if (labelMatch) {
        const labelName = labelMatch[1]
        const foundLabel = labels.find((label) => label.name === labelName)

        if (foundLabel) {
          initializedRef.current = true
          const labelId = String(foundLabel.id)
          onChange([labelId])
        }
      }
    }
  }, [labels, value.length, onChange, router.query.q])






  const items = labels.map((label) => ({
    type: 'item' as const,
    label: <DropdownItemwithLabel label={label} />,
    className: 'overflow-hidden',
    onSelect: (e: Event) => {
      e.preventDefault()
      const labelId = String(label.id)

      if (value.includes(labelId)) {
        onChange(value.filter((id) => id !== labelId))
      } else {
        onChange([...value, labelId])
      }
    }
  }))


  const selectedItems = items.filter((item) => {
    const labelItem = labels.find(
      (l) =>
        item.label &&
        React.isValidElement(item.label) &&
        (item.label.props as { label?: typeof l }).label === l
    )

    return labelItem && value.includes(String(labelItem.id))
  })

  const handleOpenChange = (open: boolean) => {
    if (!open && onClose) {
      onClose(value)
    }
  }

  return (
    <FilterDropdown
      name='Labels'
      items={items}
      selectedItems={selectedItems}
      isChosen={value.length === 0}
      hasSearch={true}
      onOpenChange={handleOpenChange}
    />
  )
}

