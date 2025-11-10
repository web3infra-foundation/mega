import React from 'react'

import { Button, ChevronDownIcon } from '@gitmono/ui'


export function ProjectsDropdown() {
  return (
    <Button size='sm' variant={'plain'} tooltipShortcut='Projects' >
      <div className='flex items-center justify-center '>
        Projects <ChevronDownIcon />
      </div>
    </Button>
  )
}

