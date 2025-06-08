import { ReactNode } from 'react'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom, useAtomValue } from 'jotai'
import { atomFamily } from 'jotai/utils'
import { useRouter } from 'next/router'

import {
  Button,
  ChatBubbleIcon,
  CheckCircleFilledFlushIcon,
  ChevronDownIcon,
  Command,
  ConditionalWrap,
  useCommand
} from '@gitmono/ui'

import { Item } from '@/components/Issues/IssuesContent'
import { darkModeAtom, filterAtom, sortAtom } from '@/components/Issues/utils/store'
import { SubjectCommand } from '@/components/Subject/SubjectCommand'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

import { IssueIndexTabFilter } from './IssueIndex'

interface Props {
  Issuelists: Item[]
  hideProject?: boolean
}

export function IssueList({ Issuelists, hideProject }: Props) {
  const { scope } = useScope()
  const router = useRouter()
  const filter = useAtomValue(filterAtom(scope))
  const sort = useAtomValue(sortAtom({ scope, filter }))
  // const groups = useMemo(() => groupByDate(notes, (note) => note[sort]), [notes, sort])
  const needsCommandWrap = !useCommand()
  const isDark = useAtomValue(darkModeAtom)

  return (
    <>
      {!isDark ? (
        <div className='issuecontainer overflow-hidden rounded-md border border-[#d0d7de]'>
          <ListBanner />

          <ConditionalWrap
            condition={needsCommandWrap}
            wrap={(children) => (
              <SubjectCommand>
                <Command.List className='flex flex-1 flex-col'>{children}</Command.List>
              </SubjectCommand>
            )}
          >
            {Issuelists.map((i) => {
              return (
                <ListItem
                  key={i.link}
                  title={i.title}
                  leftIcon={<CheckCircleFilledFlushIcon color='#378f50' size={16} />}
                  rightIcon={<ChatBubbleIcon />}
                  onClick={() => router.push(`/${scope}/issue/${i.link}`)}
                >
                  <div className='text-xs text-[#59636e]'>
                    {i.link} · {i.owner} {i.status}{' '}
                    {formatDistance(fromUnixTime(i.open_timestamp), new Date(), { addSuffix: true })}
                  </div>
                </ListItem>
              )
            })}
          </ConditionalWrap>
        </div>
      ) : (
        <div>darkMode</div>
      )}
    </>
  )
}

type IssuePickerType = 'Author' | 'Labels' | 'Projects' | 'Milestones' | 'Assignees' | 'Types'

const ListPicker = <T extends IssuePickerType>({ Sign }: { Sign: T }) => {
  const filterAtom = atomFamily(() => atomWithWebStorage<T>(`${Sign}:picker`, Sign))
  const [filter, setFilter] = useAtom(filterAtom(Sign))

  // TODO
  // logic of onClick will change later
  // storage will store the specific value from backend when chose the options
  return (
    <>
      <Button size='sm' onClick={() => setFilter(Sign)} variant={'plain'} tooltipShortcut={Sign}>
        <div className='flex items-center justify-center'>
          {Sign}
          <ChevronDownIcon />
        </div>
      </Button>
    </>
  )
}

export const ListBanner = () => {
  // TODO: Authors, Labels, Projects, Milestones, Assignees need to be stored in storgae in future
  const pickerTypes: IssuePickerType[] = ['Author', 'Labels', 'Projects', 'Milestones', 'Assignees', 'Types']

  return (
    <>
      <BreadcrumbTitlebar className='justify-between'>
        <ConditionalWrap condition={true} wrap={(c) => <div>{c}</div>}>
          <IssueIndexTabFilter />
        </ConditionalWrap>
        <ConditionalWrap condition={true} wrap={(c) => <div>{c}</div>}>
          {pickerTypes.map((p) => (
            <ListPicker key={p} Sign={p} />
          ))}
        </ConditionalWrap>
      </BreadcrumbTitlebar>
    </>
  )
}

export const ListItem = ({
  title,
  children,
  leftIcon,
  rightIcon,
  onClick
}: {
  title: string
  children?: ReactNode
  leftIcon?: ReactNode
  rightIcon?: ReactNode
  onClick?: () => void
}) => {
  return (
    <>
      <div className='container flex justify-between border-b border-gray-300 px-3.5 py-3 hover:bg-black/[0.08]'>
        <div className='left flex gap-3'>
          <div className='mt-1'>{leftIcon}</div>
          <div
            onClick={(e) => {
              e.stopPropagation()
              onClick?.()
            }}
            className='inner flex flex-col hover:cursor-pointer'
          >
            {title}
            {children}
          </div>
        </div>
        <div className='right'>
          <div className='mt-1'>{rightIcon}</div>
        </div>
      </div>
    </>
  )
}
