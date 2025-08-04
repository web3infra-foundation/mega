import { ConversationItem } from '@gitmono/types/generated'
import HandleTime from '../components/HandleTime'


interface ReopenItemProps {
  conv: ConversationItem
}
const ReopenItem = ({ conv }: ReopenItemProps) => {

  return (
    <>
      <div className='flex items-center space-x-2'>
        <div>{conv.comment}</div>
        <div className='text-sm text-gray-500 hover:text-gray-700'>
            <HandleTime created_at={conv.created_at}/>
        </div>
      </div>
    </>
  )
}

export default ReopenItem
