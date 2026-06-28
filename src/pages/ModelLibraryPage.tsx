import { useState } from 'react';
import { CustomTitleBar } from '../components/CustomTitleBar';
import { ModelStatusIndicator } from '../components/ModelStatusIndicator';
import { ModelLibraryTabs, type ModelLibraryTab } from '../components/ModelLibraryTabs';
import { ModelWizard } from '../components/ModelWizard';

export function ModelLibraryPage() {
  const [showWizard, setShowWizard] = useState(false);
  const [activeTab, setActiveTab] = useState<ModelLibraryTab>('my-library');

  const browseModelsManually = () => {
    setShowWizard(false);
    setActiveTab('browse');
  };

  return (
    <div className="flex flex-col h-screen bg-dark-bg">
      <CustomTitleBar />
      
      <div className="flex items-center justify-between px-8 py-4 border-b border-dark-border bg-dark-bg-secondary">
        <div>
          <h1 className="text-2xl font-bold text-dark-text mb-1">Model Library</h1>
          <p className="text-dark-text-secondary text-sm">Manage and discover AI models</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowWizard(true)}
            className="px-5 py-2.5 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg transition-all flex items-center gap-2 shadow-glow-sm hover:shadow-glow"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
            </svg>
            <span className="font-medium">Model Wizard</span>
          </button>
          <ModelStatusIndicator />
        </div>
      </div>
      
      <div className="flex-1 overflow-hidden">
        <ModelLibraryTabs activeTab={activeTab} onTabChange={setActiveTab} />
      </div>

      {showWizard && (
        <div className="fixed inset-x-0 top-10 bottom-0 bg-black bg-opacity-60 flex items-center justify-center z-50 p-4">
          <div className="bg-dark-bg rounded-2xl shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-hidden border border-dark-border-light">
            <div className="sticky top-0 bg-dark-bg-secondary border-b border-dark-border px-6 py-4 flex items-center justify-between">
              <div>
                <h2 className="text-xl font-semibold text-dark-text">Model Setup Wizard</h2>
                <p className="text-dark-text-secondary text-sm mt-0.5">Hardware-aware model recommendations</p>
              </div>
              <button
                onClick={() => setShowWizard(false)}
                className="text-dark-text-secondary hover:text-dark-text hover:bg-dark-surface rounded-lg p-2 transition-all"
              >
                <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
            <div className="overflow-y-auto max-h-[calc(90vh-80px)]">
              <ModelWizard
                onClose={() => setShowWizard(false)}
                onBrowseManually={browseModelsManually}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
