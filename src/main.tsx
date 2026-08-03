import React from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import CreatorWindow from './windows/Creator';
import PetWindow from './windows/Pet';

async function main() {
  const label = (await getCurrentWindow()).label;
  const Component = label === 'pet' ? PetWindow : CreatorWindow;

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <Component />
    </React.StrictMode>
  );
}

main();
