import { useModels } from '../hooks/useModels';
import { useModelDelete } from '../hooks/useModelDelete';
import { useModelDownload } from '../hooks/useModelDownload';
import { ErrorBanner } from './ErrorBanner';
import { LoadingSpinner } from './LoadingSpinner';
import { formatBytes, formatSpeed } from '../lib/utils';

interface MyLibraryProps {
  onBrowseModels?: () => void;
}

export function MyLibrary({ onBrowseModels }: MyLibraryProps) {
  const { installedModels, isLoading, error, refreshModels } = useModels();
  const { deleteModel, error: deleteError } = useModelDelete(refreshModels);
  const { downloads } = useModelDownload(refreshModels);

  const handleUseInChat = (modelName: string) => {
    // Store selected model in localStorage
    localStorage.setItem('selected-model', modelName);
    // Trigger navigation to chat page
    window.dispatchEvent(new CustomEvent('navigate-to-chat', { detail: { model: modelName } }));
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <LoadingSpinner size="lg" />
      </div>
    );
  }

  // Get currently downloading models
  const downloadingModels = Array.from(downloads.entries()).filter(
    ([_, progress]) => progress.isDownloading
  );

  return (
    <div className="p-8 max-w-7xl mx-auto">
      {(error || deleteError) && (
        <div className="mb-6">
          <ErrorBanner message={error || deleteError || ''} onRetry={refreshModels} />
        </div>
      )}

      {/* Currently Downloading Section */}
      {downloadingModels.length > 0 && (
        <div className="mb-10">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-2 h-2 bg-dark-accent rounded-full animate-pulse" />
            <h2 className="text-xl font-semibold text-dark-text">Downloading</h2>
            <span className="text-dark-text-muted text-sm">
              {downloadingModels.length} {downloadingModels.length === 1 ? 'model' : 'models'}
            </span>
          </div>
          
          <div className="space-y-3">
            {downloadingModels.map(([modelName, progress]) => (
              <div
                key={modelName}
                className="bg-dark-surface border border-dark-border rounded-xl p-5 hover:border-dark-accent transition-all"
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 bg-dark-accent bg-opacity-20 rounded-lg flex items-center justify-center">
                      <svg className="w-5 h-5 text-dark-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                      </svg>
                    </div>
                    <div>
                      <h3 className="text-dark-text font-medium">{modelName}</h3>
                      <p className="text-dark-text-secondary text-sm">{progress.status}</p>
                      <p className="text-dark-text-secondary text-xs mt-1">
                        {typeof progress.downloadedBytes === 'number' && typeof progress.totalBytes === 'number' && progress.totalBytes > 0
                          ? `${formatBytes(progress.downloadedBytes)} / ${formatBytes(progress.totalBytes)}`
                          : 'Preparing download'}
                        {typeof progress.speedMbps === 'number' && progress.speedMbps > 0
                          ? ` | ${formatSpeed(progress.speedMbps)}`
                          : ''}
                      </p>
                    </div>
                  </div>
                  <span className="text-dark-accent font-semibold">{progress.percentage}%</span>
                </div>
                
                <div className="w-full bg-dark-bg rounded-full h-2 overflow-hidden">
                  <div
                    className="bg-gradient-to-r from-dark-accent to-dark-accent-light h-2 rounded-full transition-all duration-300"
                    style={{ width: `${progress.percentage}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Installed Models Section */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <h2 className="text-xl font-semibold text-dark-text">My Models</h2>
            <span className="px-2.5 py-0.5 bg-dark-surface border border-dark-border rounded-full text-dark-text-secondary text-sm">
              {installedModels.length}
            </span>
          </div>
          <button
            onClick={refreshModels}
            className="text-dark-text-secondary hover:text-dark-text transition-colors"
            title="Refresh"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
        </div>

        {installedModels.length === 0 ? (
          <div className="bg-dark-surface border border-dark-border rounded-xl p-12 text-center">
            <div className="w-16 h-16 bg-dark-bg rounded-full flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-dark-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
              </svg>
            </div>
            <h3 className="text-dark-text font-medium mb-2">No models installed</h3>
            <p className="text-dark-text-secondary text-sm mb-6">
              Download a curated model from Browse, or use the wizard later for AI recommendations.
            </p>
            <button
              onClick={onBrowseModels}
              className="inline-flex items-center justify-center px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white text-sm font-medium rounded-lg transition-colors disabled:opacity-50"
              disabled={!onBrowseModels}
            >
              Browse Models
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {installedModels.map((model) => {
              const sizeInGB = (model.size / (1024 * 1024 * 1024)).toFixed(1);
              const modifiedDate = new Date(model.modified_at).toLocaleDateString('en-US', {
                month: 'short',
                day: 'numeric',
                year: 'numeric'
              });

              return (
                <div
                  key={model.name}
                  className="bg-dark-surface border border-dark-border rounded-xl p-5 hover:border-dark-accent hover:shadow-glow-sm transition-all group"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex-1 min-w-0">
                      <h3 className="text-dark-text font-semibold truncate mb-1">{model.name}</h3>
                      <div className="flex items-center gap-2 text-dark-text-secondary text-xs">
                        <span>{sizeInGB} GB</span>
                        <span>•</span>
                        <span>{modifiedDate}</span>
                      </div>
                    </div>
                    <div className="w-10 h-10 bg-dark-accent bg-opacity-10 rounded-lg flex items-center justify-center flex-shrink-0 ml-3">
                      <svg className="w-5 h-5 text-dark-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
                      </svg>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 mt-4">
                    <button
                      onClick={() => deleteModel(model.name)}
                      className="flex-1 px-3 py-2 bg-dark-bg hover:bg-dark-danger hover:text-white border border-dark-border hover:border-dark-danger text-dark-text-secondary text-sm rounded-lg transition-all"
                    >
                      Delete
                    </button>
                    <button
                      onClick={() => handleUseInChat(model.name)}
                      className="px-3 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white text-sm rounded-lg transition-colors"
                      title="Use in chat"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                      </svg>
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
