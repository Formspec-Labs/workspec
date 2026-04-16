import {StrictMode} from 'react';
import {createRoot} from 'react-dom/client';
import App from './App.tsx';
import './index.css';
import { WosProvider } from './context/WosContext.tsx';
import { ToastProvider } from './context/ToastContext.tsx';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <WosProvider>
      <ToastProvider>
        <App />
      </ToastProvider>
    </WosProvider>
  </StrictMode>,
);
