import { Button } from '@gitmono/ui/Button'
import { DropdownMenuProps } from '@gitmono/ui/DropdownMenu'
import { ChevronDownIcon } from '@gitmono/ui/Icons'

import { NewCallDropdownMenu } from '@/components/Calls/NewCallDropdownMenu'

interface Props {
  alignMenu?: DropdownMenuProps['align']
}

export function NewCallButton({ alignMenu = 'end' }: Props) {
  return (
    <NewCallDropdownMenu
      align={alignMenu}
      trigger={
        <Button variant='primary' rightSlot={<ChevronDownIcon />}>
          New call
        </Button>
      }
    />
  )
}
