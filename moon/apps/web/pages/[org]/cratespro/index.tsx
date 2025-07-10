import Head from 'next/head'
 import { useRouter } from 'next/router'
import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const Logo = () => (
  <div className="flex items-center">
    <ChatBubblePlusIcon className="h-8 w-8 mr-2 text-orange-500" />
    <span className="text-xl font-bold">Rust</span>
  </div>
)

const MagnifierIcon = () => (
  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" strokeWidth="2" viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" /><line x1="21" y1="21" x2="16.65" y2="16.65" /></svg>
)

const cardList = [
  {
    key: 'distribth',
    title: 'Rust Distribth',
    icon: (
      <svg width="48" height="48" fill="none" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" stroke="#222" strokeWidth="2" /><circle cx="12" cy="12" r="2" fill="#222" /><circle cx="5" cy="5" r="1.5" fill="#222" /><circle cx="19" cy="5" r="1.5" fill="#222" /><circle cx="5" cy="19" r="1.5" fill="#222" /><circle cx="19" cy="19" r="1.5" fill="#222" /><line x1="12" y1="12" x2="5" y2="5" stroke="#222" strokeWidth="1.5" /><line x1="12" y1="12" x2="19" y2="5" stroke="#222" strokeWidth="1.5" /><line x1="12" y1="12" x2="5" y2="19" stroke="#222" strokeWidth="1.5" /><line x1="12" y1="12" x2="19" y2="19" stroke="#222" strokeWidth="1.5" /></svg>
    ),
    bg: 'from-pink-50 to-white',
  },
  {
    key: 'ecosystem',
    title: 'Crate Ecosystem',
    icon: (
      <svg width="48" height="48" fill="none" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" stroke="#222" strokeWidth="2" /><path d="M8 15l4-4 4 4" stroke="#222" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /><path d="M12 11V7" stroke="#222" strokeWidth="1.5" strokeLinecap="round" /></svg>
    ),
    bg: 'from-yellow-50 to-white',
  },
  {
    key: 'tour',
    title: 'Rust Tour & Doc',
    icon: (
      <svg width="48" height="48" fill="none" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" stroke="#222" strokeWidth="2" /><path d="M8 16l4-8 4 8" stroke="#222" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
    ),
    bg: 'from-pink-50 to-white',
  },
  {
    key: 'news',
    title: 'Rust News',
    icon: (
      <svg width="48" height="48" fill="none" viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2" stroke="#222" strokeWidth="2" /><rect x="7" y="7" width="5" height="3" rx="1" fill="#222" /><rect x="7" y="12" width="10" height="2" rx="1" fill="#222" /><rect x="7" y="16" width="7" height="1.5" rx="0.75" fill="#222" /></svg>
    ),
    bg: 'from-blue-50 to-white',
  },
]

const CratesproPage: PageWithLayout<any> = () => {
  const router = useRouter()

  // Navigation function
  const handleCardClick = (key: string) => {
    const routes: { [key: string]: string } = {
      news: '/news', // Example route for 'news'
      // Add more routes as needed
    }
    if (routes[key]) {
      router.push(routes[key])
    } else {
      alert(`No route defined for ${key}`)
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
              <span className="absolute inset-y-0 left-3 flex items-center pointer-events-none">
                <MagnifierIcon />
              </span>
              <input
                type="text"
                placeholder="Search..."
                className="pl-10 pr-4 py-2 w-full rounded-full border border-gray-200 text-base focus:outline-none focus:ring-2 focus:ring-blue-200 bg-white shadow-sm"
                style={{ minWidth: 200 }}
              />
            </div>
          </div>
          <div className="border-b border-gray-200 w-full" />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-10 w-full max-w-5xl mx-auto mt-36">
            {cardList.map(card => (
              <button
                key={card.key}
                onClick={() => handleCardClick(card.key)}
                className={`flex flex-col items-start justify-center h-64 rounded-xl border bg-gradient-to-br ${card.bg} shadow-md p-8 transition hover:scale-105 hover:shadow-lg focus:outline-none`}
              >
                <div className="mb-6">{card.icon}</div>
                <span className="text-3xl font-display font-medium text-gray-800">{card.title}</span>
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