import React, { memo, ReactNode } from 'react'
import { atomWithStorage } from 'jotai/utils'

import { Command, ConditionalWrap, LoadingSpinner, useCommand } from '@gitmono/ui'

import { SubjectCommand } from '@/components/Subject'

export const commitPath = atomWithStorage('commitPath', '')

export const CommitsItem = memo(
  ({
    title,
    children,
    sha,
    labels,
    rightIcon,
    onClick,
    copyIcon
  }: {
    title: string
    children?: ReactNode
    sha?: ReactNode
    labels?: ReactNode
    rightIcon?: ReactNode
    copyIcon?: ReactNode
    onClick?: () => void
  }) => {
    return (
      <div className='flex flex-1 flex-col'>
        <div
          className='border-primary hover:bg-secondary flex flex-1 justify-between rounded-md border px-3.5 py-3 transition-colors'
          onClick={() => onClick?.()}
        >
          <div className='left flex min-w-0 gap-3'>
            <div
              onClick={(e) => {
                e.stopPropagation()
                onClick?.()
              }}
              className='inner flex min-w-0 flex-col hover:cursor-pointer'
            >
              <div className='text-primary max-w-lg truncate text-base font-semibold md:max-w-xl' title={title}>
                {title}
              </div>

              {children && <div className='text-tertiary mt-0.5 text-xs'>{children}</div>}
            </div>
          </div>

          <div className='right text-secondary ml-4 flex items-center gap-2 text-xs'>
            {labels && <div className='flex items-center'>{labels}</div>}

            {sha && (
              <div className='flex items-center'>
                <div className='hover:bg-tertiary rounded-md px-1.5 py-1 font-mono transition duration-150 ease-in-out'>
                  {sha}
                </div>
              </div>
            )}

            {copyIcon && (
              <div className='flex items-center'>
                <div className='hover:bg-tertiary rounded-md p-1 transition duration-150 ease-in-out'>{copyIcon}</div>
              </div>
            )}

            {rightIcon && (
              <div className='flex items-center'>
                <div className='hover:bg-tertiary rounded-md p-1 transition duration-150 ease-in-out'>{rightIcon}</div>
              </div>
            )}
          </div>
        </div>
      </div>
    )
  }
)

CommitsItem.displayName = 'CommitsItem'

export function CommitsList<T>({
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
    <div className='bg-primary overflow-auto'>
      {header}

      {isLoading ? (
        <div className='bg-secondary flex h-[400px] items-center justify-center'>
          <LoadingSpinner />
        </div>
      ) : lists.length === 0 ? (
        <div className='bg-secondary flex h-[200px] items-center justify-center'>
          <div className='text-center'>
            <strong className='text-primary text-xl'>No results</strong>
            <p className='text-tertiary'>Try adjusting your search filters.</p>
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
