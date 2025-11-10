import React, { useEffect, useState } from 'react'

import { Button, ButtonProps, ChevronLeftIcon, ChevronRightIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import {  BreadcrumbTitlebarContainer } from '@/components/Titlebar/BreadcrumbTitlebar'

import { getPages } from './utils/getPages'

// import { currentPage } from './utils/store'

interface PaginationType {
  totalNum: number
  pageSize: number
  currentPage: number
  onChange: (page: number) => void
}

export const Pagination = ({
                                               totalNum,
                                               pageSize,
                                               onChange,
                                               currentPage,
                                             }: PaginationType ) => {
  if (totalNum < 0 || pageSize < 0) throw new Error('invalid props')
  const totalPages = Math.ceil(totalNum / pageSize)
  const [pages, setPages] = useState<(number | '...')[]>([])

  const handleChange = (page: number) => {
    if (page < 1 || page > totalPages) return
    onChange?.(page)
  }

  useEffect(() => {
    setPages(getPages(currentPage, totalPages))
  }, [currentPage, totalPages])
  return (
    <>
      {totalPages <= 1 ? (
          <div></div>
      ): (
        <BreadcrumbTitlebarContainer className='h-auto justify-center gap-2 border-b-transparent pt-1 '>
          {currentPage === 1 ? (
            <PreviousOrNext isNext={false} disabled={true} color='text-[#818b98]' />
          ) : (
            <PreviousOrNext
              onClick={() => handleChange(currentPage - 1)}
              isNext={false}
              disabled={false}
              color='text-[#0969da]'
            />
          )}


          {pages.map((p, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <React.Fragment key={index}>
              {p === '...' ? (
                <PaginationItem disabled={true} variant='plain' key={p}>
                  {p}
                </PaginationItem>
              ) : (
                <PaginationItem
                  onClick={() => handleChange(p)}
                  variant={currentPage === p ? 'flat' : 'plain'}
                  key={p}
                  tooltip={p.toString()}
                >
                  {p}
                </PaginationItem>
              )}
            </React.Fragment>
          ))}



          {currentPage === totalPages ? (
            <PreviousOrNext isNext={true} disabled={true} color='text-[#818b98]' />
          ) : (
            <PreviousOrNext
              onClick={() => handleChange(currentPage + 1)}
              isNext={true}
              disabled={false}
              color='text-[#0969da]'
            />
          )}
        </BreadcrumbTitlebarContainer>
      )}
    </>
  )
}

const PaginationItem = (props: ButtonProps) => {
  return <Button {...props} className='shadow-none' />
}

type PreviousOrNext = ButtonProps & { isNext: boolean; color: string }

const PreviousOrNext = ({ isNext, color, ...rest }: PreviousOrNext) => {
  return (
    <>
      {isNext ? (
        <PaginationItem {...rest}>
          <div className={cn('flex items-center justify-center', color)}>
            <span>Next</span>
            <ChevronRightIcon />
          </div>
        </PaginationItem>
      ) : (
        <PaginationItem {...rest}>
          <div className={cn('flex items-center justify-center', color)}>
            <ChevronLeftIcon />
            <span>Previous</span>
          </div>
        </PaginationItem>
      )}
    </>
  )
}