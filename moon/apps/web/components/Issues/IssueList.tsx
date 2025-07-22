import React, { forwardRef, memo, ReactNode, useRef, useState } from 'react'
import { useAtomValue } from 'jotai'

import { LabelItem, SyncOrganizationMember } from '@gitmono/types/index'
import {
  Button,
  ChevronDownIcon,
  cn,
  Command,
  ConditionalWrap,
  LazyLoadingSpinner,
  LoadingSpinner,
  SearchIcon,
  useCommand
} from '@gitmono/ui'
import { DropdownMenu } from '@gitmono/ui/DropdownMenu'
import { MenuItem } from '@gitmono/ui/Menu'

import { darkModeAtom } from '@/components/Issues/utils/store'
import { SubjectCommand } from '@/components/Subject/SubjectCommand'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'

import { MemberAvatar } from '../MemberAvatar'

export function IssueList<T>({
  Issuelists,
  header,
  children,
  isLoading = false
}: {
  Issuelists: T[]
  hideProject?: boolean
  header?: React.ReactNode
  children?: (issue: T[]) => React.ReactNode
  isLoading?: boolean
}) {
  const needsCommandWrap = !useCommand()
  const isDark = useAtomValue(darkModeAtom)

  return (
    <>
      {!isDark ? (
        <div className='max-h-[600px] overflow-auto rounded-md border border-[#d0d7de]'>
          {header}

          {isLoading ? (
            <div className='flex h-[400px] items-center justify-center'>
              <LoadingSpinner />
            </div>
          ) : (
            <ConditionalWrap
              condition={needsCommandWrap}
              wrap={(children) => (
                <SubjectCommand>
                  <Command.List className='flex flex-1 flex-col'>{children}</Command.List>
                </SubjectCommand>
              )}
            >
              {children?.(Issuelists)}
            </ConditionalWrap>
          )}
        </div>
      ) : (
        <div>darkMode</div>
      )}
    </>
  )
}

interface ListBannerProps {
  pickerTypes: string[]
  children?: (p: string) => React.ReactNode
  tabfilter?: React.ReactNode
}

export const ListBanner = forwardRef<HTMLDivElement, ListBannerProps>(
  ({ pickerTypes, children, tabfilter }: ListBannerProps, ref) => {
    return (
      <>
        <div ref={ref}>
          <BreadcrumbTitlebar className='justify-between'>
            <ConditionalWrap condition={true} wrap={(c) => <div>{c}</div>}>
              {tabfilter}
              {/* <IssueIndexTabFilter /> */}
            </ConditionalWrap>
            <ConditionalWrap condition={true} wrap={(c) => <div>{c}</div>}>
              {pickerTypes.map((p) => {
                return <React.Fragment key={p}>{children?.(p)}</React.Fragment>
              })}
            </ConditionalWrap>
          </BreadcrumbTitlebar>
        </div>
      </>
    )
  }
)

ListBanner.displayName = 'ListBanner'

export const DropdownItemwithAvatar = ({
  member,
  classname
}: {
  member: SyncOrganizationMember
  classname?: string
}) => {
  return (
    <div
      className={cn(
        'flex items-center gap-2 rounded-md border-l-4 border-transparent p-2 hover:border-[#0969da]',
        classname
      )}
    >
      <MemberAvatar size='sm' member={member} />
      <span className='text-sm font-semibold'>{member.user.display_name}</span>
      <span className='ml-1 text-xs text-gray-500'>{member.user.username}</span>
    </div>
  )
}

export const DropdownItemwithLabel = ({ classname, label }: { classname?: string; label: LabelItem }) => {
  return (
    <div
      className={cn(
        'flex items-center gap-2 rounded-md border-l-4 border-transparent p-2 hover:border-[#0969da]',
        classname
      )}
    >
      <div
        className='h-3.5 w-3.5 rounded-full border'
        //eslint-disable-next-line react/forbid-dom-props
        style={{ backgroundColor: label.color, borderColor: label.color }}
      />
      <span className='text-sm font-semibold'>{label.name}</span>
      <span className='ml-1 text-xs text-gray-500'>{label.description}</span>
    </div>
  )
}

export const DropdownOrder = ({
  name,
  dropdownArr,
  dropdownItem,
  onOpen,
  open,
  inside
}: {
  name: string
  dropdownArr: MenuItem[]
  dropdownItem?: MenuItem[]
  onOpen?: (open: boolean) => void
  open?: boolean
  inside?: React.ReactNode
}) => {
  return (
    <>
      <DropdownMenu
        open={open}
        onOpenChange={onOpen}
        key={name}
        align='end'
        desktop={{ width: 'w-72 max-h-[50vh] overflow-auto bg-white' }}
        items={[
          {
            type: 'item',
            disabled: true,
            label: <p>Sort by</p>
            // className: 'sticky top-0 z-50 bg-white'
          },
          ...dropdownArr,
          { type: 'separator' },
          {
            type: 'item',
            disabled: true,
            label: <p>Order</p>
            // className: 'sticky top-0 z-50 bg-white'
          },
          ...(dropdownItem as MenuItem[])
        ]}
        trigger={
          <Button size='sm' variant={'plain'} tooltipShortcut={name}>
            {inside ? (
              inside
            ) : (
              <>
                {name} <ChevronDownIcon />
              </>
            )}
          </Button>
        }
      />
    </>
  )
}
export const DropdownReview = ({
  name,
  dropdownArr,
  dropdownItem,
  onOpen,
  open
}: {
  name: string
  dropdownArr: MenuItem[]
  dropdownItem?: MenuItem[]
  onOpen?: (open: boolean) => void
  open?: boolean
  inside?: React.ReactNode
}) => {
  return (
    <>
      <DropdownMenu
        open={open}
        onOpenChange={onOpen}
        key={name}
        align='end'
        desktop={{ width: 'w-72 max-h-[50vh] overflow-auto bg-white' }}
        items={[...dropdownArr, ...(dropdownItem as MenuItem[])]}
        trigger={
          <Button size='sm' variant={'plain'} tooltipShortcut={name}>
            <div className='flex items-center'>
              {name} <ChevronDownIcon />
            </div>
          </Button>
        }
      />
    </>
  )
}

// dropdownArr是不一样的，其他一样
export const Dropdown = ({
  name,
  dropdownArr,
  dropdownItem,
  isChosen,
  onOpen,
  // open,
  inside
}: {
  name: string
  dropdownArr: MenuItem[]
  dropdownItem?: MenuItem[]
  isChosen: boolean
  onOpen?: (open: boolean) => void
  open?: boolean
  inside?: React.ReactNode
}) => {
  const [query, setQuery] = useState('')
  // const { scope } = useScope()
  // const [sort] = useAtom(sortAtom({ scope, filter: 'sortPicker' }))
  const isSearching = query.length > 0
  const ref = useRef<HTMLInputElement>(null)
  const [open, setOpen] = useState<boolean>(false)

  const handleOpenChange = (isOpen: boolean) => {
    setOpen(isOpen)
    if (onOpen) {
      onOpen(isOpen) // 把状态变化通知给父组件
    }
  }

  const DropdownSearch = () => (
    <div className='flex flex-1 flex-row items-center gap-2'>
      <span className='text-tertiary flex h-5 w-5 items-center justify-center'>
        {isSearching ? <LazyLoadingSpinner fallback={<SearchIcon />} /> : <SearchIcon />}
      </span>
      <input
        ref={ref}
        className='flex-1 border-none bg-transparent p-0 text-sm outline-none ring-0 focus:ring-0'
        placeholder={`filter by ${name}`}
        role='searchbox'
        autoComplete='off'
        autoCorrect='off'
        spellCheck={false}
        type='text'
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            setQuery('')
            ref.current?.blur()
          } else if (e.key === 'Enter') {
            e.preventDefault()
            e.stopPropagation()
          }
        }}
      />
    </div>
  )

  return (
    <>
      {isChosen ? (
        <DropdownMenu
          open={open}
          onOpenChange={(open) => handleOpenChange(open)}
          key={name}
          align='end'
          desktop={{ width: 'w-72 max-h-[50vh] overflow-auto bg-white' }}
          items={[
            {
              type: 'item',
              disabled: true,
              label: <p>Filter by {name}</p>
              // className: 'sticky top-0 z-50 bg-white'
            },
            {
              type: 'item',
              label: <DropdownSearch />
              // onSelect: (e) => {
              //   e.preventDefault()
              //   onOpen?.(false)
              // }
              // className: 'sticky top-10 z-50 bg-white'
            },
            { type: 'separator' },
            ...dropdownArr
          ]}
          trigger={
            <Button size='sm' variant={'plain'} tooltipShortcut={name}>
              <div className='flex items-center justify-center'>
                {inside ? (
                  inside
                ) : (
                  <>
                    {name} <ChevronDownIcon />
                  </>
                )}
              </div>
            </Button>
          }
        />
      ) : (
        <DropdownMenu
          key={name}
          align='end'
          open={open}
          onOpenChange={(open) => handleOpenChange(open)}
          desktop={{ width: 'w-72' }}
          items={[
            {
              type: 'item',
              label: <p>Filter by {name}</p>,
              disabled: true
              // className: 'sticky top-0 z-50 bg-white pt-4'
            },
            {
              type: 'item',
              label: <DropdownSearch />,
              onSelect: (e) => e.preventDefault()
              // className: 'sticky top-10 z-50 bg-white pt-4'
            },
            { type: 'separator' },
            { type: 'heading', label: 'Group assignees' },
            ...(dropdownItem as MenuItem[]),
            { type: 'separator' },
            { type: 'heading', label: 'Suggestions' },
            ...dropdownArr
          ]}
          trigger={
            <Button size='sm' variant={'plain'} tooltipShortcut={name}>
              <div className='flex items-center justify-center'>
                {inside ? (
                  inside
                ) : (
                  <>
                    {name} <ChevronDownIcon />
                  </>
                )}
              </div>
            </Button>
          }
        />
      )}
    </>
  )
}

export const ListItem = memo(
  ({
    title,
    children,
    leftIcon,
    rightIcon,
    onClick
  }: {
    title: string
    children?: ReactNode
    leftIcon?: ReactNode
    rightIcon?: ReactNode
    onClick?: () => void
  }) => {
    return (
      <>
        <div className='container flex justify-between border-b border-gray-300 px-3.5 py-3 hover:bg-black/[0.08]'>
          <div className='left flex gap-3'>
            <div className='mt-1'>{leftIcon}</div>
            <div
              onClick={(e) => {
                e.stopPropagation()
                onClick?.()
              }}
              className='inner flex flex-col hover:cursor-pointer'
            >
              {title}
              {children}
            </div>
          </div>
          <div className='right'>
            <div className='mt-1'>{rightIcon}</div>
          </div>
        </div>
      </>
    )
  }
)
ListItem.displayName = 'ListItem'