"use client";
// show helo world
import React, { useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";

export default function HelloRust({ ...props }) {
  const [content, setContent] = React.useState<string>("数据加载中...");
  invoke("get_list");
    useEffect(() => {
      invoke("hello_string", { name: "lunar" }).then((response: string) => {
        setContent(response);
      });
    }, []);
  return <div style={{ textAlign: "center" }}>{content}</div>;
}
