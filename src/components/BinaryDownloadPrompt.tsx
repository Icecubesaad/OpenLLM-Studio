import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createRuntimeInstallError, type RuntimeInstallError } from '../lib/runtimeErrors';

interface DownloadProgress {
  variant: unknown;
  bytes_downloaded: number;
  total_bytes: number;
  percentage: number;
}

interface BinaryStatus {
  variant: unknown;
  installed: boolean;
  version: string | null;
  path: string | null;
}

interface BinaryDownloadPromptProps {
  onBinaryReady?: () => void;
}

/** Extract the variant key string regardless of how Rust serialized it */
function extractVariantKey(variant: unknown): string {
  if (typeof variant === 'string') return variant;
  if (typeof variant === 'object' && variant !== null) return Object.keys(variant)[0] || 'Unknown';
  return 'Unknown';
}

const VARIANT_DISPLAY: Record<string, string> = {
  Cpu: 'CPU Only',
  Cuda12_4: 'NVIDIA GPU (CUDA 12.4)',
  Cuda13_1: 'NVIDIA GPU (CUDA 13.1)',
  Vulkan: 'Vulkan (AMD)',
  Sycl: 'Intel GPU (SYCL)',
  Metal: 'Apple Metal',
};

export function BinaryDownloadPrompt({ onBinaryReady }: BinaryDownloadPromptProps) {
  const [recommendedVariant, setRecommendedVariant] = useState<string>('');
  const [statuses, setStatuses] = useState<BinaryStatus[]>([]);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [error, setError] = useState<RuntimeInstallError | null>(null);
  const [showErrorDetails, setShowErrorDetails] = useState(false);

  useEffect(() => {
    loadInitialData();
    
    // Listen for download progress
    const unlisten = listen<DownloadProgress>('binary-download-progress', (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  async function loadInitialData() {
    try {
      const [recommended, currentStatuses] = await Promise.all([
        invoke<unknown>('get_recommended_binary'),
        invoke<BinaryStatus[]>('get_binary_statuses'),
      ]);
      const recKey = extractVariantKey(recommended);
      setRecommendedVariant(recKey);
      setStatuses(currentStatuses);
      console.log('[BinaryDownloadPrompt] Recommended:', recKey);
      console.log('[BinaryDownloadPrompt] Statuses:', JSON.stringify(currentStatuses));
    } catch (e) {
      setError({
        message: "Couldn't detect the recommended runtime.",
        details: String(e),
      });
    }
  }

  async function handleDownload() {
    setDownloading(true);
    setError(null);
    
    try {
      // Send variant as a string — serde handles unit variant deserialization from strings
      console.log('[BinaryDownloadPrompt] Downloading variant:', recommendedVariant);
      await invoke('download_binary', { variant: recommendedVariant });
      await loadInitialData();
      onBinaryReady?.();
    } catch (e) {
      setError(createRuntimeInstallError(e, `the ${displayName} runtime`));
      setShowErrorDetails(false);
    } finally {
      setDownloading(false);
    }
  }

  // Check if the recommended binary is installed
  const isRecommendedInstalled = statuses.some(
    s => extractVariantKey(s.variant) === recommendedVariant && s.installed === true
  );

  // Also check if ANY binary is installed (user may have a different variant)
  const anyInstalled = statuses.some(s => s.installed === true);

  console.log('[BinaryDownloadPrompt] recommendedVariant:', recommendedVariant);
  console.log('[BinaryDownloadPrompt] isRecommendedInstalled:', isRecommendedInstalled);
  console.log('[BinaryDownloadPrompt] anyInstalled:', anyInstalled);

  if (isRecommendedInstalled || anyInstalled) {
    return null; // Already installed, don't show prompt
  }

  const displayName = VARIANT_DISPLAY[recommendedVariant] || recommendedVariant;

  return (
    <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
      <div className="bg-[#111111] rounded-2xl p-6 max-w-md w-full mx-4 shadow-2xl border border-[#2A2A2A]">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-10 h-10 rounded-xl bg-[#1F1F1F] flex items-center justify-center border border-[#333333]">
            <svg className="w-5 h-5 text-[#C15F3C]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </div>
          <div>
            <h2 className="text-lg font-semibold text-white">GPU Acceleration Available</h2>
            <p className="text-sm text-[#B1ADA1]">Download optimized binaries</p>
          </div>
        </div>

        <p className="text-[#B1ADA1] mb-4">
          We detected <span className="text-[#C15F3C] font-medium">{displayName}</span> on your system.
          Download the optimized runtime for faster inference?
        </p>

        {downloading && progress && (
          <div className="mb-4">
            <div className="flex justify-between text-sm text-[#B1ADA1] mb-1">
              <span>Downloading...</span>
              <span>{progress.percentage.toFixed(1)}%</span>
            </div>
            <div className="h-2 bg-[#2A2A2A] rounded-full overflow-hidden">
              <div 
                className="h-full bg-[#C15F3C] transition-all duration-300"
                style={{ width: `${progress.percentage}%` }}
              />
            </div>
            <p className="text-xs text-[#B1ADA1] mt-1">
              {(progress.bytes_downloaded / 1024 / 1024).toFixed(1)} MB / {(progress.total_bytes / 1024 / 1024).toFixed(1)} MB
            </p>
          </div>
        )}

        {error && (
          <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg">
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

        <div className="flex gap-3">
          <button
            onClick={handleDownload}
            disabled={downloading}
            className="flex-1 px-4 py-2.5 bg-[#C15F3C] hover:bg-[#A84E2F] text-white font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {downloading ? 'Downloading...' : 'Download & Enable'}
          </button>
          <button
            onClick={() => onBinaryReady?.()}
            disabled={downloading}
            className="px-4 py-2.5 bg-[#1F1F1F] hover:bg-[#2A2A2A] text-[#B1ADA1] font-medium rounded-lg border border-[#333333] transition-colors disabled:opacity-50"
          >
            Use CPU Only
          </button>
        </div>

        <p className="text-xs text-[#B1ADA1] mt-3 text-center">
          Download size: ~200 MB. You can change this later in Settings.
        </p>
      </div>
    </div>
  );
}
