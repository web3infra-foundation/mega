'use client'

import { Divider } from '@/components/catalyst/divider'
import { Heading, Subheading } from '@/components/catalyst/heading'
import { Input } from '@/components/catalyst/input'
import { Text } from '@/components/catalyst/text'
import { invoke } from '@tauri-apps/api/core'
import { useState } from 'react'
import { Button, Skeleton, message } from "antd";
import { usePeerId } from '@/app/api/fetcher'
interface MegaStartParams {
  bootstrap_node: string,
}

export default function Settings() {

  const [messageApi, contextHolder] = message.useMessage();

  const success = () => {
    messageApi.open({
      type: 'success',
      content: 'Save setting successful',
    });
  };

  const [loadings, setLoadings] = useState<boolean[]>([]);
  const [params, setParams] = useState<MegaStartParams>({
    bootstrap_node: "http://gitmono.org/relay",
  });
  const { peerId, isLoading, isError } = usePeerId();
  if (isLoading) return <Skeleton />;

  const enterLoading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings];
      newLoadings[index] = true;
      return newLoadings;
    });
  }

  const exitLoading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings];
      newLoadings[index] = false;
      return newLoadings;
    });
  }

  const stopMega = async () => {
    invoke('stop_mega_service', { params: params })
      .then((message) => console.log("result:", message))
      .catch((err) => console.error("err:", err));
  };

  const restartMega = async () => {
    enterLoading(1);
    invoke('restart_mega_service', { params: params })
      .then((message) => {
        console.log("result:", message);
        success()
      })
      .catch((err) => console.error("err:", err));

    setTimeout(() => {
      exitLoading(1);
    }, 1000);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setParams((prevParams) => ({
      ...prevParams,
      [name]: value,
    }));
  };

  return (
    <form method="post" className="mx-auto max-w-4xl">
      {contextHolder}
      <Heading>Settings</Heading>
      <Divider className="my-10 mt-6" />

      <section className="grid gap-x-8 gap-y-6 sm:grid-cols-2">
        <div className="space-y-1">
          <Subheading>ZTM Server IP Address</Subheading>
          <Text>Reboot Mega service after saving.</Text>
        </div>
        <div>
          <Input disabled={loadings[1]} aria-label="Bootstrap Node" name="bootstrap_node"
            value={params.bootstrap_node}
            onChange={handleInputChange}
          />
        </div>
      </section>

      <Divider className="my-10" soft />

      <section className="grid gap-x-8 gap-y-6 sm:grid-cols-2">
        <div className="space-y-1">
          <Subheading>ZTM Agent Peer Id</Subheading>
        </div>
        <div>
          <Input disabled={true} aria-label="Peer Id" name="peer_id"
            value={peerId.data}
          />
        </div>
      </section>

      <Divider className="my-10" soft />


      <div className="flex justify-end gap-4">
        <Button>
          Reset
        </Button>
        <Button type="primary" loading={loadings[1]} onClick={restartMega} >Save changes</Button>
      </div>
    </form>
  )
}
