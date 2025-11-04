import { useState } from 'react'
import { Flex } from '@radix-ui/themes'
import { formatDistanceToNow } from 'date-fns'

import { Avatar, Button, ClockIcon, EyeIcon } from '@gitmono/ui'

import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'
import { useGetLatestCommit } from '@/hooks/useGetLatestCommit'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'

const CommitHyStyle = {
  width: '100%',
  background: '#fff',
  border: '1px solid #d1d9e0',
  borderRadius: 8
}

interface CommitHistoryProps {
  flag: string
  path?: string
  refs?: string
}

export default function CommitHistory({ flag, path, refs }: CommitHistoryProps) {
  const [Expand, setExpand] = useState(false)
  const { data: commitData } = useGetLatestCommit(path, refs)
  const { data: memberData } = useGetOrganizationMember({ username: commitData?.author, enabled: !!commitData?.author })

  const ExpandDetails = () => {
    setExpand(!Expand)
  }

  if (!commitData) {
    return null
  }

  const commit = commitData
  // Convert Unix timestamp (in seconds) to milliseconds for Date object
  const dateInMs = commit.date ? parseInt(commit.date) * 1000 : 0
  const formattedDate = dateInMs ? formatDistanceToNow(new Date(dateInMs), { addSuffix: true }) : ''
  const shortHash = commit.oid?.substring(0, 7) || ''

  return (
    <>
      <div style={CommitHyStyle}>
        <Flex align='center' className='p-1'>
          <MemberHovercard username={commit.author} role='member'>
            <Flex align='center'>
              <Avatar src={memberData?.user?.avatar_url || ''} />
              <span className='mx-3 font-bold'>{commit.author}</span>
            </Flex>
          </MemberHovercard>
          <span className='min-w-0 flex-1 truncate text-sm text-gray-500'>{commit.short_message}</span>
          {flag === 'contents' && (
            <Flex>
              <Button
                size='sm'
                variant='plain'
                className='ml-1 p-0'
                tooltip='Open commit details'
                onClick={ExpandDetails}
              >
                <EyeIcon size={24} />
              </Button>
            </Flex>
          )}

          <span className='ml-auto mr-3 text-xs text-gray-400'>
            {shortHash} · {formattedDate}
          </span>
          <Button size='large' variant='plain' className='flex items-center'>
            <Flex align='center'>
              <ClockIcon size={24} />
              <span className='ml-2'>History</span>
            </Flex>
          </Button>
        </Flex>
      </div>

      {Expand && commitData && (
        <p className='ml-4 text-neutral-500'>
          Signed-off-by: {commitData.author} {'<'}
          {memberData?.user?.email}
          {'>'}
        </p>
      )}
    </>
  )
}
