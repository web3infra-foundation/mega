import React from 'react'

import { Button, ChevronDownIcon } from '@gitmono/ui'

export function MilestonesDropdown() {
  return (
    <Button size='sm' variant={'plain'} tooltipShortcut='Milestones'>
      <div className='flex items-center justify-center'>
        Milestones <ChevronDownIcon />
      </div>
    </Button>
  )
}
