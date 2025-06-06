import type { Meta, StoryObj } from '@storybook/react'

import * as Icons from './index'

interface IconProps {
  size?: number
  color?: string
  [key: string]: any
}

const meta = {
  title: 'Icons/AllIcons',
  argTypes: {
    size: {
      control: { type: 'number' },
      defaultValue: 24
    },
    color: {
      control: { type: 'color' },
      defaultValue: '#000000'
    }
  }
} satisfies Meta<IconProps>

export default meta

type Story = StoryObj<IconProps>

export const AllIcons: Story = {
  render: (args) => (
    <div className='grid grid-cols-[repeat(auto-fill,minmax(80px,1fr))] gap-6 p-6'>
      {Object.entries(Icons).map(([name, IconComponent]) => (
        <div key={name} className='flex flex-col items-center text-center text-xs'>
          <div className='text-current' style={{ color: args.color }}>
            <IconComponent size={args.size} />
          </div>
          <div className='mt-2 select-text'>{name}</div>
        </div>
      ))}
    </div>
  )
}
