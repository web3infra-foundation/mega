import React from 'react';
import { Card } from 'antd/lib';

const MRDetailPage = ({ mrDetail }) => (
  <Card title="Card title">
    <Card
      style={{ marginTop: 16 }}
      type="inner"
      title="Inner Card title"
      extra={<a href="#">More</a>}
    >
      Inner Card content
      {mrDetail.status}
    </Card>
  </Card>
);

export default MRDetailPage;