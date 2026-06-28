import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createRuntimeInstallError, type RuntimeInstallError } from '../lib/runtimeErrors';

// Rust unit-variant enums serialize as plain strings via serde, e.g. "Cpu"
// BinaryStatus.variant is a string like "Cpu", "Cuda12_4", etc.
interface BinaryStatus {
  variant: string;
  installed: boolean;
  version: string | null;
  path: string | null;
}

interface DownloadProgress {
  variant: string;
  bytes_downloaded: number;
  total_bytes: number;
  percentage: number;
}

const VARIANT_LABELS: Record<string, { name: string; description: string; size: string }> = {
  Cpu: { name: 'CPU Only', description: 'Runs on CPU, no GPU required', size: '~120 MB' },
  Cuda12_4: { name: 'NVIDIA GPU (CUDA 12.4)', description: 'Optimized for NVIDIA GPUs with CUDA 12.4', size: '~200 MB' },
  Cuda13_1: { name: 'NVIDIA GPU (CUDA 13.1)', description: 'Optimized for NVIDIA GPUs with CUDA 13.1', size: '~200 MB' },
  Vulkan: { name: 'Vulkan (AMD)', description: 'Cross-platform GPU acceleration for AMD GPUs', size: '~180 MB' },
  Sycl: { name: 'Intel GPU (SYCL)', description: 'Optimized for Intel Arc and Xe graphics', size: '~220 MB' },
  Metal: { name: 'Apple Metal', description: 'Optimized for Apple Silicon', size: '~150 MB' },
};

/** Extract the variant key string regardless of how Rust serialized it */
function extractVariantKey(variant: unknown): string {
  if (typeof variant === 'string') return variant;
  if (typeof variant === 'object' && variant !== null) return Object.keys(variant)[0] || 'Unknown';
  return 'Unknown';
}

export function RuntimeSection() {
  const [statuses, setStatuses] = useState<BinaryStatus[]>([]);
  const [recommended, setRecommended] = useState<string>('');
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [error, setError] = useState<RuntimeInstallError | null>(null);
  const [showErrorDetails, setShowErrorDetails] = useState(false);

  useEffect(() => {
    loadData();

    const unlisten = listen<DownloadProgress>('binary-download-progress', (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  async function loadData() {
    try {
      const [rec, stats] = await Promise.all([
        invoke<unknown>('get_recommended_binary'),
        invoke<BinaryStatus[]>('get_binary_statuses'),
      ]);
      const recKey = extractVariantKey(rec);
      // Normalize variant keys in statuses (could be string or object depending on serde config)
      const normalizedStats = stats.map(s => ({
        ...s,
        variant: extractVariantKey(s.variant),
      }));
      setRecommended(recKey);
      setStatuses(normalizedStats as unknown as BinaryStatus[]);
      console.log('[RuntimeSection] Recommended:', recKey);
      console.log('[RuntimeSection] Statuses:', JSON.stringify(normalizedStats));
    } catch (e) {
      console.error('Failed to load binary data:', e);
      setError({
        message: "Couldn't load runtime information.",
        details: String(e),
      });
    }
  }

  async function handleDownload(variantKey: string) {
    setDownloading(variantKey);
    setError(null);
    setShowErrorDetails(false);

    try {
      // Send variant as the string directly — serde handles both "Cpu" and {"Cpu": null}
      await invoke('download_binary', { variant: variantKey });
      await loadData();
    } catch (e) {
      const runtimeName = VARIANT_LABELS[variantKey]?.name || variantKey;
      setError(createRuntimeInstallError(e, `the ${runtimeName} runtime`));
    } finally {
      setDownloading(null);
      setProgress(null);
    }
  }

  return (
    <div className="glass rounded-2xl p-6">
      <div className="flex items-center gap-4 mb-6">
        <div className="w-12 h-12 rounded-xl bg-[#1F1F1F] flex items-center justify-center border border-[#333333]">
          <svg className="w-6 h-6 text-[#C15F3C]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <div>
          <h3 className="text-base font-medium text-white">Runtime Engine</h3>
          <p className="text-sm text-[#B1ADA1]">Download optimized inference binaries</p>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-xl">
          <p className="whitespace-pre-line text-sm font-medium text-red-300">{error.message}</p>
          <button
            type="button"
            onClick={() => setShowErrorDetails((value) => !value)}
            className="mt-2 text-xs font-medium text-red-200 underline underline-offset-4 hover:text-white"
          >
            {showErrorDetails ? 'Hide Details' : 'View Details'}
          </button>
          {showErrorDetails && (
            <pre className="mt-2 max-h-32 overflow-auto rounded-md bg-black/30 p-2 text-xs leading-relaxed text-red-100 whitespace-pre-wrap">
              {error.details}
            </pre>
          )}
        </div>
      )}

      {downloading && progress && (
        <div className="mb-4 p-4 bg-[#1F1F1F] rounded-xl border border-[#333333]">
          <div className="flex justify-between text-sm mb-2">
            <span className="text-white">Downloading {VARIANT_LABELS[downloading]?.name || downloading}...</span>
            <span className="text-[#B1ADA1]">{progress.percentage.toFixed(1)}%</span>
          </div>
          <div className="h-2 bg-[#2A2A2A] rounded-full overflow-hidden">
            <div 
              className="h-full bg-gradient-to-r from-[#C15F3C] to-[#D47A5A] transition-all duration-300"
              style={{ width: `${progress.percentage}%` }}
            />
          </div>
          <p className="text-xs text-[#B1ADA1] mt-2">
            {(progress.bytes_downloaded / 1024 / 1024).toFixed(1)} MB / {(progress.total_bytes / 1024 / 1024).toFixed(1)} MB
          </p>
        </div>
      )}

      <div className="space-y-3">
        {statuses.map((status) => {
          const variantKey = typeof status.variant === 'string' ? status.variant : extractVariantKey(status.variant);
          const info = VARIANT_LABELS[variantKey] || { name: variantKey, description: '', size: '' };
          const isRecommended = variantKey === recommended;
          const isDownloading = downloading === variantKey;

          return (
            <div 
              key={variantKey}
              className={`p-4 rounded-xl border transition-all ${
                status.installed 
                  ? 'bg-[#1F1F1F] border-[#333333]' 
                  : 'bg-[#151515] border-[#2A2A2A]'
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  {status.installed ? (
                    <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center">
                      <svg className="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                      </svg>
                    </div>
                  ) : (
                    <div className="w-8 h-8 rounded-lg bg-[#2A2A2A] flex items-center justify-center">
                      <svg className="w-4 h-4 text-[#B1ADA1]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                      </svg>
                    </div>
                  )}
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-white">{info.name}</span>
                      {isRecommended && (
                        <span className="px-2 py-0.5 text-xs bg-[#C15F3C]/20 text-[#C15F3C] rounded-full">Recommended</span>
                      )}
                      {status.installed && (
                        <span className="px-2 py-0.5 text-xs bg-green-500/20 text-green-400 rounded-full">Installed</span>
                      )}
                    </div>
                    <p className="text-xs text-[#B1ADA1] mt-0.5">{info.description}</p>
                  </div>
                </div>

                {!status.installed && (
                  <button
                    onClick={() => handleDownload(variantKey)}
                    disabled={downloading !== null}
                    className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-all ${
                      isDownloading
                        ? 'bg-[#C15F3C]/50 text-white cursor-wait'
                        : 'bg-[#C15F3C] hover:bg-[#A84E2F] text-white'
                    } disabled:opacity-50`}
                  >
                    {isDownloading ? 'Downloading...' : `Download ${info.size}`}
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <p className="text-xs text-[#B1ADA1] mt-4">
        Binaries are downloaded to your app data folder and used for local inference.
      </p>
    </div>
  );
}
