import { LabelItem } from '@gitmono/types/generated'

import { SearchProps } from '@/components/Issues/Search'

type SearchType = SearchProps['SearchListTable']['items']

export const tags: LabelItem[] = [
  {
    id: 12334444,
    name: 'Bug',
    color: '#ff4d4f',
    description: 'Issue related to bug'
  },
  {
    id: 333444555,
    name: 'Feature',
    color: '#1890ff',
    description: 'Feature or enhancement'
  },
  {
    id: 6667778888,
    name: 'Docs',
    color: '#52c41a',
    description: 'Related to documentation'
  },
  {
    id: 8887775555,
    name: 'Design',
    color: '#faad14',
    description: 'Design consideration'
  },
  {
    id: 55544433222,
    name: 'Question',
    color: '#13c2c2',
    description: 'General questions or clarification'
  }
]

export const orderTags = ['Created on', 'Last updated', 'Total comments', 'Best match', 'Oldest', 'Newest']

export const searchList: SearchType = [
  {
    type: 'item',
    label: 'Is'
  },
  {
    type: 'item',
    label: 'State'
  },
  {
    type: 'item',
    label: 'Author'
  },
  {
    type: 'item',
    label: 'Project'
  },
  {
    type: 'item',
    label: 'Involvs'
  },
  { type: 'separator' },

  {
    type: 'item',
    label: 'AND'
  },
  {
    type: 'item',
    label: 'OR'
  },
  {
    type: 'item',
    label: 'Exculd'
  }
]

export const fuseOptions = {
  isCaseSensitive: false,
  // includeScore: false,
  // ignoreDiacritics: false,
  // shouldSort: true,
  // includeMatches: false,
  findAllMatches: true,
  // minMatchCharLength: 1,
  location: 0,
  threshold: 0,
  // distance: 100,
  useExtendedSearch: true,
  ignoreLocation: false,
  // ignoreFieldNorm: false,
  // fieldNormWeight: 1,
  keys: ['label']
}
