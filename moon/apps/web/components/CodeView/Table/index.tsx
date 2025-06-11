import { Spinner, Table } from '@radix-ui/themes'

import { columnsType, DirectoryType } from './type'

const table = <T extends DirectoryType>({
  columns,
  datasource,
  size,
  align,
  justify,
  onClick,
  loading = false
}: {
  columns: columnsType<T>[]
  datasource: T[]
  size?: '1' | '2' | '3' | undefined
  align?: 'center' | 'start' | 'end' | undefined
  justify?: 'center' | 'start' | 'end' | undefined
  onClick?: (record: T) => void
  loading?: boolean
}) => {
  return (
    <>
      <Spinner loading={loading}>
        <Table.Root size={size}>
          <Table.Header>
            <Table.Row align={align}>
              {columns.map((c) => (
                <Table.ColumnHeaderCell key={c.title}>{c.title}</Table.ColumnHeaderCell>
              ))}
            </Table.Row>
          </Table.Header>

          <Table.Body>
            {datasource.map((d, index) => {
              if (d) {
                return (
                  // eslint-disable-next-line react/no-array-index-key
                  <Table.Row className='hover:bg-gray-100' key={index}>
                    {columns.map((c, index) => (
                      <Table.Cell
                        onClick={(e) => {
                          e.stopPropagation()
                          onClick?.(d)
                        }}
                        justify={justify}
                        // eslint-disable-next-line react/no-array-index-key
                        key={c.key + index}
                      >
                        {c.render ? c.render(c.dataIndex[0], d, index) : null}
                      </Table.Cell>
                    ))}
                  </Table.Row>
                )
              } else {
                return null
              }
            })}
          </Table.Body>
        </Table.Root>
      </Spinner>
    </>
  )
}

export default table
