'use client'

import React from 'react';

export default function FilesChanged({ outputHtml }: { outputHtml: Readonly<string> }) {
  return (
    <div className="w-full">
      <div
        dangerouslySetInnerHTML={{ __html: outputHtml }}
        style={{ fontFamily: 'monospace' }}
      />
    </div>
  )
}