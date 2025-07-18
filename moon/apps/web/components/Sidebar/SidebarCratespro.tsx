// import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'
import { useRouter } from 'next/router'
import { useScope } from '@/contexts/scope'
import { SidebarLink } from './SidebarLink'
import Image from 'next/image'

export function SidebarCratespro() {
  const { scope } = useScope()
  const router = useRouter()

  return (
    <SidebarLink
      id='rust'
      label='Rust'
      href={`/${scope}/rust`}
      active={router.pathname === '/[org]/rust'}
      leadingAccessory={
        <Image
          src="/rust/Rust-Tour-Doc.png"
          alt="Rust Logo"
          width={21}
          height={21}
          // 如果还需要自定义样式可以加 className 或 style
        />
      }
    />
  )
}