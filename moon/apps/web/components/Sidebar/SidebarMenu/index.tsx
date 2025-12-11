import { useMemo } from 'react'

import { SidebarGroup } from '@/components/Sidebar/SidebarGroup'
import { DynamicSidebarItem } from '@/components/Sidebar/SidebarMenu/DynamicSidebarItem'
import { useGetSidebarList } from '@/hooks/Sidebar/useGetSidebarList'

export function SidebarMenu() {
  const { data } = useGetSidebarList()

  const menuItems = useMemo(() => {
    const items = data?.data || []

    return items.filter((item) => item.visible !== false).sort((a, b) => (a.order_index || 0) - (b.order_index || 0))
  }, [data])

  return (
    <SidebarGroup className='pt-0'>
      {menuItems.map((menuItem) => (
        <DynamicSidebarItem key={menuItem.id} config={menuItem} />
      ))}
    </SidebarGroup>
  )
}
