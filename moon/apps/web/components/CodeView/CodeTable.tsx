'use client'

import 'github-markdown-css/github-markdown-light.css'

import { useMemo } from 'react'
import { FolderIcon } from '@heroicons/react/20/solid'
import { formatDistance, fromUnixTime } from 'date-fns'
import { usePathname, useRouter } from 'next/navigation'
import RTable from './Table'
import { columnsType, DirectoryType } from './Table/type'
import Markdown from 'react-markdown'
import FileIcon from './FileIcon/FileIcon'

export interface DataType {
  oid: string
  name: string
  content_type: string
  message: string
  date: number
}

const CodeTable = ({ directory, loading, readmeContent, onCommitInfoChange}: any) => {
  const router = useRouter()
  const pathname = usePathname()
  let real_path = pathname?.replace('/tree', '')

  const markdownContentStyle = {
    margin:' 0 auto',
    marginTop: 20,
    border: '1px solid rgba(0, 0, 0, 0.112)',
    padding: '2%',
    borderRadius: '0.5rem',
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
              {record.content_type === 'file' && <FileIcon filename={record.name}  style={{width:'16px', height:'16px'}}/>}
              <a className='cursor-pointer transition-colors duration-300 hover:text-[#69b1ff] pl-2'>{record.name}</a>
             
            </div>
          </>
        )
      },
      {
        title: 'Message',
        dataIndex: ['commit_message'],
        key: 'commit_message',
        render: (_, {commit_message}) => (
          <a className='cursor-pointer transition-colors duration-300 text-gray-600 hover:text-[#69b1ff]'>{commit_message}</a>
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

      onCommitInfoChange?.(newPath);
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
