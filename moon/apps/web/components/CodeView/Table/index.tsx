import { Table } from '@radix-ui/themes'

import { columnsType, DirectoryType } from './type'

const table = <T extends DirectoryType>({
  columns,
  datasource,
  size,
  align,
  justify,
  onClick
}: {
  columns: columnsType<T>[]
  datasource: T[]
  size?: '1' | '2' | '3' | undefined
  align?: 'center' | 'start' | 'end' | undefined
  justify?: 'center' | 'start' | 'end' | undefined
  onClick?: (record: T) => void
}) => {
  return (
    <>
      <Table.Root size={size}>
        <Table.Header>
          <Table.Row align={align}>
            {columns.map((c) => (
              <>
                <Table.ColumnHeaderCell>{c.title}</Table.ColumnHeaderCell>
              </>
            ))}
          </Table.Row>
        </Table.Header>

        <Table.Body>
          {datasource.map((d) => {
            if (d) {
              return (
                <Table.Row className='hover:bg-gray-100' key={d.oid}>
                  {columns.map((c, index) => (
                    <Table.Cell
                      onClick={(e) => {
                        e.stopPropagation()
                        onClick?.(d)
                      }}
                      justify={justify}
                      key={c.key}
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
    </>
  )
}

export default table
