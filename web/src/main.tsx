import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import './styles/tokens.css';
import './styles/globals.css';
import { initializeLanguage } from './i18n/Settings';

const root = document.getElementById('root');
if (!root) throw new Error('H3Forge root element is missing.');
void initializeLanguage().then(() => createRoot(root).render(<StrictMode><App /></StrictMode>));
