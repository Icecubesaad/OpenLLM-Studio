import { MyLibrary } from './MyLibrary';
import { ModelLibrary } from './ModelLibrary';
import { HuggingFaceTab } from './HuggingFaceTab';

export type ModelLibraryTab = 'my-library' | 'browse' | 'huggingface';

interface ModelLibraryTabsProps {
  activeTab: ModelLibraryTab;
  onTabChange: (tab: ModelLibraryTab) => void;
}

export function ModelLibraryTabs({ activeTab, onTabChange }: ModelLibraryTabsProps) {
  return (
    <div className="flex flex-col h-full">
      {/* Tab navigation */}
      <div className="flex border-b border-dark-border bg-dark-bg-secondary">
        <button
          onClick={() => onTabChange('my-library')}
          className={`px-6 py-3 font-medium transition-all relative ${
            activeTab === 'my-library'
              ? 'text-dark-accent'
              : 'text-dark-text-secondary hover:text-dark-text'
          }`}
        >
          <div className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
            </svg>
            My Library
          </div>
          {activeTab === 'my-library' && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-dark-accent" />
          )}
        </button>
        <button
          onClick={() => onTabChange('browse')}
          className={`px-6 py-3 font-medium transition-all relative ${
            activeTab === 'browse'
              ? 'text-dark-accent'
              : 'text-dark-text-secondary hover:text-dark-text'
          }`}
        >
          <div className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            Browse
          </div>
          {activeTab === 'browse' && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-dark-accent" />
          )}
        </button>
        <button
          onClick={() => onTabChange('huggingface')}
          className={`px-6 py-3 font-medium transition-all relative ${
            activeTab === 'huggingface'
              ? 'text-dark-accent'
              : 'text-dark-text-secondary hover:text-dark-text'
          }`}
        >
          <div className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
            </svg>
            HuggingFace
          </div>
          {activeTab === 'huggingface' && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-dark-accent" />
          )}
        </button>
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-hidden">
        {activeTab === 'my-library' ? (
          <div className="h-full overflow-y-auto">
            <MyLibrary onBrowseModels={() => onTabChange('browse')} />
          </div>
        ) : activeTab === 'browse' ? (
          <div className="h-full overflow-y-auto">
            <ModelLibrary />
          </div>
        ) : (
          <div className="h-full overflow-y-auto p-6">
            <HuggingFaceTab />
          </div>
        )}
      </div>
    </div>
  );
}
