import { Flex } from '@radix-ui/themes'
import { Avatar, Button, ClockIcon, EyeIcon } from '@gitmono/ui'
import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'
import CommitDetails from './CommitDetails'
import { useState } from 'react'
import { useGetCommitBinding } from '@/hooks/useGetCommitBinding'

interface UserInfo {
  avatar_url: string
  name: string
}

export interface CommitInfo {
  user: UserInfo
  message: string
  hash: string
  date: string
}

const CommitHyStyle = {
  width: '100%', 
  background: '#fff', 
  border: '1px solid #d1d9e0', 
  borderRadius: 8 
}

export default function CommitHistory({ flag, info, commitSha }: {
  flag: string, 
  info: CommitInfo,
  commitSha?: string // New prop for commit SHA to fetch binding info
}) {
  const [Expand, setExpand] = useState(false)
  const { data: commitBinding, isLoading: bindingLoading } = useGetCommitBinding(commitSha)
  
  const ExpandDetails = () => {
    setExpand(!Expand)
  }

  // Use binding info if available, otherwise fall back to passed info
  const displayUser = commitBinding?.user ? {
    avatar_url: commitBinding.avatar_url || info.user.avatar_url,
    name: commitBinding.display_name
  } : info.user

  return (
    <>
    <div style={CommitHyStyle}>
      <Flex align='center' className='p-1'>
        <MemberHovercard username={displayUser.name} role='member'>
          <Flex align='center'>
            <Avatar src={displayUser.avatar_url} />
            <span className="font-bold mx-3">
              {bindingLoading ? '加载中...' : displayUser.name}
            </span>
          </Flex>
        </MemberHovercard>
        <span className='text-gray-500 text-sm'>
          {info.message}
        </span>
        {commitBinding?.is_anonymous && (
          <span className='text-orange-500 text-xs ml-2 px-2 py-1 bg-orange-50 rounded'>
            匿名提交
          </span>
        )}
        {commitBinding?.is_verified_user && (
          <span className='text-green-500 text-xs ml-2 px-2 py-1 bg-green-50 rounded'>
            已验证用户
          </span>
        )}
        {
          flag === 'contents' &&
          <Flex>
            <Button 
              size='sm'
              variant='plain'
              className='p-0 ml-1'
              tooltip='Open commit details'
              onClick={ExpandDetails}>
              <EyeIcon size={24} />
            </Button>
          </Flex>
        }

        <span className='text-gray-400 text-xs ml-auto mr-3'>
          {info.hash} · {info.date}
        </span>
        <Button
          size='large'
          variant='plain'
          className='flex items-center'
        >
          <Flex align='center'>
            <ClockIcon size={24} />
            <span className='ml-2'>History</span>
          </Flex>
        </Button>
      </Flex>
    </div>
    {Expand &&<CommitDetails/>}
    </>
  )
}
