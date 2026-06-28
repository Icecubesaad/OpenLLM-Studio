import { useState, useMemo } from 'react';
import { useHfSearch } from '../hooks/useHfSearch';
import { useHfModelFiles } from '../hooks/useHfModelFiles';
import { useHfDownload } from '../hooks/useHfDownload';
import { useHardwareInfo } from '../hooks/useHardwareInfo';
import { useAppStore } from '../store/appStore';
import { HfSearchBar } from './HfSearchBar';
import { HfModelCard } from './HfModelCard';
import { HfModelFileList } from './HfModelFileList';
import { DownloadQueue } from './DownloadQueue';
import { ErrorBanner } from './ErrorBanner';
import { LoadingSpinner } from './LoadingSpinner';
import type { HfModel } from '../lib/types';

type ModelSize = 'all' | 'small' | 'medium' | 'large';
type QuantizationType = 'all' | 'Q2' | 'Q4' | 'Q5' | 'Q8';
type ModelFamily = 'all' | 'llama' | 'mistral' | 'gemma' | 'qwen' | 'other';

export function HuggingFaceTab() {
  const { hfToken } = useAppStore();
  const [selectedModel, setSelectedModel] = useState<HfModel | null>(null);
  const [sizeFilter, setSizeFilter] = useState<ModelSize>('all');
  const [quantizationFilter, setQuantizationFilter] = useState<QuantizationType>('all');
  const [familyFilter, setFamilyFilter] = useState<ModelFamily>('all');
  
  const { searchResults, isSearching, error: searchError, searchQuery, activeSearchQuery, setQuery, searchModels, loadMore, hasMore } = useHfSearch();
  const { modelFiles, modelDetails, isLoading: isLoadingFiles, error: filesError, loadFiles } = useHfModelFiles();
  const { downloads, downloadModel, pauseDownload, resumeDownload, cancelDownload } = useHfDownload();
  const { hardwareInfo: detectedHardware } = useHardwareInfo();
  const { installedModels } = useAppStore();

  // Filter search results based on selected filters
  const filteredResults = useMemo(() => {
    return searchResults.filter((model) => {
      // Size filter - extract parameter count from model ID or tags
      if (sizeFilter !== 'all') {
        const paramMatch = model.id.match(/(\d+\.?\d*)b/i) || model.tags.find(t => t.match(/(\d+\.?\d*)b/i))?.match(/(\d+\.?\d*)b/i);
        if (paramMatch) {
          const params = parseFloat(paramMatch[1]);
          if (sizeFilter === 'small' && params >= 3) return false;
          if (sizeFilter === 'medium' && (params < 3 || params > 10)) return false;
          if (sizeFilter === 'large' && params <= 10) return false;
        } else {
          // If we can't determine size, exclude from size-specific filters
          return false;
        }
      }

      // Quantization filter - check if model ID or tags contain quantization type
      if (quantizationFilter !== 'all') {
        const modelText = `${model.id} ${model.tags.join(' ')}`.toLowerCase();
        const hasQuantization = modelText.includes(quantizationFilter.toLowerCase());
        if (!hasQuantization) return false;
      }

      // Family filter - check model ID for family name
      if (familyFilter !== 'all') {
        const modelIdLower = model.id.toLowerCase();
        const tagsLower = model.tags.map(t => t.toLowerCase()).join(' ');
        const searchText = `${modelIdLower} ${tagsLower}`;
        
        if (familyFilter === 'llama' && !searchText.includes('llama')) return false;
        if (familyFilter === 'mistral' && !searchText.includes('mistral')) return false;
        if (familyFilter === 'gemma' && !searchText.includes('gemma')) return false;
        if (familyFilter === 'qwen' && !searchText.includes('qwen')) return false;
        if (familyFilter === 'other') {
          // "Other" means none of the main families
          if (searchText.includes('llama') || searchText.includes('mistral') || 
              searchText.includes('gemma') || searchText.includes('qwen')) {
            return false;
          }
        }
      }

      return true;
    });
  }, [searchResults, sizeFilter, quantizationFilter, familyFilter]);

  const handleSelectModel = (model: HfModel) => {
    setSelectedModel(model);
    loadFiles(model.id);
  };

  const handleDownload = async (filename: string) => {
    if (!selectedModel) return;

    // Check if already downloading - if so, cancel
    const modelName = deriveModelName(selectedModel.id, filename);
    const existingDownload = downloads.get(modelName);
    
    if (existingDownload) {
      try {
        await cancelDownload(modelName);
      } catch (err) {
        console.error('Failed to cancel download:', err);
      }
      return;
    }

    // Public HuggingFace models do not need a token, but gated/private repos still do.
    if (modelRequiresHfToken(selectedModel, modelDetails) && !hfToken) {
      alert('This model requires a HuggingFace token. Public models can be downloaded without one.');
      return;
    }

    // Start download
    try {
      await downloadModel(selectedModel.id, filename, modelName);
    } catch (err) {
      console.error('Failed to start download:', err);
      alert(`Download failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleBackToSearch = () => {
    setSelectedModel(null);
  };

  const handleLoadMore = () => {
    if (!isSearching && hasMore) {
      loadMore();
    }
  };

  const selectedModelRequiresToken = selectedModel
    ? modelRequiresHfToken(selectedModel, modelDetails)
    : false;

  return (
    <div className="flex flex-col h-full">
      {/* Search bar */}
      <div className="mb-4">
        <HfSearchBar
          searchQuery={searchQuery}
          onChange={setQuery}
          onSearch={() => searchModels()}
          isSearching={isSearching}
          autoFocus={!selectedModel}
        />
      </div>

      {!hfToken && (
        <div className="mb-4 rounded-lg border border-dark-border bg-dark-surface p-3">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <p className="text-sm font-medium text-dark-text">Browsing public HuggingFace models</p>
              <p className="text-xs text-dark-text-secondary">
                A token is only needed for gated or private repositories.
              </p>
            </div>
            <span className="w-fit rounded-full bg-green-600/15 px-2.5 py-1 text-xs font-medium text-green-400">
              No token required
            </span>
          </div>
        </div>
      )}

      {/* Filter controls - only show when not viewing a specific model */}
      {!selectedModel && activeSearchQuery && (
        <div className="mb-4 p-4 bg-dark-surface border border-dark-border rounded-lg">
          <div className="flex flex-wrap gap-4">
            {/* Model Size Filter */}
            <div className="flex-1 min-w-[200px]">
              <label htmlFor="size-filter" className="block text-sm font-medium text-dark-text mb-2">
                Model Size
              </label>
              <select
                id="size-filter"
                value={sizeFilter}
                onChange={(e) => setSizeFilter(e.target.value as ModelSize)}
                className="w-full bg-dark-bg border border-dark-border rounded px-3 py-2 text-dark-text focus:outline-none focus:ring-2 focus:ring-dark-accent"
              >
                <option value="all">All Sizes</option>
                <option value="small">Small (&lt;3B)</option>
                <option value="medium">Medium (3-10B)</option>
                <option value="large">Large (&gt;10B)</option>
              </select>
            </div>

            {/* Quantization Filter */}
            <div className="flex-1 min-w-[200px]">
              <label htmlFor="quantization-filter" className="block text-sm font-medium text-dark-text mb-2">
                Quantization
              </label>
              <select
                id="quantization-filter"
                value={quantizationFilter}
                onChange={(e) => setQuantizationFilter(e.target.value as QuantizationType)}
                className="w-full bg-dark-bg border border-dark-border rounded px-3 py-2 text-dark-text focus:outline-none focus:ring-2 focus:ring-dark-accent"
              >
                <option value="all">All Types</option>
                <option value="Q2">Q2 (Smallest)</option>
                <option value="Q4">Q4 (Balanced)</option>
                <option value="Q5">Q5 (High Quality)</option>
                <option value="Q8">Q8 (Best Quality)</option>
              </select>
            </div>

            {/* Model Family Filter */}
            <div className="flex-1 min-w-[200px]">
              <label htmlFor="family-filter" className="block text-sm font-medium text-dark-text mb-2">
                Model Family
              </label>
              <select
                id="family-filter"
                value={familyFilter}
                onChange={(e) => setFamilyFilter(e.target.value as ModelFamily)}
                className="w-full bg-dark-bg border border-dark-border rounded px-3 py-2 text-dark-text focus:outline-none focus:ring-2 focus:ring-dark-accent"
              >
                <option value="all">All Families</option>
                <option value="llama">Llama</option>
                <option value="mistral">Mistral</option>
                <option value="gemma">Gemma</option>
                <option value="qwen">Qwen</option>
                <option value="other">Other</option>
              </select>
            </div>
          </div>

          {/* Filtered result count */}
          {searchResults.length > 0 && (
            <div className="mt-3 text-sm text-dark-text-secondary">
              Showing {filteredResults.length} of {searchResults.length} results
            </div>
          )}
        </div>
      )}

      {/* Download queue - shows active downloads */}
      {downloads.size > 0 && (
        <DownloadQueue
          downloads={downloads}
          onPause={pauseDownload}
          onResume={resumeDownload}
          onCancel={cancelDownload}
        />
      )}

      {/* Error display */}
      {searchError && (
        <div className="mb-4">
          <ErrorBanner message={searchError} onRetry={() => searchModels()} />
        </div>
      )}

      {filesError && (
        <div className="mb-4">
          <ErrorBanner message={filesError} onRetry={() => selectedModel && loadFiles(selectedModel.id)} />
        </div>
      )}

      {/* Content area */}
      <div className="flex-1 overflow-y-auto">
        {selectedModel ? (
          <div>
            {/* Back button and model info */}
            <div className="mb-4">
              <button
                onClick={handleBackToSearch}
                onKeyDown={(e) => {
                  if (e.key === 'Escape') {
                    handleBackToSearch();
                  }
                }}
                className="flex items-center gap-2 text-dark-accent hover:text-dark-accent-hover transition-colors focus:outline-none focus:ring-2 focus:ring-dark-accent rounded px-2 py-1 -ml-2"
                aria-label="Back to search results"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
                Back to search
              </button>
              
              <h2 className="text-xl font-semibold text-dark-text mb-2">{selectedModel.id}</h2>
              {modelDetails && (
                <div className="mb-6 rounded-xl border border-dark-border bg-dark-surface p-4 space-y-3">
                  <div className="flex flex-wrap items-center gap-2 text-xs text-dark-text-secondary">
                    {modelDetails.author && <span>By {modelDetails.author}</span>}
                    {modelDetails.pipeline_tag && <span className="px-2 py-1 rounded-full bg-dark-bg">{modelDetails.pipeline_tag}</span>}
                    {modelDetails.license && <span className="px-2 py-1 rounded-full bg-dark-bg">{modelDetails.license}</span>}
                    <span>{modelDetails.downloads.toLocaleString()} downloads</span>
                    <span>{modelDetails.likes.toLocaleString()} likes</span>
                  </div>
                  {modelDetails.description && (
                    <p className="text-sm text-dark-text-secondary leading-6">
                      {modelDetails.description}
                    </p>
                  )}
                </div>
              )}
              
              {selectedModelRequiresToken && !hfToken && (
                <div className="bg-yellow-900/20 border border-yellow-800 rounded-lg p-3 mb-4" role="alert">
                  <div className="flex items-start gap-2">
                    <svg className="w-5 h-5 text-yellow-500 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                    </svg>
                    <div>
                      <p className="text-dark-text text-sm font-medium">Authentication Required</p>
                      <p className="text-dark-text-secondary text-sm">
                        This model is gated or private. Add a HuggingFace token in Settings to download it.
                      </p>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* File list */}
            {isLoadingFiles ? (
              <div className="flex justify-center py-12">
                <LoadingSpinner size="lg" />
              </div>
            ) : (
              <HfModelFileList
                repoId={selectedModel.id}
                files={modelFiles}
                hardwareInfo={detectedHardware}
                onDownload={handleDownload}
                downloads={downloads}
                installedModels={installedModels}
              />
            )}
          </div>
        ) : (
          <div>
            {activeSearchQuery && !isSearching && filteredResults.length === 0 && searchResults.length === 0 ? (
              <div className="text-center py-12 text-dark-text-secondary">
                No models found for "{activeSearchQuery}"
              </div>
            ) : activeSearchQuery && !isSearching && filteredResults.length === 0 && searchResults.length > 0 ? (
              <div className="text-center py-12 text-dark-text-secondary">
                No models match the selected filters
              </div>
            ) : filteredResults.length > 0 ? (
              <>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {filteredResults.map((model) => (
                    <HfModelCard
                      key={model.id}
                      model={model}
                      onSelect={() => handleSelectModel(model)}
                      isSelected={false}
                    />
                  ))}
                </div>
                
                {/* Load More button */}
                {hasMore && (
                  <div className="mt-6 flex justify-center">
                    <button
                      onClick={handleLoadMore}
                      disabled={isSearching}
                      className="px-6 py-3 bg-dark-accent text-white rounded-lg hover:bg-dark-accent-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-dark-accent"
                    >
                      {isSearching ? (
                        <span className="flex items-center gap-2">
                          <LoadingSpinner size="sm" />
                          Loading...
                        </span>
                      ) : (
                        'Load More'
                      )}
                    </button>
                  </div>
                )}
              </>
            ) : !activeSearchQuery ? (
              <div className="text-center py-12 text-dark-text-secondary">
                <svg className="w-16 h-16 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
                <p className="text-lg mb-2">Search HuggingFace Models</p>
                <p className="text-sm">
                  {searchQuery.trim() ? `Press Search to look up "${searchQuery.trim()}"` : 'Enter a search query to find GGUF models'}
                </p>
              </div>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
}

function deriveModelName(repoId: string, filename: string): string {
  const baseName = repoId.split('/').pop() || repoId;
  const quantization = filename.match(/Q\d+_K_[MSL]|Q\d+_\d+|Q\d+|F\d+/i)?.[0] || '';
  return `${baseName}-${quantization}`.toLowerCase().replace(/[^a-z0-9-]/g, '-');
}

function modelRequiresHfToken(model: HfModel, details?: { tags: string[] } | null): boolean {
  const tags = [...model.tags, ...(details?.tags ?? [])].map((tag) => tag.toLowerCase());
  return tags.some((tag) => tag === 'gated' || tag === 'private');
}
