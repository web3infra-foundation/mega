import React, { forwardRef, memo, ReactNode } from 'react'

import { Button, ChatBubbleIcon, Command, ConditionalWrap, LoadingSpinner, useCommand } from '@gitmono/ui'

import { SubjectCommand } from '@/components/Subject/SubjectCommand'
import { BreadcrumbTitlebarContainer } from '@/components/Titlebar/BreadcrumbTitlebar'
import {  PostApiIssueListData } from '@gitmono/types'
import { getFontColor } from '@/utils/getFontColor'
import { MemberHoverAvatarList } from '@/components/Issues/MemberHoverAvatarList'



export function List<T>({
  lists,
  header,
  children,
  isLoading = false
}: {
  lists: T[]
  hideProject?: boolean
  header?: React.ReactNode
  children?: (issue: T[]) => React.ReactNode
  isLoading?: boolean
}) {
  const needsCommandWrap = !useCommand()

  return (
    <div className='overflow-auto rounded-md border border-primary bg-primary'>
      {header}

      {isLoading ? (
        <div className='flex h-[400px] items-center justify-center bg-secondary'>
          <LoadingSpinner />
        </div>
      ) : lists.length === 0 ? (
        <div className='flex h-[200px] items-center justify-center bg-secondary'>
          <div className='text-center '>
            <strong className='text-xl'>No results</strong>
            <p className='text-gray-500'>Try adjusting your search filters.</p>
          </div>
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
          {children?.(lists)}
        </ConditionalWrap>
      )}
    </div>
  )
}


export function IndexTabFilter({
                                      part,
                                      setPart,
                                      openTooltip,
                                      closeTooltip
                                    }: {
  part: string
  setPart: (part: string) => void
  openTooltip?: string
  closeTooltip?: string
}) {
  return (
    <>
      <Button
        size='sm'
        onClick={() => setPart('open')}
        variant={part === 'open' ? 'flat' : 'plain'}
        tooltip={openTooltip}
      >
        Open
      </Button>
      <Button
        size='sm'
        onClick={() => setPart('closed')}
        variant={part === 'closed' ? 'flat' : 'plain'}
        tooltip={closeTooltip}
      >
        Closed
      </Button>
    </>
  )
}


interface ListBannerProps {
  children?: React.ReactNode
  tabfilter?: React.ReactNode
}

export const ListBanner = forwardRef<HTMLDivElement, ListBannerProps>(
  ({ children, tabfilter }: ListBannerProps, ref) => {
    return (
      <>
        <div ref={ref}>
          <BreadcrumbTitlebarContainer className='justify-between'>
            <ConditionalWrap condition={true} wrap={(c) => <div>{c}</div>}>
              {tabfilter}
            </ConditionalWrap>
            <ConditionalWrap condition={true} wrap={(c) => <div className='flex items-center '>{c}</div>}>
              {children}
            </ConditionalWrap>
          </BreadcrumbTitlebarContainer>
        </div>
      </>
    )
  }
)

ListBanner.displayName = 'ListBanner'



export const ListItem = memo(
  ({
    title,
    children,
    leftIcon,
    labels,
    rightIcon,
    onClick
  }: {
    title: string
    children?: ReactNode
    leftIcon?: ReactNode
    labels?: ReactNode
    rightIcon?: ReactNode
    onClick?: () => void
  }) => {
    return (
      <>
        <div className='container flex justify-between border-b border-secondary px-3.5 py-3 hover:bg-tertiary'>
          <div className='left flex gap-3'>
            <div className='mt-1'>{leftIcon}</div>

            <div
              onClick={(e) => {
                e.stopPropagation()
                onClick?.()
              }}
              className='inner flex flex-col hover:cursor-pointer'
            >
              <div className='max-w-lg truncate font-semibold md:max-w-xl' title={title}>
                {title}
              </div>

              {children}
            </div>

            {labels && <div className='flex items-center'>{labels}</div>}
          </div>

          <div className='right flex items-center'>{rightIcon}</div>
        </div>
      </>
    )
  }
)

ListItem.displayName = 'ListItem'


export type ItemsType = NonNullable<PostApiIssueListData['data']>['items']

export const ItemLabels = ({ item }: { item: ItemsType[number] }) => {
  return (
    <div
      style={{
        visibility: `${item.labels.length === 0 ? 'hidden' : 'unset'}`
      }}
      className='flex items-center gap-2 text-sm'
    >
      {item.labels.map((label) => {
        const fontColor = getFontColor(label.color)

        return (
          <span
            key={label.id}
            style={{
              backgroundColor: label.color,
              color: fontColor.toHex(),
              borderRadius: '16px',
              padding: '0px 8px',
              fontSize: '12px',
              fontWeight: '550',
              justifyContent: 'center',
              textAlign: 'center'
            }}
          >
            {label.name}
          </span>
        )
      })}
    </div>
  )
}

export const ItemRightIcons = ({ item }: { item: ItemsType[number] }) => {
  return (
    // <div className='mr-10 flex w-fit items-center justify-between gap-10'>
    <div className='flex items-center gap-4'>
      <div
        style={{
          visibility: `${item.comment_num === 0 ? 'hidden' : 'unset'}`
        }}
        className='flex items-center gap-1 text-sm text-gray-500'
      >
        <ChatBubbleIcon />
        <span>{item.comment_num}</span>
      </div>

      <div className='min-w-15'>
        <MemberHoverAvatarList users={item} />
      </div>
    </div>
  )
}
