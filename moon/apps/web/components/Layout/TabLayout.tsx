import { PropsWithChildren } from 'react'
import { ChecklistIcon, CommentDiscussionIcon, FileDiffIcon } from '@primer/octicons-react'
import { UnderlineNav } from '@primer/react'
import { useAtom } from 'jotai'

import { tabAtom } from '../MrView/components/Checks/cpns/store'

export const TabLayout = ({ children }: PropsWithChildren) => {
  const [tab, setTab] = useAtom(tabAtom)

  return (
    <>
      <UnderlineNav aria-label='Repository with leading icons'>
        <UnderlineNav.Item
          aria-selected={tab === 'conversation'}
          onClick={() => setTab('conversation')}
          icon={CommentDiscussionIcon}
          // aria-current='page'
        >
          Conversation
        </UnderlineNav.Item>
        <UnderlineNav.Item aria-selected={tab === 'check'} onClick={() => setTab('check')} icon={ChecklistIcon}>
          Checks
        </UnderlineNav.Item>
        <UnderlineNav.Item
          aria-selected={tab === 'filechange'}
          onClick={() => setTab('filechange')}
          icon={FileDiffIcon}
        >
          Files Changed
        </UnderlineNav.Item>
      </UnderlineNav>
      {children}
    </>
  )
}
