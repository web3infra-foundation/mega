import { Card, Flex } from '@radix-ui/themes'
import { Avatar, Button, ClockIcon } from '@gitmono/ui'
import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'

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

export default function CommitHistory({ info }: { info: CommitInfo }) {
  return (
    <Card style={{ width: '100%', background: '#fff', border: '1px solid #d1d9e0', borderRadius: 8 }}>
      <Flex align='center' className='p-2'>
        <MemberHovercard username={info.user.name} role='member'>
          <Flex align='center'>
            <Avatar src={info.user.avatar_url} />
            <span className="font-bold mx-3">
              {info.user.name}
            </span>
          </Flex>
        </MemberHovercard>
        <span className='text-gray-500 text-sm'>
          {info.message}
        </span>

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
    </Card>
  )
}
