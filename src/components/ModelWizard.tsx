import { useEffect, useMemo, useRef, useState } from 'react';
import { useHardwareInfo } from '../hooks/useHardwareInfo';
import { downloadHfModel, getHfModelFiles, getModelRecommendations, listenToDownloadProgress, listenToWizardStatus, pullModelWithRetry } from '../lib/tauri';
import type { DownloadProgress } from '../lib/tauri';
import type { HfModelFile } from '../lib/types';
import { useAppStore } from '../store/appStore';

interface ModelRecommendation {
  model_id: string;
  model_name: string;
  repo_id: string;
  filename: string;
  quantization?: string | null;
  suitability_score: number;
  estimated_memory_gb: number;
  estimated_tokens_per_sec: number;
  quality_rating: number;
  description: string;
}

interface ModelWizardProps {
  onClose?: () => void;
  onBrowseManually?: () => void;
}

export function ModelWizard({ onClose, onBrowseManually }: ModelWizardProps) {
  const [step, setStep] = useState(1);
  const [useCase, setUseCase] = useState<'coding' | 'chat' | 'agents'>('chat');
  const [recommendations, setRecommendations] = useState<ModelRecommendation[]>([]);
  const [selectedModel, setSelectedModel] = useState<ModelRecommendation | null>(null);
  const [availableFiles, setAvailableFiles] = useState<HfModelFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<HfModelFile | null>(null);
  const [loadingRecs, setLoadingRecs] = useState(false);
  const [recError, setRecError] = useState<string | null>(null);
  const [loadingFiles, setLoadingFiles] = useState(false);
  const [filesError, setFilesError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [activeDownloadModelName, setActiveDownloadModelName] = useState<string | null>(null);
  const [wizardStage, setWizardStage] = useState<string | null>(null);
  const { hardwareInfo, isDetecting, redetect } = useHardwareInfo();
  const { openRouterApiKey, claudeApiKey, openAiApiKey, recommendationApiKey, hfToken } = useAppStore();
  const hasAnyApiKey = !!(openRouterApiKey || recommendationApiKey || claudeApiKey || openAiApiKey);
  const recommendationRequestRef = useRef(0);
  const filesRequestRef = useRef(0);
  const lastAutoFetchKeyRef = useRef<string | null>(null);

  const browseManually = () => {
    if (onBrowseManually) {
      onBrowseManually();
      return;
    }

    onClose?.();
  };

  const useCaseDescriptions = {
    coding: 'Optimized for code generation, refactoring, and technical tasks',
    chat: 'Best for conversational AI and general-purpose interactions',
    agents: 'Specialized for tool-calling and autonomous task execution',
  };

  const availableMemory = useMemo(
    () => hardwareInfo ? Math.max(hardwareInfo.ram_gb, hardwareInfo.vram_gb) : 8,
    [hardwareInfo],
  );
  const selectedModelIsCatalog = selectedModel ? isCatalogRecommendation(selectedModel) : false;

  const recommendedFilename = useMemo(() => {
    if (!selectedModel || isCatalogRecommendation(selectedModel) || availableFiles.length === 0) {
      return null;
    }

    return (
      availableFiles.find((file) => file.filename === selectedModel.filename)?.filename
      || availableFiles.find((file) => file.quantization === selectedModel.quantization)?.filename
      || availableFiles[0]?.filename
      || null
    );
  }, [availableFiles, selectedModel]);

  useEffect(() => {
    const autoFetchKey = step === 3 && hardwareInfo
      ? `${useCase}:${hardwareInfo.ram_gb}:${hardwareInfo.vram_gb}:${hasAnyApiKey}`
      : null;

    if (
      step === 3
      && hardwareInfo
      && recommendations.length === 0
      && !loadingRecs
      && autoFetchKey
      && lastAutoFetchKeyRef.current !== autoFetchKey
    ) {
      lastAutoFetchKeyRef.current = autoFetchKey;
      void fetchRecommendations();
    }
  }, [step, hardwareInfo, recommendations.length, loadingRecs, hasAnyApiKey, useCase]);

  useEffect(() => {
    if (step !== 4 || !selectedModel || isCatalogRecommendation(selectedModel)) {
      return;
    }

    const requestId = ++filesRequestRef.current;

    setLoadingFiles(true);
    setFilesError(null);
    setWizardStage('Looking for models on Hugging Face...');

    void (async () => {
      try {
        const files = await getHfModelFiles(selectedModel.repo_id, hfToken || undefined);

        if (requestId !== filesRequestRef.current) {
          return;
        }

        setAvailableFiles(files);

        const preferredFile = files.find((file) => file.filename === selectedModel.filename)
          || files.find((file) => file.quantization === selectedModel.quantization)
          || files[0]
          || null;

        setSelectedFile((current) => {
          if (current) {
            const persisted = files.find((file) => file.filename === current.filename);
            if (persisted) {
              return persisted;
            }
          }
          return preferredFile;
        });
      } catch (error) {
        if (requestId !== filesRequestRef.current) {
          return;
        }

        setAvailableFiles([]);
        setSelectedFile(null);
        setFilesError(String(error));
      } finally {
        if (requestId === filesRequestRef.current) {
          setLoadingFiles(false);
          setWizardStage(null);
        }
      }
    })();
  }, [step, selectedModel, hfToken]);

  useEffect(() => {
    const setupListener = async () => {
      const unlisten = await listenToDownloadProgress((progress) => {
        setDownloadProgress(progress);
      });
      return unlisten;
    };

    let unlisten: (() => void) | undefined;
    setupListener().then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listenToWizardStatus((status) => {
      setWizardStage(status);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    if (!downloading || !activeDownloadModelName || !downloadProgress) {
      return;
    }

    if (downloadProgress.model_name !== activeDownloadModelName) {
      return;
    }

    if (downloadProgress.status === 'success') {
      setWizardStage(null);
      setDownloading(false);
      setActiveDownloadModelName(null);
      alert('Download complete.\n\nThe model is ready to use in the chat.');
      return;
    }

    if (downloadProgress.status.startsWith('Error:')) {
      setWizardStage(null);
      setDownloading(false);
      setActiveDownloadModelName(null);
      alert(`Download failed: ${downloadProgress.status.replace(/^Error:\s*/, '')}`);
    }
  }, [activeDownloadModelName, downloadProgress, downloading]);

  const fetchRecommendations = async () => {
    if (!hardwareInfo) {
      return;
    }

    const requestId = ++recommendationRequestRef.current;
    const state = useAppStore.getState();
    const orKey = state.openRouterApiKey || state.recommendationApiKey;
    const clKey = state.claudeApiKey;
    const oaKey = state.openAiApiKey;
    const preferredProvider = state.wizardProvider;

    const providerCandidates = [
      { provider: preferredProvider, apiKey: preferredProvider === 'openrouter' ? orKey : preferredProvider === 'claude' ? clKey : oaKey },
      { provider: 'openrouter' as const, apiKey: orKey },
      { provider: 'claude' as const, apiKey: clKey },
      { provider: 'openai' as const, apiKey: oaKey },
    ];

    const activeProvider = providerCandidates.find((candidate) => candidate.apiKey);
    const providerForRequest = activeProvider?.provider || 'openrouter';
    const selectedProviderModel = providerForRequest === 'openrouter'
      ? state.openRouterModel
      : providerForRequest === 'claude'
        ? state.claudeModel
        : state.openAiModel;

    setLoadingRecs(true);
    setRecError(null);
    setWizardStage(activeProvider?.apiKey ? 'Getting AI recommendation...' : 'Finding local model options...');

    let lastError: unknown = null;

    try {
      for (let attempt = 1; attempt <= 2; attempt += 1) {
        try {
          if (attempt > 1) {
            setWizardStage(`Retrying AI recommendation... (attempt ${attempt}/2)`);
          }

          const recs = await getModelRecommendations(
            hardwareInfo.ram_gb,
            hardwareInfo.vram_gb,
            useCase,
            activeProvider?.apiKey || undefined,
            providerForRequest,
            selectedProviderModel,
            hfToken || undefined,
          ) as ModelRecommendation[];

          if (requestId !== recommendationRequestRef.current) {
            return;
          }

          setRecommendations(recs);
          setSelectedModel((current) => {
            if (current) {
              return recs.find((rec) => rec.repo_id === current.repo_id && rec.filename === current.filename)
                || recs.find((rec) => rec.model_id === current.model_id)
                || recs[0]
                || null;
            }
            return recs[0] || null;
          });
          return;
        } catch (error) {
          lastError = error;
          if (attempt < 2) {
            await delay(450 * attempt);
          }
        }
      }

      if (requestId !== recommendationRequestRef.current) {
        return;
      }

      setRecError(String(lastError));
      setRecommendations([]);
      setSelectedModel(null);
    } finally {
      if (requestId === recommendationRequestRef.current) {
        setLoadingRecs(false);
        setWizardStage(null);
      }
    }
  };

  const handleDownload = async () => {
    if (!selectedModel) {
      return;
    }

    const isCatalogModel = isCatalogRecommendation(selectedModel);
    if (!isCatalogModel && !selectedFile) {
      return;
    }

    const modelName = isCatalogModel
      ? selectedModel.model_id
      : deriveModelName(selectedModel.repo_id, selectedFile!.filename);

    setDownloading(true);
    setDownloadProgress(null);
    setActiveDownloadModelName(modelName);
    setWizardStage(isCatalogModel
      ? `Pulling ${selectedModel.model_name}...`
      : `Starting ${selectedFile!.quantization || 'selected'} download...`);
    onClose?.();

    try {
      if (isCatalogModel) {
        await pullModelWithRetry(
          selectedModel.model_id,
          selectedModel.quantization || undefined,
        );
      } else {
        await downloadHfModel(
          selectedModel.repo_id,
          selectedFile!.filename,
          modelName,
          hfToken || undefined,
        );
      }
    } catch (error) {
      setWizardStage(null);
      setDownloading(false);
      setActiveDownloadModelName(null);
      const errorMsg = String(error);

      if (errorMsg.includes('TLS handshake timeout') || errorMsg.includes('timeout')) {
        alert('Download failed due to network timeout.\n\nPlease try again. The download will resume from where it left off.');
      } else {
        alert(`Download failed: ${errorMsg}\n\nPlease check your internet connection and try again.`);
      }
    }
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between mb-8">
        {[1, 2, 3, 4, 5].map((s) => (
          <div key={s} className="flex items-center">
            <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
              step >= s ? 'bg-dark-accent text-white' : 'bg-dark-surface text-dark-text-secondary'
            }`}>
              {s}
            </div>
            {s < 5 && <div className={`w-16 h-1 ${step > s ? 'bg-dark-accent' : 'bg-dark-surface'}`} />}
          </div>
        ))}
      </div>

      {step === 1 && (
        <div className="space-y-4">
          <div>
            <h3 className="text-xl font-semibold text-dark-text mb-2">Step 1: Hardware Scan</h3>
            <p className="text-dark-text-secondary">Detecting your system specifications...</p>
          </div>

          {isDetecting ? (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-dark-accent mx-auto mb-3"></div>
              <p className="text-dark-text-secondary">Detecting hardware...</p>
            </div>
          ) : hardwareInfo ? (
            <div className="bg-dark-surface rounded-lg p-4 space-y-3">
              <div className="flex justify-between">
                <span className="text-dark-text-secondary">GPU:</span>
                <span className="text-dark-text font-medium">{hardwareInfo.gpu_name}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-dark-text-secondary">VRAM:</span>
                <span className="text-dark-text font-medium">{hardwareInfo.vram_gb.toFixed(1)} GB</span>
              </div>
              <div className="flex justify-between">
                <span className="text-dark-text-secondary">RAM:</span>
                <span className="text-dark-text font-medium">{hardwareInfo.ram_gb.toFixed(1)} GB</span>
              </div>
              <div className="flex justify-between">
                <span className="text-dark-text-secondary">CPU Cores:</span>
                <span className="text-dark-text font-medium">{hardwareInfo.cpu_cores}</span>
              </div>
            </div>
          ) : (
            <div className="bg-dark-surface rounded-lg p-4 text-center">
              <p className="text-red-400 mb-3">Failed to detect hardware</p>
              <button
                onClick={redetect}
                disabled={isDetecting}
                className="px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg disabled:opacity-50"
              >
                {isDetecting ? 'Detecting...' : 'Retry Detection'}
              </button>
            </div>
          )}

          <button
            onClick={() => setStep(2)}
            disabled={!hardwareInfo || isDetecting}
            className="w-full px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Next
          </button>
        </div>
      )}

      {step === 2 && (
        <div className="space-y-4">
          <div>
            <h3 className="text-xl font-semibold text-dark-text mb-2">Step 2: Select Use Case</h3>
            <p className="text-dark-text-secondary">What will you primarily use this model for?</p>
          </div>

          <div className="space-y-3">
            {(['coding', 'chat', 'agents'] as const).map((uc) => (
              <button
                key={uc}
                onClick={() => setUseCase(uc)}
                className={`block w-full p-4 text-left rounded-lg border-2 transition-colors ${
                  useCase === uc
                    ? 'border-dark-accent bg-dark-accent bg-opacity-10'
                    : 'border-dark-border hover:border-dark-accent-hover'
                }`}
              >
                <div className="font-semibold text-dark-text capitalize mb-1">{uc}</div>
                <div className="text-sm text-dark-text-secondary">{useCaseDescriptions[uc]}</div>
              </button>
            ))}
          </div>

          <div className="flex gap-2">
            <button
              onClick={() => setStep(1)}
              className="flex-1 px-4 py-2 border border-dark-border text-dark-text rounded-lg hover:bg-dark-surface"
            >
              Back
            </button>
            <button
              onClick={() => {
                setRecommendations([]);
                setSelectedModel(null);
                setAvailableFiles([]);
                setSelectedFile(null);
                setFilesError(null);
                lastAutoFetchKeyRef.current = null;
                setStep(3);
              }}
              className="flex-1 px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg"
            >
              Next
            </button>
          </div>
        </div>
      )}

      {step === 3 && (
        <div className="space-y-4">
          <div>
            <h3 className="text-xl font-semibold text-dark-text mb-2">Step 3: Select Model</h3>
            <p className="text-dark-text-secondary">Recommendations based on your hardware and use case</p>
          </div>

          {loadingRecs ? (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-dark-accent mx-auto mb-3"></div>
              <p className="text-dark-text-secondary">{wizardStage || 'Finding local model options...'}</p>
            </div>
          ) : recommendations.length > 0 ? (
            <div className="space-y-3">
              {recommendations.map((rec) => (
                <button
                  key={`${rec.repo_id}:${rec.filename}`}
                  onClick={() => setSelectedModel(rec)}
                  className={`w-full bg-dark-surface rounded-lg p-4 border-2 transition-colors text-left ${
                    selectedModel?.repo_id === rec.repo_id && selectedModel?.filename === rec.filename
                      ? 'border-dark-accent bg-dark-accent bg-opacity-10'
                      : 'border-dark-border hover:border-dark-accent-hover'
                  }`}
                >
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <h4 className="text-dark-text font-semibold">{rec.model_name}</h4>
                      <p className="text-sm text-dark-text-secondary">
                        {isCatalogRecommendation(rec) ? 'Ollama catalog pull' : rec.repo_id}
                      </p>
                    </div>
                    <div className="text-right">
                      <div className="text-dark-accent font-semibold">{(rec.suitability_score * 100).toFixed(0)}%</div>
                      <div className="text-xs text-dark-text-secondary">Match</div>
                    </div>
                  </div>
                  <p className="text-sm text-dark-text-secondary mb-3">{rec.description}</p>
                  <div className="flex gap-4 text-xs text-dark-text-secondary">
                    <span>{isCatalogRecommendation(rec) ? rec.model_id : (rec.quantization || 'GGUF')}</span>
                    <span>{rec.estimated_memory_gb.toFixed(1)} GB</span>
                    <span>~{rec.estimated_tokens_per_sec.toFixed(0)} tok/s</span>
                    <span>{(rec.quality_rating * 100).toFixed(0)}% quality</span>
                  </div>
                </button>
              ))}
            </div>
          ) : recError ? (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <p className="text-dark-text font-medium mb-2">Failed to get recommendations</p>
              <p className="text-dark-text-secondary text-sm mb-4">{recError}</p>
              <div className="flex flex-col sm:flex-row justify-center gap-2">
                <button
                  onClick={() => {
                    lastAutoFetchKeyRef.current = null;
                    void fetchRecommendations();
                  }}
                  className="px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg"
                >
                  Retry
                </button>
                <button
                  onClick={browseManually}
                  className="px-4 py-2 border border-dark-border text-dark-text rounded-lg hover:bg-dark-bg"
                >
                  Browse manually
                </button>
              </div>
            </div>
          ) : (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <p className="text-dark-text font-medium mb-2">No recommendations yet</p>
              <p className="text-dark-text-secondary text-sm mb-4">
                Click below to find local model options for your hardware.
              </p>
              <div className="flex flex-col sm:flex-row justify-center gap-2">
                <button
                  onClick={() => {
                    lastAutoFetchKeyRef.current = null;
                    void fetchRecommendations();
                  }}
                  className="px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg"
                >
                  Find Models
                </button>
                <button
                  onClick={browseManually}
                  className="px-4 py-2 border border-dark-border text-dark-text rounded-lg hover:bg-dark-bg"
                >
                  Browse manually
                </button>
              </div>
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={() => setStep(2)}
              className="flex-1 px-4 py-2 border border-dark-border text-dark-text rounded-lg hover:bg-dark-surface"
            >
              Back
            </button>
            <button
              onClick={() => {
                if (!selectedModel) {
                  return;
                }

                setStep(isCatalogRecommendation(selectedModel) ? 5 : 4);
              }}
              disabled={!selectedModel}
              className="flex-1 px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg disabled:opacity-50"
            >
              Next
            </button>
          </div>
        </div>
      )}

      {step === 4 && selectedModel && !selectedModelIsCatalog && (
        <div className="space-y-4">
          <div>
            <h3 className="text-xl font-semibold text-dark-text mb-2">Step 4: Select Quantization</h3>
            <p className="text-dark-text-secondary">Real GGUF files from Hugging Face for {selectedModel.model_name}</p>
          </div>

          {loadingFiles ? (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-dark-accent mx-auto mb-3"></div>
              <p className="text-dark-text-secondary">{wizardStage || 'Looking for models on Hugging Face...'}</p>
            </div>
          ) : filesError ? (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <p className="text-dark-text font-medium mb-2">Failed to load quantizations</p>
              <p className="text-dark-text-secondary text-sm mb-4">{filesError}</p>
              <button
                onClick={() => setStep(3)}
                className="px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg"
              >
                Pick another model
              </button>
            </div>
          ) : availableFiles.length > 0 ? (
            <div className="space-y-2">
              {availableFiles.map((file) => {
                const isSelected = selectedFile?.filename === file.filename;
                const isRecommended = file.filename === recommendedFilename;
                const fits = file.estimated_ram_gb <= availableMemory * 1.15;

                return (
                  <button
                    key={file.filename}
                    onClick={() => setSelectedFile(file)}
                    className={`w-full p-4 rounded-lg border-2 transition-colors text-left ${
                      isSelected
                        ? 'border-dark-accent bg-dark-accent bg-opacity-10'
                        : 'border-dark-border hover:border-dark-accent-hover'
                    }`}
                  >
                    <div className="flex justify-between items-start gap-3 mb-1">
                      <div>
                        <div className="flex items-center gap-2 flex-wrap">
                          <div className="font-semibold text-dark-text">{file.quantization || 'Unknown quantization'}</div>
                          {isRecommended && (
                            <span className="px-2 py-0.5 rounded-full bg-green-600/20 text-green-400 text-xs font-medium">
                              Recommended
                            </span>
                          )}
                          {!fits && (
                            <span className="px-2 py-0.5 rounded-full bg-red-600/20 text-red-400 text-xs font-medium">
                              May not fit
                            </span>
                          )}
                        </div>
                        <p className="text-sm text-dark-text-secondary break-all">{file.filename}</p>
                      </div>
                      <div className="text-right">
                        <div className="text-dark-text font-medium">{formatFileSizeGb(file.size)}</div>
                        <div className="text-xs text-dark-text-secondary">{getQuantQualityLabel(file.quantization)}</div>
                      </div>
                    </div>
                    <p className="text-sm text-dark-text-secondary">
                      Estimated RAM: {file.estimated_ram_gb.toFixed(1)} GB
                    </p>
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="bg-dark-surface rounded-lg p-6 text-center">
              <p className="text-dark-text font-medium mb-2">No GGUF files found</p>
              <p className="text-dark-text-secondary text-sm">This recommended repo does not currently expose downloadable GGUF files.</p>
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={() => setStep(3)}
              className="flex-1 px-4 py-2 border border-dark-border text-dark-text rounded-lg hover:bg-dark-surface"
            >
              Back
            </button>
            <button
              onClick={() => setStep(5)}
              disabled={!selectedFile || loadingFiles}
              className="flex-1 px-4 py-2 bg-dark-accent hover:bg-dark-accent-hover text-white rounded-lg disabled:opacity-50"
            >
              Next
            </button>
          </div>
        </div>
      )}

      {step === 5 && selectedModel && (selectedModelIsCatalog || selectedFile) && (
        <div className="space-y-4">
          <div>
            <h3 className="text-xl font-semibold text-dark-text mb-2">Step 5: Download & Deploy</h3>
            <p className="text-dark-text-secondary">
              {selectedModelIsCatalog
                ? 'Ready to pull the selected catalog model'
                : 'Ready to download the exact file you selected'}
            </p>
          </div>

          <div className="bg-dark-surface rounded-lg p-6 space-y-4">
            <div>
              <h4 className="text-dark-text font-semibold mb-2">Selected Configuration</h4>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between gap-4">
                  <span className="text-dark-text-secondary">Model:</span>
                  <span className="text-dark-text text-right">{selectedModel.model_name}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-dark-text-secondary">Source:</span>
                  <span className="text-dark-text text-right break-all">
                    {selectedModelIsCatalog ? 'Ollama catalog pull' : selectedModel.repo_id}
                  </span>
                </div>
                {selectedModelIsCatalog && (
                  <div className="flex justify-between gap-4">
                    <span className="text-dark-text-secondary">Catalog name:</span>
                    <span className="text-dark-text text-right break-all">{selectedModel.model_id}</span>
                  </div>
                )}
                <div className="flex justify-between gap-4">
                  <span className="text-dark-text-secondary">Quantization:</span>
                  <span className="text-dark-text">
                    {selectedModelIsCatalog ? (selectedModel.quantization || 'Automatic') : (selectedFile?.quantization || 'Unknown')}
                  </span>
                </div>
                {!selectedModelIsCatalog && selectedFile && (
                  <div className="flex justify-between gap-4">
                    <span className="text-dark-text-secondary">File size:</span>
                    <span className="text-dark-text">{formatFileSizeGb(selectedFile.size)}</span>
                  </div>
                )}
                <div className="flex justify-between gap-4">
                  <span className="text-dark-text-secondary">Estimated RAM:</span>
                  <span className="text-dark-text">
                    {(selectedModelIsCatalog ? selectedModel.estimated_memory_gb : selectedFile?.estimated_ram_gb || 0).toFixed(1)} GB
                  </span>
                </div>
                {!selectedModelIsCatalog && selectedFile && (
                  <>
                    <div className="flex justify-between gap-4">
                      <span className="text-dark-text-secondary">Recommended:</span>
                      <span className="text-dark-text">{selectedModel.quantization || 'No preferred quantization'}</span>
                    </div>
                    <div className="flex justify-between gap-4">
                      <span className="text-dark-text-secondary">Selected file:</span>
                      <span className="text-dark-text text-right break-all">{selectedFile.filename}</span>
                    </div>
                  </>
                )}
              </div>
            </div>

            {downloading && downloadProgress && (
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className={`${downloadProgress.status.startsWith('Error:') ? 'text-red-400' : 'text-dark-text-secondary'}`}>
                    {downloadProgress.status}
                  </span>
                  {downloadProgress.completed && downloadProgress.total && (
                    <span className="text-dark-text">
                      {((downloadProgress.completed / downloadProgress.total) * 100).toFixed(0)}%
                    </span>
                  )}
                </div>
                <div className="w-full bg-dark-border rounded-full h-2">
                  <div
                    className={`h-2 rounded-full transition-all ${
                      downloadProgress.status.startsWith('Error:') ? 'bg-red-500' : 'bg-dark-accent'
                    }`}
                    style={{
                      width: downloadProgress.completed && downloadProgress.total
                        ? `${(downloadProgress.completed / downloadProgress.total) * 100}%`
                        : '0%',
                    }}
                  />
                </div>
              </div>
            )}
          </div>

          <div className="flex gap-2">
            <button
              onClick={() => setStep(selectedModelIsCatalog ? 3 : 4)}
              disabled={downloading}
              className="flex-1 px-4 py-2 border border-dark-border text-dark-text rounded-lg hover:bg-dark-surface disabled:opacity-50"
            >
              Back
            </button>
            <button
              onClick={handleDownload}
              disabled={downloading}
              className="flex-1 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg disabled:opacity-50"
            >
              {downloading ? 'Downloading...' : selectedModelIsCatalog ? 'Pull & Deploy' : 'Download & Deploy'}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function deriveModelName(repoId: string, filename: string): string {
  const baseName = repoId.split('/').pop() || repoId;
  const quantization = filename.match(/Q\d+_K_[MSL]|Q\d+_\d+|Q\d+|F\d+/i)?.[0] || '';
  return `${baseName}-${quantization}`.toLowerCase().replace(/[^a-z0-9-]/g, '-');
}

function formatFileSizeGb(size: number): string {
  return `${(size / 1_073_741_824).toFixed(1)} GB`;
}

function getQuantQualityLabel(quantization: string | null | undefined): string {
  switch (quantization) {
    case 'F16':
    case 'F32':
      return 'Best quality';
    case 'Q8_0':
      return 'Excellent quality';
    case 'Q6_K':
      return 'High quality';
    case 'Q5_K_M':
    case 'Q5_K_S':
      return 'Very good quality';
    case 'Q4_K_M':
    case 'Q4_K_S':
      return 'Balanced';
    case 'Q3_K_L':
    case 'Q3_K_M':
    case 'Q3_K_S':
      return 'Smaller and faster';
    case 'Q2_K':
      return 'Lowest size';
    default:
      return 'GGUF file';
  }
}

function isCatalogRecommendation(recommendation: ModelRecommendation): boolean {
  return !recommendation.repo_id || !recommendation.filename;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}
