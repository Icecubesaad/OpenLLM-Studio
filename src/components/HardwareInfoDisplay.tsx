import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface HardwareInfo {
  gpu_name: string;
  gpu_backend: string;
  vram_gb: number;
  ram_gb: number;
  cpu_cores: number;
  disk_space_gb?: number;
  is_shared_memory?: boolean;
}

export function HardwareInfoDisplay() {
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastScan, setLastScan] = useState<Date | null>(null);

  useEffect(() => {
    loadHardware();
  }, []);

  const loadHardware = async () => {
    setLoading(true);
    try {
      const dbPath = await invoke<string>('get_db_path');
      const info = await invoke<HardwareInfo>('detect_hardware_cached', { dbPath });
      setHardware(info);
      setLastScan(new Date());
    } catch (err) {
      console.error('Failed to load hardware info:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleRefresh = async () => {
    setLoading(true);
    try {
      const info = await invoke<HardwareInfo>('detect_hardware');
      setHardware(info);
      setLastScan(new Date());
    } catch (err) {
      console.error('Failed to refresh hardware info:', err);
    } finally {
      setLoading(false);
    }
  };

  if (loading && !hardware) {
    return (
      <div className="animate-pulse space-y-3">
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-2/3"></div>
      </div>
    );
  }

  if (!hardware) {
    return <div className="text-red-600">Failed to detect hardware</div>;
  }

  const usesUnifiedMemory = hardware.gpu_backend === 'apple_metal' || hardware.is_shared_memory === true;
  const acceleratorMemoryLabel = usesUnifiedMemory ? 'Unified Memory' : 'VRAM';
  const acceleratorMemoryGb = usesUnifiedMemory ? hardware.ram_gb : hardware.vram_gb;

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-semibold">Hardware Information</h3>
        <button
          onClick={handleRefresh}
          disabled={loading}
          className="px-3 py-1 text-sm bg-[#C15F3C] text-white rounded-lg hover:bg-[#A84E2F] disabled:opacity-50 transition-all"
        >
          {loading ? 'Scanning...' : 'Refresh'}
        </button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="text-sm text-gray-600 dark:text-gray-400">GPU</div>
          <div className="font-medium">{hardware.gpu_name}</div>
          <div className="text-xs text-gray-500 capitalize">{hardware.gpu_backend.replace('_', ' ')}</div>
        </div>

        <div>
          <div className="text-sm text-gray-600 dark:text-gray-400">{acceleratorMemoryLabel}</div>
          <div className="font-medium">
            {acceleratorMemoryGb.toFixed(1)} GB
            {usesUnifiedMemory && (
              <span className="ml-1.5 px-1.5 py-0.5 text-[10px] font-medium bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-400 rounded">shared</span>
            )}
          </div>
        </div>

        <div>
          <div className="text-sm text-gray-600 dark:text-gray-400">RAM</div>
          <div className="font-medium">{hardware.ram_gb.toFixed(1)} GB</div>
        </div>

        <div>
          <div className="text-sm text-gray-600 dark:text-gray-400">CPU Cores</div>
          <div className="font-medium">{hardware.cpu_cores}</div>
        </div>

        {hardware.disk_space_gb && (
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Available Disk</div>
            <div className="font-medium">{hardware.disk_space_gb.toFixed(1)} GB</div>
          </div>
        )}
      </div>

      {lastScan && (
        <div className="text-xs text-gray-500">
          Last scanned: {lastScan.toLocaleString()}
        </div>
      )}
    </div>
  );
}
