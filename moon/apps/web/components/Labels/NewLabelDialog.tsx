// components/Labels/NewLabelDialog.tsx
import React, { useState } from 'react';
import { Button, Dialog, TextField, RefreshIcon } from '@gitmono/ui';
import { colord, random } from 'colord';

interface NewLabelDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onCreateLabel: (name: string, description: string, color: string) => void;
}

export const NewLabelDialog: React.FC<NewLabelDialogProps> = ({ isOpen, onClose, onCreateLabel }) => {
  const [color, setColor] = useState(random().toHex());
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');

  const isDark = colord(color).isDark();
  let fontColor = colord(color);

  if(isDark) fontColor = fontColor.lighten(0.4);
  else fontColor = fontColor.darken(0.5);

  const generateRandomColor = () => {
    setColor(random().toHex());
  };

  const handleCreateLabel = () => {
    if (name.trim()) {
      onCreateLabel(name, description, color);
      setName("")
      setDescription("")
      generateRandomColor()
      onClose();
    }
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={onClose}>
      <Dialog.Title className="p-4 w-full">
        New Label
      </Dialog.Title>
      <Dialog.Content>
        <div className="p-4 w-full max-w-md">
          {/* label preview */}
          <div className="mb-4 items-center justify-center flex">
            <div
              style={{
                backgroundColor: color,
                color: fontColor.toHex(),
                border: `1px solid ${fontColor.toHex()}`,
                borderRadius: '16px',
                padding: '2px 8px',
                fontSize: '12px',
                fontWeight: '700',
                display: 'inline-block',
                textAlign: 'center'
              }}
            >
              {name || 'label preview'}
            </div>
          </div>

          <div className="mb-4">
            <TextField
              label="Name"
              value={name}
              onChange={(e) => setName(e)}
              placeholder="Label name"
            />
          </div>

          <div className="mb-4">
            <TextField
              label="Description"
              value={description}
              onChange={(e) => setDescription(e)}
              placeholder="Optionally add a description."
            />
          </div>

          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 mb-1">Color</label>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                onClick={generateRandomColor}
                className={`flex-shrink-0 bg-[${color}]`}
              >
                <RefreshIcon className="w-4 h-4" />
              </Button>
              <div className="flex items-center gap-2 border rounded-md px-2 py-1 flex-grow">
                <input
                  className="border-none bg-transparent p-0 text-sm outline-none ring-0 focus:ring-0 flex-grow"
                  value={color}
                  onChange={(e) => setColor(e.target.value)}
                />
              </div>
            </div>
          </div>

          <div className="flex justify-end gap-2">
            <Button onClick={onClose}>
              Cancel
            </Button>
            <Button
              variant="primary"
              className="bg-[#1f883d]"
              onClick={handleCreateLabel}
              disabled={!name.trim()}
            >
              Create label
            </Button>
          </div>
        </div>
      </Dialog.Content>
    </Dialog.Root>
  );
};