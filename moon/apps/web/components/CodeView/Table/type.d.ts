type RenderFn<T> = (value: any, record: T, index?: number) => React.ReactNode
export interface columnsType<T> {
  title: string
  dataIndex: string[]
  key: string
  render: RenderFn<T>
}

export interface DirectoryType {
  commit_message: string
  content_type: string
  date: string
  name: string
  oid: string
}
