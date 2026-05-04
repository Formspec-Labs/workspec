import {StrictMode} from 'react';
import {createRoot} from 'react-dom/client';
import App from './App.tsx';
import './index.css';
import { WosProvider } from './context/WosContext.tsx';

if (!import.meta.env.VITE_AI_ENABLED) {
  console.warn('[WOS Case Portal] AI features disabled — GEMINI_API_KEY not configured on server');
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <WosProvider>
      <App />
    </WosProvider>
  </StrictMode>,
);
