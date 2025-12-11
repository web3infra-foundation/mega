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
          className='border-secondary flex flex-1 justify-between rounded-md border border-gray-200 px-3.5 py-3 hover:bg-gray-50'
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
              <div className='max-w-lg truncate text-base font-semibold text-gray-900 md:max-w-xl' title={title}>
                {title}
              </div>

              {children && <div className='mt-0.5 text-xs text-gray-500'>{children}</div>}
            </div>
          </div>

          <div className='right ml-4 flex items-center gap-2 text-xs text-gray-600'>

            {labels && (
              <div className='flex items-center'>

                {labels}
              </div>
            )}


            {sha && (
              <div className='flex items-center'>
                <div className='rounded-md px-1.5 py-1 font-mono transition duration-150 ease-in-out hover:bg-gray-100'>
                  {sha}
                </div>
              </div>
            )}


            {copyIcon && (
              <div className='flex items-center'>
                <div className='rounded-md p-1 transition duration-150 ease-in-out hover:bg-gray-100'>{copyIcon}</div>
              </div>
            )}


            {rightIcon && (
              <div className='flex items-center'>
                <div className='rounded-md p-1 transition duration-150 ease-in-out hover:bg-gray-100'>{rightIcon}</div>
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
