import React from 'react'

import { Button, ChevronDownIcon } from '@gitmono/ui'


export function TypesDropdown() {
  return (
    <Button size='sm' variant={'plain'} tooltipShortcut='Types'>
      <div className='flex items-center justify-center '>
        Types <ChevronDownIcon />
      </div>
    </Button>
  )
}

