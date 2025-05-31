'use client'

import 'github-markdown-css/github-markdown-light.css'

import { useMemo } from 'react'
import { DocumentIcon, FolderIcon } from '@heroicons/react/20/solid'
import { formatDistance, fromUnixTime } from 'date-fns'
import { usePathname, useRouter } from 'next/navigation'
import Markdown from 'react-markdown'

import styles from './CodeTable.module.css'
import RTable from './Table'
import { columnsType, DirectoryType } from './Table/type'

export interface DataType {
  oid: string
  name: string
  content_type: string
  message: string
  date: number
}

const CodeTable = ({ directory, readmeContent }: any) => {
  const router = useRouter()
  const pathname = usePathname()
  let real_path = pathname?.replace('/tree', '')

  const columns = useMemo<columnsType<DirectoryType>[]>(
    () => [
      {
        title: 'Name',
        dataIndex: ['name', 'content_type'],
        key: 'name',
        render: (_, record) => (
          <>
            <div className='flex'>
              {record.content_type === 'directory' && <FolderIcon className='size-6' />}
              {record.content_type === 'file' && <DocumentIcon className='size-6' />}
              <a className='cursor-pointer transition-colors duration-300 hover:text-[#69b1ff]'>{record.name}</a>
            </div>
          </>
        )
      },
      {
        title: 'Message',
        dataIndex: ['message'],
        key: 'message',
        render: (_, { message }) => (
          <a className='cursor-pointer transition-colors duration-300 hover:text-[#69b1ff]'>{message}</a>
        )
      },
      {
        title: 'Date',
        dataIndex: ['date'],
        key: 'date',
        render: (_, { date }) => (
          <>{date && formatDistance(fromUnixTime(Number(date)), new Date(), { addSuffix: true })}</>
        )
      }
    ],
    []
  )

  const handleRowClick = (record: DirectoryType) => {
    const normalizedPath = real_path?.replace(/^\/|\/$/g, '')
    const pathParts = normalizedPath?.split('/') || []

    if (record.content_type === 'file') {
      let newPath: string

      const hasBlob = pathParts?.includes('blob')

      if (!hasBlob && pathParts.length >= 2) {
        pathParts?.splice(2, 0, 'blob')
      }
      pathParts?.push(encodeURIComponent(record.name))

      newPath = `/${pathParts?.join('/')}`
      router.push(newPath)
    } else {
      let newPath: string

      const hasTree = pathParts?.includes('tree')

      if (!hasTree && pathParts.length >= 2) {
        pathParts?.splice(2, 0, 'tree')
      }
      pathParts?.push(encodeURIComponent(record.name))

      newPath = `/${pathParts?.join('/')}`
      router.push(newPath)
    }
  }

  return (
    <div>
      <RTable columns={columns ?? []} datasource={directory} size='3' align='center' onClick={handleRowClick} />

      {readmeContent && (
        <div className={styles.markdownContent}>
          <div className='markdown-body'>
            <Markdown>{readmeContent}</Markdown>
          </div>
        </div>
      )}
    </div>
  )
}

export default CodeTable
