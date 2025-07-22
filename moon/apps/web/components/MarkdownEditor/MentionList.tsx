import { ComponentPropsWithoutRef } from 'react'
import { uniqBy } from 'remeda'

import { Mention, LinkIssue } from '@gitmono/editor/extensions'
import { OrganizationMember, SyncOrganizationMember } from '@gitmono/types/generated'
import { GitCommitIcon, UIText } from '@gitmono/ui'

import { AppBadge } from '@/components/AppBadge'
import { GuestBadge } from '@/components/GuestBadge'
import { MemberAvatar } from '@/components/MemberAvatar'
import { SuggestionItem, SuggestionRoot, useSuggestionEmpty } from '@/components/SuggestionList'
import { useGetOauthApplications } from '@/hooks/useGetOauthApplications'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'

type Props = Pick<ComponentPropsWithoutRef<typeof SuggestionRoot>, 'editor'> & {
  defaultMentions?: (OrganizationMember | SyncOrganizationMember)[]
  modal?: boolean
}

export function MentionList({ editor, defaultMentions, modal }: Props) {
  return (
    <>
      <SuggestionRoot
        modal={modal}
        editor={editor}
        char='@'
        allow={({ state, range }) => {
          const $from = state.doc.resolve(range.from)
          const type = state.schema.nodes[Mention.name]
          const allow = !!$from.parent.type.contentMatch.matchType(type)

          return allow
        }}
        minScore={0.2}
        contentClassName='p-0 max-h-[min(480px,var(--radix-popover-content-available-height))] w-[min(350px,var(--radix-popover-content-available-width))]'
        listClassName='p-1'
      >
        <InnerMentionList editor={editor} defaultMentions={defaultMentions} />
      </SuggestionRoot>
      <SuggestionRoot
        modal={modal}
        editor={editor}
        char='$'
        allow={({ state, range }) => {
          const $from = state.doc.resolve(range.from)
          const type = state.schema.nodes[LinkIssue.name]
          const allow = !!$from.parent.type.contentMatch.matchType(type)

          return allow
        }}
        minScore={0.2}
        contentClassName='p-0 max-h-[min(480px,var(--radix-popover-content-available-height))] w-[min(350px,var(--radix-popover-content-available-width))]'
        listClassName='p-1'
      >
        <InnerIssueList editor={editor} />
      </SuggestionRoot>
    </>
  )
}

function InnerMentionList({ editor, defaultMentions }: Pick<Props, 'editor' | 'defaultMentions'>) {
  const { members: allMembers } = useSyncedMembers()
  const { data: apps = [] } = useGetOauthApplications()
  const isEmptySearch = useSuggestionEmpty()
  const members = isEmptySearch && defaultMentions && defaultMentions.length > 0 ? defaultMentions : allMembers
  const dedupedMembers = uniqBy(members, (m) => m.id)

  const items = [
    ...dedupedMembers,
    ...apps
      .filter((app) => app.mentionable)
      .map((app) => ({
        id: app.id,
        role: 'app',
        user: {
          id: app.id,
          display_name: app.name,
          username: app.name,
          avatar_urls: app.avatar_urls,
          integration: true,
          notifications_paused: false
        }
      }))
  ]

  return items.map((member) => (
    <SuggestionItem
      key={member.id}
      editor={editor}
      value={member.user.username}
      keywords={[member.user.display_name !== member.user.username ? member.user.display_name : '']}
      // downrank guest members
      scoreModifier={member.role === 'guest' ? 0.3 : 1}
      onSelect={({ editor, range }) =>
        editor.commands.insertMention({
          range,
          id: member.id,
          label: member.user.display_name,
          username: member.user.username,
          role: member.role === 'app' ? 'app' : 'member'
        })
      }
    >
      <MemberAvatar
        displayStatus
        member={{
          user: member.user
        }}
        size='sm'
      />
      <div className='flex flex-1 items-center justify-between'>
        <UIText className='truncate'>{member.user.display_name}</UIText>
        {member.role === 'guest' && <GuestBadge />}
        {member.role === 'app' && <AppBadge />}
      </div>
    </SuggestionItem>
  ))
}

function InnerIssueList({ editor }: Pick<Props, 'editor'>) {
  const items = [
    {
      role: 'app',
      issue: {
        hash: 'OCN2RX13',
        name: 'MR/Issue open close icon 需要替换'
      }
    },
    {
      role: 'app',
      issue: {
        hash: 'DVUAVF2J',
        name: 'cccc'
      }
    }
  ]

  return items.map((member) => (
    <SuggestionItem
      key={member.issue.hash}
      editor={editor}
      value={member.issue.hash}
      keywords={[member.issue.hash]}
      // downrank guest members
      scoreModifier={member.role === 'guest' ? 0.3 : 1}
      onSelect={({ editor, range }) =>
        editor.commands.insertIssue({
          range,
          id: member.issue.hash,
          label: member.issue.hash
        })
      }
    >
      <GitCommitIcon />
      <div className='flex flex-1 items-center justify-between'>
        <UIText className='truncate'>{member.issue.name}</UIText>
        <UIText className='justify-self-end truncate text-sm text-gray-500'>{member.issue.hash}</UIText>
      </div>
    </SuggestionItem>
  ))
}
