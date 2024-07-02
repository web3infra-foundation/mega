import React from 'react';
import { Button, Result } from 'antd/lib';

const App: React.FC = () => (
  <Result
    status="404"
    title="404"
    subTitle="Sorry, the page you visited does not exist."
    extra={<Button type="primary" href='/' >Back Home</Button>}
  />
);

export default App;