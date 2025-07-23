import Head from 'next/head'
import { useRouter } from 'next/router'
// import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'
import { IndexSearchInput } from '@/components/IndexPages/components'
import { useState } from 'react'
import Image from 'next/image'

const Logo = () => (
  <div className="flex items-center">
    <Image src="/rust/logo.png" alt="Rust Logo" width={24} height={24} className="mr-2 relative" style={{ opacity: 0.8, top: '-2px' }} />
    <span className="text-lg font-bold">Rust</span>
  </div>
)


const cardList = [
  {
    key: 'distribth',
    title: 'Rust Distribute',
    icon: '/rust/Rust-Distribth.png',
    bgStyle: 'bg-gradient-to-br from-[#FFF0F5] to-[#FFFFFF]',
  },
  {
    key: 'ecosystem',
    title: 'Crate Ecosystem',
    icon: '/rust/Crate Ecosystem.png',
    bgStyle: 'bg-gradient-to-br from-[#FFFCF0] to-[#FFFFFF]',
  },
  {
    key: 'tour',
    title: 'Rust Tour & Doc',
    icon: '/rust/Rust-Tour-Doc.png',
    bgStyle: 'bg-gradient-to-br from-[#FFF4F0] to-[#FFFFFF]',
  },
  {
    key: 'news',
    title: 'Rust News',
    icon: '/rust/Rust-News.png',
    bgStyle: 'bg-gradient-to-br from-[#F0FBFF] to-[#FFFFFF]',
  },
]

const CratesproPage: PageWithLayout<any> = () => {
  const router = useRouter()
  const [query, setQuery] = useState('')
  const isSearchLoading = false // 如有异步搜索可改为实际loading

  const handleCardClick = (key: string) => {
    if (key === 'news') {
      // 在当前标签页跳转 rust-news 页面，保留侧边栏
      const org = router.query.org ? `/${router.query.org}` : ''

      router.push(`${org}/rust/rust-news`)
    } else if (key === 'ecosystem') {
      // 跳转到 rust-ecosystem 页面
      const org = router.query.org ? `/${router.query.org}` : ''
      
      router.push(`${org}/rust/rust-ecosystem`)
    } else {
      // 其它卡片逻辑
      router.push(`/cratespro/${key}`)
      alert(`点击了 ${key}`)
    }
  }

  return (
    <>
      <Head>
        <title>Cratespro</title>
      </Head>
      <div className="flex flex-col min-h-screen w-full bg-transparent">
        {/* 顶部栏，参考 issue 主页样式 */}
        <div className="w-full flex flex-col">
          <div className="flex items-center gap-4 px-8 pt-8 pb-4 w-full">
            <Logo />
            <div className="relative flex-1 max-w-xl">
              {/* 复用calls页面的搜索栏 */}
              <IndexSearchInput query={query} setQuery={setQuery} isSearchLoading={isSearchLoading} />
            </div>
          </div>
          <div className="border-b border-gray-200 w-full" />
        </div>
        {/* 主体卡片区，居中且间距大，卡片样式按设计稿 */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-10 w-full max-w-[900px] mx-auto mt-24">
          {cardList.map(card => (
            <button
              key={card.key}
              onClick={() => handleCardClick(card.key)}
              className="flex flex-col items-start justify-start w-[420px] h-[220px] rounded-[8px] border border-[#ff337733] bg-none flex-shrink-0 transition hover:scale-105 hover:shadow-lg focus:outline-none p-8"
              style={{
                background: card.key === 'distribth' ? 'linear-gradient(180deg, #fff0f5cc 0%, #ffffffcc 100%)'
                  : card.key === 'ecosystem' ? 'linear-gradient(180deg, #fffcf0cc 0%, #ffffffcc 100%)'
                  : card.key === 'tour' ? 'linear-gradient(180deg, #fff4f0cc 0%, #ffffffcc 100%)'
                  : card.key === 'news' ? 'linear-gradient(180deg, #f0fbffcc 0%, #ffffffcc 100%)'
                  : undefined,
              }}
            >
              <Image
                src={card.icon}
                alt={card.title}
                width={80}
                height={80}
                className="w-20 h-20 opacity-80 mb-14"
              />
              <span className="text-2xl font-display font-medium text-gray-800">{card.title}</span>
            </button>
          ))}
        </div>
      </div>
    </>
  )
}

CratesproPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default CratesproPage 