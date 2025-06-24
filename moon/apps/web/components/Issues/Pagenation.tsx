import { useEffect, useState } from 'react'
import { useAtom } from 'jotai'

import { Button, ButtonProps, ChevronLeftIcon, ChevronRightIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'

import { getPages } from './utils/getPages'
import { currentPage } from './utils/store'

interface PaginationType {
  totalNum: number
  pageSize: number
  onChange?: (page: number) => void
}

export const Pagination = ({ totalNum, pageSize, onChange }: PaginationType) => {
  if (totalNum < 0 || pageSize < 0) throw new Error('invalid props')
  const totalPages = Math.ceil(totalNum / pageSize)
  const [pages, setPages] = useState<(number | '...')[]>([])
  const [current, setCurrent] = useAtom(currentPage)
  const handleChange = (page: number) => {
    if (page < 1 || page > totalPages) return
    setCurrent(page)
    onChange?.(page)
  }

  useEffect(() => {
    setPages(getPages(current, totalPages))
  }, [current, totalPages])
  return (
    <>
      <BreadcrumbTitlebar className='h-auto justify-center gap-2 border-b-transparent'>
        {current === 1 ? (
          <PreviousOrNext isNext={false} disabled={true} color='text-[#818b98]' />
        ) : (
          <PreviousOrNext
            onClick={() => handleChange(current - 1)}
            isNext={false}
            disabled={false}
            color='text-[#0969da]'
          />
        )}
        {totalPages === 1 ? (
          <PaginationItem tooltip='1'>1</PaginationItem>
        ) : (
          pages.map((p) => (
            <>
              {p === '...' ? (
                <PaginationItem disabled={true} variant='plain' key={p}>
                  {p}
                </PaginationItem>
              ) : (
                <PaginationItem
                  onClick={() => handleChange(p)}
                  variant={current === p ? 'flat' : 'plain'}
                  key={p}
                  tooltip={p.toString()}
                >
                  {p}
                </PaginationItem>
              )}
            </>
          ))
        )}
        {current === totalPages ? (
          <PreviousOrNext isNext={true} disabled={true} color='text-[#818b98]' />
        ) : (
          <PreviousOrNext
            onClick={() => handleChange(current + 1)}
            isNext={true}
            disabled={false}
            color='text-[#0969da]'
          />
        )}
      </BreadcrumbTitlebar>
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
