import { SaveOverviewPage } from './pages/SaveOverviewPage';
import { useTranslation } from 'react-i18next';

export function App() {
  useTranslation();
  return <SaveOverviewPage />;
}
