type RenderFn<T> = (value: any, record: T, index?: number) => React.ReactNode
export interface columnsType<T> {
  title: string
  dataIndex: string[]
  key: string
  render: RenderFn<T>
}

export interface DirectoryType {
  content_type: string
  date: string
  message: string
  name: string
  oid: string
}
