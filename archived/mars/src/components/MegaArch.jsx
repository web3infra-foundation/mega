'use client'

function Chart() {
  return (
    <svg viewBox="0 0 800 600" xmlns="http://www.w3.org/2000/svg">
      <circle cx="350" cy="300" r="150" fill="#bbdefb" />
      <circle cx="500" cy="300" r="150" fill="#2196f3" />
      <circle cx="350" cy="300" r="150" fill="#f5f9ff" opacity="0.9" />
      <circle cx="500" cy="300" r="150" fill="#f5f9ff" opacity="0.9" />
      <circle cx="350" cy="300" r="150" fill="none" stroke="#1976d2" strokeWidth="2" />
      <circle cx="500" cy="300" r="150" fill="none" stroke="#2196f3" strokeWidth="2" />
      <text x="270" y="290" textAnchor="middle" fontSize="16" fontWeight="bold" fill="#333333">
        Monorepo
      </text>
      <text x="270" y="310" textAnchor="middle" fontSize="12" fill="#333333">
        Monolithic
      </text>
      <text x="580" y="290" textAnchor="middle" fontSize="16" fontWeight="bold" fill="#333333">
        Decentralized
      </text>
      <text x="580" y="310" textAnchor="middle" fontSize="12" fill="#333333">
        Collaboration
      </text>
      <g transform="translate(385,215)">
        <rect width="80" height="35" rx="5" fill="#4caf50" opacity="0.8" />
        <text x="40" y="22" textAnchor="middle" fontSize="10" fill="white">Git Compatible</text>
      </g>
      <g transform="translate(385,260)">
        <rect width="80" height="35" rx="5" fill="#ff9800" opacity="0.8" />
        <text x="40" y="17" textAnchor="middle" fontSize="10" fill="white">
          <tspan x="40" y="17">Trunk-based</tspan>
          <tspan x="40" y="29">Development</tspan>
        </text>
      </g>
      <g transform="translate(385,305)">
        <rect width="80" height="35" rx="5" fill="#e91e63" opacity="0.8" />
        <text x="40" y="22" textAnchor="middle" fontSize="10" fill="white">Code Owners</text>
      </g>
      <g transform="translate(385,350)">
        <rect width="80" height="35" rx="5" fill="#9c27b0" opacity="0.8" />
        <text x="40" y="17" textAnchor="middle" fontSize="10" fill="white">
          <tspan x="40" y="17">Conventional</tspan>
          <tspan x="40" y="29">Commits</tspan>
        </text>
      </g>
    </svg>
  )
}

export function MegaArch() {
  return (
    <div>
      <Chart />
    </div>
  )
}
