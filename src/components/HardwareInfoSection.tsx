import { LoadingSpinner } from './LoadingSpinner';
import type { HardwareInfo } from '../lib/types';

interface HardwareInfoSectionProps {
  hardwareInfo: HardwareInfo | null;
  onRedetect: () => void;
  isDetecting: boolean;
}

export function HardwareInfoSection({ hardwareInfo, onRedetect, isDetecting }: HardwareInfoSectionProps) {
  const usesUnifiedMemory = hardwareInfo
    ? hardwareInfo.gpu_backend === 'apple_metal' || hardwareInfo.is_shared_memory === true
    : false;
  const acceleratorMemoryGb = hardwareInfo
    ? usesUnifiedMemory ? hardwareInfo.ram_gb : hardwareInfo.vram_gb
    : 0;
  const acceleratorMemoryLabel = usesUnifiedMemory ? 'Unified Memory' : 'VRAM';

  const getBackendLabel = (backend: string) => {
    switch (backend) {
      case 'nvidia':
        return 'NVIDIA CUDA';
      case 'amd':
        return 'AMD ROCm';
      case 'intel':
        return 'Intel SYCL';
      case 'apple_metal':
        return 'Apple Metal';
      case 'cpu_only':
        return 'CPU Only';
      default:
        return backend;
    }
  };

  const getBackendColor = (backend: string) => {
    switch (backend) {
      case 'nvidia':
        return 'text-[#76b900]';
      case 'amd':
        return 'text-[#ed1c24]';
      case 'intel':
        return 'text-[#3b82f6]';
      case 'apple_metal':
        return 'text-[#555]';
      case 'cpu_only':
        return 'text-[#6a6a6a]';
      default:
        return 'text-[#1a1a1a]';
    }
  };

  return (
    <div className="glass rounded-2xl p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-base font-medium text-white">Hardware Information</h3>
          <p className="text-sm text-[#B1ADA1] mt-1">System specifications detected</p>
        </div>
        <button
          onClick={onRedetect}
          disabled={isDetecting}
          className="px-4 py-2 bg-[#1F1F1F] hover:bg-[#2A2A2A] border border-[#333333] hover:border-[#C15F3C] text-white rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isDetecting ? (
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Detecting...
            </span>
          ) : (
            'Re-detect'
          )}
        </button>
      </div>

      {isDetecting ? (
        <div className="flex flex-col items-center justify-center py-12">
          <LoadingSpinner size="lg" color="coral" />
          <p className="text-[#B1ADA1] text-sm mt-4">Detecting hardware...</p>
        </div>
      ) : hardwareInfo ? (
        <div className="grid grid-cols-2 gap-4">
          {/* GPU Card */}
          <div className="bg-[#1F1F1F] rounded-xl p-4 border border-[#2A2A2A]">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-lg bg-[#1C1C1C] flex items-center justify-center border border-[#333333]">
                <svg className="w-5 h-5 text-[#C15F3C]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
                </svg>
              </div>
              <div>
                <p className="text-xs text-[#B1ADA1]">GPU</p>
                <p className="text-sm text-white font-medium">{hardwareInfo.gpu_name}</p>
              </div>
            </div>
            <p className={`text-xs ${getBackendColor(hardwareInfo.gpu_backend)}`}>
              {getBackendLabel(hardwareInfo.gpu_backend)}
            </p>
          </div>

          {/* Accelerator memory card */}
          <div className="bg-[#1F1F1F] rounded-xl p-4 border border-[#2A2A2A]">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-lg bg-[#1C1C1C] flex items-center justify-center border border-[#333333]">
                <svg className="w-5 h-5 text-[#C15F3C]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                </svg>
              </div>
              <div>
                <p className="text-xs text-[#B1ADA1]">{acceleratorMemoryLabel}</p>
                <p className="text-sm text-white font-medium">
                  {acceleratorMemoryGb.toFixed(1)} GB
                  {usesUnifiedMemory && (
                    <span className="ml-1.5 px-1.5 py-0.5 text-[10px] font-medium bg-amber-900/40 text-amber-400 rounded">shared</span>
                  )}
                </p>
              </div>
            </div>
            <div className="w-full bg-[#2A2A2A] rounded-full h-1.5">
              <div 
                className="bg-gradient-to-r from-[#C15F3C] to-[#D47A5A] h-1.5 rounded-full" 
                style={{ width: `${Math.min((acceleratorMemoryGb / (usesUnifiedMemory ? 64 : 24)) * 100, 100)}%` }}
              ></div>
            </div>
          </div>

          {/* RAM Card */}
          <div className="bg-[#1F1F1F] rounded-xl p-4 border border-[#2A2A2A]">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-lg bg-[#1C1C1C] flex items-center justify-center border border-[#333333]">
                <svg className="w-5 h-5 text-[#5a9a6e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z" />
                </svg>
              </div>
              <div>
                <p className="text-xs text-[#B1ADA1]">System RAM</p>
                <p className="text-sm text-white font-medium">{hardwareInfo.ram_gb.toFixed(1)} GB</p>
              </div>
            </div>
            <div className="w-full bg-[#2A2A2A] rounded-full h-1.5">
              <div 
                className="bg-[#5a9a6e] h-1.5 rounded-full" 
                style={{ width: `${Math.min((hardwareInfo.ram_gb / 64) * 100, 100)}%` }}
              ></div>
            </div>
          </div>

          {/* CPU Card */}
          <div className="bg-[#1F1F1F] rounded-xl p-4 border border-[#2A2A2A]">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-lg bg-[#1C1C1C] flex items-center justify-center border border-[#333333]">
                <svg className="w-5 h-5 text-[#c49a5c]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
              </div>
              <div>
                <p className="text-xs text-[#B1ADA1]">CPU Cores</p>
                <p className="text-sm text-white font-medium">{hardwareInfo.cpu_cores} cores</p>
              </div>
            </div>
            <div className="flex gap-1">
              {Array.from({ length: Math.min(hardwareInfo.cpu_cores, 16) }).map((_, i) => (
                <div key={i} className="w-2 h-2 rounded-sm bg-[#c49a5c]"></div>
              ))}
            </div>
          </div>
        </div>
      ) : (
        <div className="text-center py-12">
          <div className="w-12 h-12 mx-auto mb-4 rounded-xl bg-[#1F1F1F] flex items-center justify-center border border-[#333333]">
            <svg className="w-6 h-6 text-[#B1ADA1]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
            </svg>
          </div>
          <p className="text-[#B1ADA1] text-sm">
            No hardware information available
          </p>
          <button
            onClick={onRedetect}
            className="mt-4 px-4 py-2 bg-[#C15F3C] hover:bg-[#A84E2F] text-white rounded-xl text-sm font-medium transition-all"
          >
            Detect Hardware
          </button>
        </div>
      )}
    </div>
  );
}
