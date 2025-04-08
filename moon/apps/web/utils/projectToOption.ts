import { createElement } from 'react'

import { SyncProject } from '@gitmono/types/generated'
import { SelectOption } from '@gitmono/ui/Select'

import { ProjectAccessory } from '@/components/Projects/ProjectAccessory'

export function projectToOption(project: SyncProject): SelectOption {
  return {
    value: project.id,
    label: project.name,
    leftSlot: createElement(ProjectAccessory, { project: project })
  }
}
