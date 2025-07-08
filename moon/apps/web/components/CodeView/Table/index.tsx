import { Table } from '@radix-ui/themes'
import { columnsType, DirectoryType } from './type'
import { Skeleton } from '@mui/material';
import { useMemo } from 'react';

const TableComponent = <T extends DirectoryType>({
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
  // 使用useMemo缓存列配置，只有当columns数组变化时才重新计算
  const memoizedColumns = useMemo(() => columns,[columns]);

  return (
    <Table.Root size={size}>
      <Table.Header>
        <Table.Row align={align}>
          {memoizedColumns.map((c) => (
            <Table.ColumnHeaderCell key={c.title}>{c.title}</Table.ColumnHeaderCell>
          ))}
        </Table.Row>
      </Table.Header>

      <Table.Body>
        {loading ? (
          // 骨架屏行
          Array.from({ length: 5 }).map((_, rowIndex) => {
            const uniqueKey = `skeleton-row-${rowIndex}`; // 生成唯一 key
            
            return (
              <Table.Row key={uniqueKey}>
                {memoizedColumns.map((column) => (
                  <Table.Cell key={column.key}>
                    <Skeleton variant="rounded" height={16} width="100%" />
                  </Table.Cell>
                ))}
              </Table.Row>
            );
          })
        ) : datasource.length > 0 && (
          // 实际数据行
          datasource.map((d, index) => (

            <Table.Row className='hover:bg-gray-100' key={d?.id || index}>
              {memoizedColumns.map((c) => (
                <Table.Cell
                  onClick={(e) => {
                    e.stopPropagation();
                    onClick?.(d);
                  }}
                  justify={justify}
                  key={c.key || c.title}
                >
                  {c.render ? c.render(c.dataIndex[0], d, index) : null}
                </Table.Cell>
              ))}
            </Table.Row>
          ))
        )}
      </Table.Body>
    </Table.Root>
  );
};

export default TableComponent;