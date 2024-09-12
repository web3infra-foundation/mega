'use client'

import { Divider } from '@/components/catalyst/divider'
import { Heading, Subheading } from '@/components/catalyst/heading'
import { useState } from 'react'
import { Input, Button, message } from "antd";
import { useRouter } from 'next/navigation';

const { TextArea } = Input;

export default function Settings() {
  const [title, setTitle] = useState('');
  const [ssh_key, setSSHKey] = useState('');
  const [messageApi, contextHolder] = message.useMessage();
  const router = useRouter();

  const error = () => {
    messageApi.open({
      type: 'error',
      content: 'Invalid SSH key format',
    });
  };

  const save_ssh_key = async (title, ssh_key) => {
    const res = await fetch('/api/user/ssh', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        title: title,
        ssh_key: ssh_key
      }),
    });
    if (!res.ok) {
      error()
    } else {
      router.push('/user/keys')
    }
  }

  return (
    <form method="post" className="mx-auto max-w-4xl">
      {contextHolder}
      <Heading>Add new SSH Key</Heading>
      <Divider className="my-10 mt-6" />

      <section className="grid gap-x-8 gap-y-6 sm:grid-cols-2">
        <div className="space-y-1">
          <Subheading>Title</Subheading>
        </div>
        <div>
          <Input aria-label="title" name="title" value={title} onChange={(e) => setTitle(e.target.value)}
          />
        </div>
      </section>

      <br />
      <br />
      <section className="grid gap-x-8 gap-y-6 sm:grid-cols-2">
        <div className="space-y-1">
          <Subheading>Key</Subheading>
        </div>
      </section>
      <br />

      <TextArea rows={8}
        value={ssh_key}
        onChange={(e) => setSSHKey(e.target.value)}
        placeholder="Begins with 'ssh-rsa', 'ecdsa-sha2-nistp256', 'ecdsa-sha2-nistp384', 'ecdsa-sha2-nistp521', 'ssh-ed25519', 'sk-ecdsa-sha2-nistp256@openssh.com', or 'sk-ssh-ed25519@openssh.com'" />
      <Divider className="my-10" soft />

      <div className="flex justify-end gap-4">
        <Button>
          Reset
        </Button>
        <Button type="primary" disabled={!ssh_key} onClick={() => save_ssh_key(title, ssh_key)} >Save changes</Button>
      </div>
    </form>
  )
}


