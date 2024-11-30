import Link from 'next/link'

import clsx from 'clsx'

export function GitHubLink({ color = 'black' }) {
  return (
    <Link
      href="https://github.com/web3infra-foundation/mega"
      aria-label="Visit GitHub Repository"
      className={clsx(
        'rounded-lg transition-colors',
        color === 'black'
          ? 'bg-gray-800 text-white hover:bg-gray-900'
          : 'bg-white text-gray-900 hover:bg-gray-50',
      )}
    >
      <svg viewBox="0 0 120 40" aria-hidden="true" className="h-10">
        <path transform="translate(10,-19) scale(1.2)" fill="currentColor" d="M12.769 20.301a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2.2c-3.3.7-4-1.6-4-1.6-.5-1.4-1.3-1.8-1.3-1.8-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0c2.3-1.5 3.3-1.2 3.3-1.2.7 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6a12 12 0 0 0-3.8-23.4z"/>
        <text x="48" y="27" fill="currentColor" fontSize="20" fontWeight="bold" fontFamily="system-ui">MEGA</text>
      </svg>

    </Link>
  )
}
