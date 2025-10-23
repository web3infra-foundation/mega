'use client'

import 'github-markdown-css/github-markdown-light.css'

import { useMemo } from 'react'
import { FolderIcon } from '@heroicons/react/20/solid'
import { formatDistance, fromUnixTime } from 'date-fns'
import { usePathname, useRouter } from 'next/navigation'
import { useRouter as useNextRouter } from 'next/router'
import Markdown from 'react-markdown'

import FileIcon from './FileIcon/FileIcon'
import RTable from './Table'
import { columnsType, DirectoryType } from './Table/type'

export interface DataType {
  oid: string
  name: string
  content_type: string
  message: string
  date: number
}

const CodeTable = ({ directory, loading, readmeContent }: any) => {
  const router = useRouter()
  const pathname = usePathname()
  const nextRouter = useNextRouter()

  const refs = (nextRouter.query.version as string) || 'main'

  let real_path = pathname?.replace('/tree', '')

  const markdownContentStyle = {
    margin: ' 0 auto',
    marginTop: 20,
    border: '1px solid rgba(0, 0, 0, 0.112)',
    padding: '2%',
    borderRadius: '0.5rem'
  }

  const columns = useMemo<columnsType<DirectoryType>[]>(
    () => [
      {
        title: 'Name',
        dataIndex: ['name', 'content_type'],
        key: 'name',
        render: (_, record) => (
          <>
            <div className='flex items-center'>
              {record.content_type === 'directory' && <FolderIcon className='size-4 text-gray-600' />}
              {record.content_type === 'file' && (
                <FileIcon filename={record.name} style={{ width: '16px', height: '16px' }} />
              )}
              <a className='cursor-pointer pl-2 transition-colors duration-300 hover:text-[#69b1ff]'>{record.name}</a>
            </div>
          </>
        )
      },
      {
        title: 'Message',
        dataIndex: ['commit_message'],
        key: 'commit_message',
        render: (_, { commit_message }) => (
          <a className='cursor-pointer text-gray-600 transition-colors duration-300 hover:text-[#69b1ff]'>
            {commit_message}
          </a>
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
    const pathParts = normalizedPath?.split('/').filter(Boolean) || []

    const cleanParts = pathParts.filter((part) => part !== 'blob' && part !== 'tree' && part !== refs)

    const baseParts = cleanParts.slice(0, 2)
    const currentFilePath = cleanParts.slice(2)

    if (record.content_type === 'file') {
      const newPath = `/${baseParts.join('/')}/blob/${refs}/${[...currentFilePath, encodeURIComponent(record.name)].join('/')}`

      router.push(newPath)
    } else {
      const newPath = `/${baseParts.join('/')}/tree/${refs}/${[...currentFilePath, encodeURIComponent(record.name)].join('/')}`

      router.push(newPath)
    }
  }

  return (
    <div>
      <RTable
        columns={columns ?? []}
        datasource={directory}
        size='3'
        align='center'
        onClick={handleRowClick}
        loading={loading}
      />
      {readmeContent && (
        <div style={markdownContentStyle}>
          <div className='markdown-body'>
            <Markdown>{readmeContent}</Markdown>
          </div>
        </div>
      )}
    </div>
  )
}

export default CodeTable
