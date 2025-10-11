import { Flex } from '@radix-ui/themes'
import { Avatar, Button, ClockIcon, EyeIcon } from '@gitmono/ui'
import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'
import CommitDetails from './CommitDetails'
import { useState } from 'react'

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

export default function CommitHistory({ flag, info }: {flag:string, info: CommitInfo }) {
  const [Expand,setExpand] = useState(false)
  const ExpandDetails =()=>{
    setExpand(!Expand)
  }

  return (
    <>
    <div style={CommitHyStyle}>
      <Flex align='center' className='p-1'>
        <MemberHovercard username={info.user.name} role='member'>
          <Flex align='center'>
            <Avatar src={info.user.avatar_url} />
            <span className="font-bold mx-3">
              {info.user.name}
            </span>
          </Flex>
        </MemberHovercard>
        <span className='text-gray-500 text-sm truncate flex-1 min-w-0'>
          {info.message}
        </span>
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
