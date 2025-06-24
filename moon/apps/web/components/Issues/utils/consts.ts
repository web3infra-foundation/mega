import { SearchProps } from '@/components/Issues/Search'

type SearchType = SearchProps['SearchListTable']['items']

export const tags = [
  {
    id: 'label_1',
    name: 'Bug',
    color: '#ff4d4f',
    remarks: 'Issue related to bug',
    checked: false
  },
  {
    id: 'label_2',
    name: 'Feature',
    color: '#1890ff',
    remarks: 'Feature or enhancement',
    checked: true
  },
  {
    id: 'label_3',
    name: 'Docs',
    color: '#52c41a',
    remarks: 'Related to documentation',
    checked: false
  },
  {
    id: 'label_4',
    name: 'Design',
    color: '#faad14',
    remarks: 'Design consideration',
    checked: true
  },
  {
    id: 'label_5',
    name: 'Question',
    color: '#13c2c2',
    remarks: 'General questions or clarification',
    checked: false
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
