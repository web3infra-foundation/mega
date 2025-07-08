import { Conversation } from '@/pages/[org]/mr/[id]'
import HandleTime from './components/HandleTime'

interface MergedItemProps {
  conv: Conversation
}
const MergedItem = ({ conv }: MergedItemProps) => {

  return (
    <>
      <div className='flex items-center space-x-2'>
        <div>Merged via the queue into main</div>
        <div className='text-sm text-gray-500 hover:text-gray-700'>
            <HandleTime created_at={conv.created_at}/>
        </div>
      </div>
    </>
  )
}

export default MergedItem
