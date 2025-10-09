import { useMemo, useState, useEffect } from 'react'
import { Button, DotsHorizontal, Link, LinkIcon, TrashIcon, UIText } from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { buildMenuItems } from '@gitmono/ui/Menu'
import { useCopyToClipboard } from '@gitmono/ui/src/hooks'

import { TagResponse } from '@gitmono/types'
import { useDeleteMonoTag } from '@/hooks/useDeleteMonoTag'

interface Props {
  tags: TagResponse[]
  onDelete?: (name: string) => void
}

export function MonoTagList({ tags, onDelete }: Props) {
  const [localTags, setLocalTags] = useState(tags)

  useEffect(() => {
    setLocalTags(tags)
  }, [tags])
  if (!localTags?.length) return null
  return (
    <ul className='flex flex-col py-2'>
      {localTags.map((t) => (
        <MonoTagRow key={t.name} tag={t} onDelete={() => {
          setLocalTags(localTags.filter(tag => tag.name !== t.name));
          if (typeof onDelete === 'function') onDelete(t.name);
        }} />
      ))}
    </ul>
  )
}


function MonoTagRow({ tag, onDelete }: { tag: TagResponse; onDelete?: () => void }) {
  return <InnerRow tag={tag} onDelete={onDelete} />
}

function InnerRow({ tag, onDelete }: { tag: TagResponse; onDelete?: () => void }) {
  const [copy] = useCopyToClipboard()
  const [menuOpen, setMenuOpen] = useState(false)
  const del = useDeleteMonoTag()

  const subtitle = useMemo(() => {
    return tag.message || `${tag.object_type} ${tag.object_id.substring(0, 8)}`
  }, [tag])

  const href = `/code/tags/${encodeURIComponent(tag.name)}`

  return (
    <li className='hover:bg-tertiary group-has-[button[aria-expanded="true"]]:bg-tertiary group relative -mx-3 flex items-center gap-3 rounded-md py-1.5 pl-3 pr-1.5'>
      <Link href={href} className='absolute inset-0 z-0' />

  {/* removed leading hashtag icon per design update */}

      <div className='flex flex-1 flex-col gap-0.5'>
        <UIText weight='font-medium' size='text-[15px]' className='line-clamp-1'>
          {tag.name}
        </UIText>
        {subtitle && (
          <UIText quaternary size='text-[12px]' className='line-clamp-1'>
            {subtitle}
          </UIText>
        )}
      </div>

      <div className='hidden flex-none items-center gap-0.5 lg:flex'>
        <div className='flex opacity-0 group-hover:opacity-100 group-has-[button[aria-expanded="true"]]:opacity-100'>
          <DropdownMenu
            trigger={<Button variant='plain' iconOnly={<DotsHorizontal />} accessibilityLabel='Open menu' />}
            open={menuOpen}
            onOpenChange={setMenuOpen}
            align='end'
            items={buildMenuItems([
              {
                type: 'item',
                leftSlot: <LinkIcon />,
                label: 'Copy name',
                onSelect: () => {
                  void copy(tag.name)
                }
              },
              {
                type: 'item',
                leftSlot: <TrashIcon isAnimated />,
                label: 'Delete',
                destructive: true,
                onSelect: () => {
                  del.mutate(tag.name, {
                    onSuccess: () => {
                      if (onDelete) onDelete();
                    }
                  });
                }
              }
            ])}
          />
        </div>
      </div>
    </li>
  )
}
